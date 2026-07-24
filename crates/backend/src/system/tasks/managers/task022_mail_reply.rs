//! task022_mail_reply — регламент отправки ответов пользователям.
//!
//! Сканирует журнал a039 на входящие, для которых агент уже подготовил результат
//! (`prepared`) либо зафиксирован отказ/ошибка (`rejected_forbidden` | `failed`),
//! собирает письмо (текст ответа + при необходимости ссылку на чат/артефакт),
//! отправляет его отправителю (тред по Message-ID) и переводит запись в `replied`,
//! добавляя исходящую запись журнала. Проверяет срок (`due_at`) и логирует просрочку.

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use contracts::domain::a039_mail_message::aggregate::{status, MailMessage};
use contracts::system::tasks::aggregate::ScheduledTask;
use contracts::system::tasks::metadata::{TaskConfigField, TaskConfigFieldType, TaskMetadata};
use contracts::system::tasks::progress::TaskProgress;
use serde::Deserialize;
use std::sync::Arc;

use crate::domain::a039_mail_message;
use crate::shared::mail;
use crate::system::tasks::logger::TaskLogger;
use crate::system::tasks::manager::{TaskManager, TaskRunOutcome};

static METADATA: TaskMetadata = TaskMetadata {
    task_type: "task022_mail_reply",
    write_tables: &["a039_mail_message"],
    display_name: "Почта — отправка ответов",
    description: "Проверяет подготовленные ответы и отказы в журнале писем, проверяет сроки и \
        отправляет письма-ответы пользователям (в треде исходного письма).",
    external_apis: &[],
    constraints: &[
        "Отправляет только зарегистрированным пользователям (получатель = отправитель запроса)",
        "Соблюдает rate-limit отправки из [mail]",
        "Одно письмо-ответ на один входящий запрос",
    ],
    config_fields: &[TaskConfigField {
        key: "max_replies",
        label: "Максимум ответов за запуск",
        hint: "Сколько ответов отправить за один тик",
        field_type: TaskConfigFieldType::Integer,
        required: false,
        default_value: Some("10"),
        min_value: Some(1),
        max_value: Some(50),
    }],
    max_duration_seconds: 600,
};

#[derive(Debug, Deserialize)]
struct MailReplyConfig {
    #[serde(default = "default_max_replies")]
    max_replies: usize,
}

fn default_max_replies() -> usize {
    10
}

pub struct Task022MailReplyManager;

impl Task022MailReplyManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TaskManager for Task022MailReplyManager {
    fn task_type(&self) -> &'static str {
        "task022_mail_reply"
    }

    fn metadata(&self) -> &'static TaskMetadata {
        &METADATA
    }

    async fn run(
        &self,
        task: &ScheduledTask,
        session_id: &str,
        logger: Arc<TaskLogger>,
    ) -> Result<TaskRunOutcome> {
        let config: MailReplyConfig =
            serde_json::from_str(&task.config_json).unwrap_or(MailReplyConfig {
                max_replies: default_max_replies(),
            });
        let max_replies = config.max_replies.clamp(1, 50);

        if !crate::shared::config::get_mail_config().enabled {
            logger.write_log(
                session_id,
                "Почта отключена ([mail].enabled=false) — пропуск.",
            )?;
            return Ok(TaskRunOutcome::completed());
        }

        let pending = a039_mail_message::service::list_pending_reply().await?;
        logger.write_log(
            session_id,
            &format!("Mail reply: ожидают ответа {} писем", pending.len()),
        )?;

        let mut sent = 0usize;
        for mut rec in pending.into_iter().take(max_replies) {
            // Просрочка — для наблюдаемости (всё равно отвечаем).
            if is_overdue(&rec) {
                logger.write_log(
                    session_id,
                    &format!(
                        "Запись {} просрочена (due_at={:?})",
                        rec.base.code, rec.due_at
                    ),
                )?;
            }

            // Rate-limit отправки — если исчерпан, оставляем на следующий тик.
            if let Err(e) = mail::check_and_record_send() {
                logger.write_log(
                    session_id,
                    &format!("Rate-limit отправки: {e} — стоп до след. тика"),
                )?;
                break;
            }

            match self.reply_one(&mut rec, session_id, &logger).await {
                Ok(true) => sent += 1,
                Ok(false) => {}
                Err(e) => {
                    logger.write_log(
                        session_id,
                        &format!("{}: ошибка отправки: {e}", rec.base.code),
                    )?;
                }
            }
        }

        logger.write_log(
            session_id,
            &format!("Mail reply завершён: отправлено {sent}"),
        )?;
        Ok(TaskRunOutcome::completed())
    }

    fn get_progress(&self, _session_id: &str) -> Option<TaskProgress> {
        None
    }
}

impl Task022MailReplyManager {
    /// Отправить один ответ. Возвращает true, если письмо ушло.
    async fn reply_one(
        &self,
        rec: &mut MailMessage,
        session_id: &str,
        logger: &Arc<TaskLogger>,
    ) -> Result<bool> {
        let to = if rec.from_addr.trim().is_empty() {
            // На случай отсутствия from — используем связанный адрес пользователя нельзя,
            // тогда отменяем отправку.
            logger.write_log(
                session_id,
                &format!("{}: нет адреса получателя — пропуск", rec.base.code),
            )?;
            return Ok(false);
        } else {
            mail::extract_addr(&rec.from_addr)
        };

        let subject = reply_subject(&rec.subject);
        let body = self.build_body(rec).await?;

        mail::send_email_reply(&to, &subject, &body, rec.message_id_hdr.as_deref()).await?;

        // Исходящая запись журнала + перевод входящего в replied.
        a039_mail_message::service::record_outbound(
            &to,
            &subject,
            &body,
            rec.user_ref.clone(),
            &rec.to_string_id(),
        )
        .await?;

        rec.status = status::REPLIED.to_string();
        a039_mail_message::service::save(rec).await?;

        logger.write_log(
            session_id,
            &format!("{}: ответ отправлен на {to}", rec.base.code),
        )?;
        Ok(true)
    }

    /// Собрать тело ответа по статусу записи.
    async fn build_body(&self, rec: &MailMessage) -> Result<String> {
        if rec.status == status::REJECTED_FORBIDDEN {
            return Ok(format!(
                "Здравствуйте!\n\nК сожалению, у вас недостаточно прав для выполнения этого запроса.\n{}\n\nОбратитесь к администратору, если доступ необходим.",
                rec.error.clone().unwrap_or_default()
            ));
        }
        if rec.status == status::FAILED {
            return Ok(
                "Здравствуйте!\n\nНе удалось обработать ваш запрос автоматически. \
                 Мы уже знаем о проблеме; попробуйте переформулировать запрос или обратитесь в поддержку."
                    .to_string(),
            );
        }

        // prepared — берём текст ответа агента из чата.
        let mut answer = self.fetch_answer(rec).await.unwrap_or_default();
        if answer.trim().is_empty() {
            answer = "Ответ подготовлен, подробности — по ссылке ниже.".to_string();
        }

        let mut body = format!("Здравствуйте!\n\n{answer}");

        // Ссылка на чат/артефакт, если задан base_url.
        let base = crate::shared::config::get_mail_config()
            .base_url
            .trim()
            .to_string();
        if !base.is_empty() {
            if let Some(chat) = &rec.chat_ref {
                let link = format!(
                    "{}/#/a018_llm_chat_details/{}",
                    base.trim_end_matches('/'),
                    chat
                );
                if rec.artifact_ref.is_some() {
                    body.push_str(&format!("\n\nПолный результат и артефакт: {link}"));
                } else {
                    body.push_str(&format!("\n\nОткрыть в приложении: {link}"));
                }
            }
        }

        Ok(body)
    }

    /// Достать текст ответа ассистента из связанного чата.
    async fn fetch_answer(&self, rec: &MailMessage) -> Result<String> {
        let chat_id = match &rec.chat_ref {
            Some(c) => c,
            None => return Ok(String::new()),
        };
        let messages = crate::domain::a018_llm_chat::service::get_messages(chat_id).await?;
        // Предпочитаем конкретное сообщение по message_ref, иначе последнее сообщение ассистента.
        if let Some(mid) = &rec.message_ref {
            if let Some(m) = messages.iter().find(|m| m.id.to_string() == *mid) {
                return Ok(m.content.clone());
            }
        }
        Ok(messages
            .iter()
            .rev()
            .find(|m| {
                matches!(
                    m.role,
                    contracts::domain::a018_llm_chat::aggregate::ChatRole::Assistant
                )
            })
            .map(|m| m.content.clone())
            .unwrap_or_default())
    }
}

fn reply_subject(subject: &str) -> String {
    let s = subject.trim();
    if s.is_empty() {
        "Re: ваш запрос".to_string()
    } else if s.to_lowercase().starts_with("re:") {
        s.to_string()
    } else {
        format!("Re: {s}")
    }
}

fn is_overdue(rec: &MailMessage) -> bool {
    rec.due_at
        .as_deref()
        .and_then(|d| chrono::DateTime::parse_from_rfc3339(d).ok())
        .map(|due| Utc::now() > due.with_timezone(&Utc))
        .unwrap_or(false)
}
