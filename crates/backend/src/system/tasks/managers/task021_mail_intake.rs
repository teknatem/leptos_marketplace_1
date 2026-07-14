//! task021_mail_intake — поллинг INBOX, диспетчеризация письма к нужному LLM-агенту
//! и подготовка ответа.
//!
//! Конвейер (см. также [`super::task022_mail_reply`]): читаем новые письма, привязываем
//! каждое к активному пользователю `sys_users`, классифицируем интент → тип агента,
//! проверяем права отправителя, прогоняем агента (чат a018) и записываем результат в
//! журнал a039 со статусом `prepared`. Само письмо-ответ шлёт регламент task022.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use contracts::domain::a038_llm_connection::aggregate::AgentType;
use contracts::domain::a039_mail_message::aggregate::status;
use contracts::domain::common::AggregateId;
use contracts::system::tasks::aggregate::ScheduledTask;
use contracts::system::tasks::metadata::{TaskConfigField, TaskConfigFieldType, TaskMetadata};
use contracts::system::tasks::progress::TaskProgress;
use serde::Deserialize;
use std::sync::Arc;

use crate::domain::{a018_llm_chat, a038_llm_connection, a039_mail_message};
use crate::shared::mail;
use crate::system::tasks::logger::TaskLogger;
use crate::system::tasks::manager::{TaskManager, TaskRunOutcome};

static METADATA: TaskMetadata = TaskMetadata {
    task_type: "task021_mail_intake",
    display_name: "Почта — приём и обработка запросов",
    description: "Читает входящие письма от зарегистрированных пользователей, определяет нужного \
        специалиста по содержимому, проверяет права отправителя, прогоняет LLM-агента и \
        готовит ответ. Ответ отправляет регламент task022_mail_reply.",
    external_apis: &[],
    constraints: &[
        "Обрабатывает письма только от активных пользователей (sys_users)",
        "Тип агента определяется правами отправителя (роль)",
        "Не отправляет письма — только готовит ответы (статус prepared)",
    ],
    config_fields: &[
        TaskConfigField {
            key: "max_emails",
            label: "Максимум писем за запуск",
            hint: "Сколько новых писем обработать за один тик",
            field_type: TaskConfigFieldType::Integer,
            required: false,
            default_value: Some("5"),
            min_value: Some(1),
            max_value: Some(20),
        },
        TaskConfigField {
            key: "sla_minutes",
            label: "Срок ответа (мин)",
            hint: "Через сколько минут ответ считается просроченным",
            field_type: TaskConfigFieldType::Integer,
            required: false,
            default_value: Some("60"),
            min_value: Some(5),
            max_value: Some(1440),
        },
    ],
    max_duration_seconds: 1800,
};

#[derive(Debug, Deserialize)]
struct MailIntakeConfig {
    #[serde(default = "default_max_emails")]
    max_emails: usize,
    #[serde(default = "default_sla_minutes")]
    sla_minutes: i64,
}

fn default_max_emails() -> usize {
    5
}
fn default_sla_minutes() -> i64 {
    60
}

pub struct Task021MailIntakeManager;

impl Task021MailIntakeManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TaskManager for Task021MailIntakeManager {
    fn task_type(&self) -> &'static str {
        "task021_mail_intake"
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
        let config: MailIntakeConfig =
            serde_json::from_str(&task.config_json).unwrap_or(MailIntakeConfig {
                max_emails: default_max_emails(),
                sla_minutes: default_sla_minutes(),
            });
        let max_emails = config.max_emails.clamp(1, 20);
        let sla_minutes = config.sla_minutes.clamp(5, 1440);

        if !crate::shared::config::get_mail_config().enabled {
            logger.write_log(session_id, "Почта отключена ([mail].enabled=false) — пропуск.")?;
            return Ok(TaskRunOutcome::completed());
        }

        let headers = match mail::list_inbox(max_emails).await {
            Ok(h) => h,
            Err(e) => {
                logger.write_log(session_id, &format!("Не удалось прочитать INBOX: {e}"))?;
                return Ok(TaskRunOutcome::completed());
            }
        };

        logger.write_log(
            session_id,
            &format!("Mail intake: получено {} писем из INBOX", headers.len()),
        )?;

        let mut processed = 0usize;
        for h in headers {
            let uid = h.uid as i64;

            // Дедуп: письмо уже в журнале.
            if a039_mail_message::service::find_inbound_by_uid(uid)
                .await?
                .is_some()
            {
                continue;
            }

            if let Err(e) = self
                .process_one(uid, sla_minutes, session_id, &logger)
                .await
            {
                logger.write_log(session_id, &format!("UID {uid}: ошибка обработки: {e}"))?;
            } else {
                processed += 1;
            }
        }

        logger.write_log(session_id, &format!("Mail intake завершён: обработано {processed}"))?;
        Ok(TaskRunOutcome::completed())
    }

    fn get_progress(&self, _session_id: &str) -> Option<TaskProgress> {
        None
    }
}

impl Task021MailIntakeManager {
    async fn process_one(
        &self,
        uid: i64,
        sla_minutes: i64,
        session_id: &str,
        logger: &Arc<TaskLogger>,
    ) -> Result<()> {
        let email = mail::read_email(uid as u32).await?;
        let from_addr = mail::extract_addr(&email.from);
        let user = crate::system::users::repository::find_active_by_email(&from_addr).await?;

        // Зафиксировать входящее (status=received) с привязкой к пользователю (если найден).
        let mut rec = a039_mail_message::service::record_inbound(
            uid,
            email.message_id.clone(),
            &email.from,
            &email.to,
            &email.subject,
            &email.body,
            user.as_ref().map(|u| u.id.clone()),
        )
        .await?;

        // Незарегистрированный отправитель — без ответа.
        let user = match user {
            Some(u) => u,
            None => {
                rec.status = status::REJECTED_UNKNOWN_SENDER.to_string();
                rec.error = Some(format!("Отправитель '{from_addr}' не найден среди активных пользователей"));
                a039_mail_message::service::save(&mut rec).await?;
                logger.write_log(session_id, &format!("UID {uid}: отклонено (неизвестный отправитель {from_addr})"))?;
                return Ok(());
            }
        };

        // Классификация интента → тип агента (rule-based, без LLM).
        let classify_text = format!("{}\n{}", email.subject, email.body);
        let intent =
            crate::shared::llm::router::quick_intent(&classify_text, &AgentType::BusinessAnalyst);
        let agent_type = crate::shared::llm::router::intent_to_agent_type(&intent);
        rec.intent = Some(intent.clone());
        rec.agent_type = Some(agent_type.as_str().to_string());

        // Авторизация: тип агента должен быть разрешён роли отправителя.
        let allowed = mail::policy::allowed_agent_types_for_user(&user);
        if !allowed.contains(&agent_type) {
            rec.status = status::REJECTED_FORBIDDEN.to_string();
            rec.error = Some(format!(
                "У пользователя {} нет прав на агента {}",
                user.username,
                agent_type.display_name()
            ));
            rec.due_at = Some((Utc::now() + Duration::minutes(sla_minutes)).to_rfc3339());
            a039_mail_message::service::save(&mut rec).await?;
            logger.write_log(session_id, &format!("UID {uid}: отказано (нет прав на {})", agent_type.display_name()))?;
            return Ok(());
        }

        // Выбор подключения нужного типа + прогон агента.
        let connection = a038_llm_connection::service::ensure_connection_for(agent_type.clone()).await?;
        let chat_id = a018_llm_chat::service::create(
            a018_llm_chat::service::LlmChatDto {
                id: None,
                code: Some(format!("MAIL-{uid}")),
                description: a039_subject(&email.subject),
                comment: Some(format!("Почтовый запрос от {}", user.username)),
                agent_id: connection.to_string_id(),
                model_name: Some(connection.model_name.clone()),
            },
            Some(user.id.clone()),
        )
        .await?;

        rec.chat_ref = Some(chat_id.to_string());
        rec.due_at = Some((Utc::now() + Duration::minutes(sla_minutes)).to_rfc3339());

        match a018_llm_chat::service::send_message(
            &chat_id.to_string(),
            a018_llm_chat::service::SendMessageRequest {
                content: email.body.clone(),
                model_name: Some(connection.model_name.clone()),
                attachment_ids: Vec::new(),
                request_id: email.message_id.clone(),
            },
            None,
        )
        .await
        {
            Ok(assistant) => {
                rec.status = status::PREPARED.to_string();
                rec.message_ref = Some(assistant.id.to_string());
                rec.artifact_ref = assistant.artifact_id.as_ref().map(|a| a.as_string());
                a039_mail_message::service::save(&mut rec).await?;
                logger.write_log(
                    session_id,
                    &format!("UID {uid}: подготовлен ответ ({}, чат {})", agent_type.display_name(), chat_id),
                )?;
            }
            Err(e) => {
                rec.status = status::FAILED.to_string();
                rec.error = Some(format!("Ошибка прогона агента: {e}"));
                a039_mail_message::service::save(&mut rec).await?;
                logger.write_log(session_id, &format!("UID {uid}: ошибка прогона агента: {e}"))?;
            }
        }

        Ok(())
    }
}

/// Тема для description чата (непустая).
fn a039_subject(subject: &str) -> String {
    let s = subject.trim();
    if s.is_empty() {
        "Почтовый запрос (без темы)".to_string()
    } else {
        s.chars().take(200).collect()
    }
}
