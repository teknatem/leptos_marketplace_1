use anyhow::{Context, Result};
use async_trait::async_trait;
use contracts::system::tasks::aggregate::ScheduledTask;
use contracts::system::tasks::metadata::{ExternalApiInfo, TaskMetadata};
use contracts::system::tasks::progress::TaskProgress;
use std::sync::Arc;

use crate::system::tasks::logger::TaskLogger;
use crate::system::tasks::manager::{TaskManager, TaskRunOutcome};
use crate::system::tickets::{bitrix, change_token, repository};

static METADATA: TaskMetadata = TaskMetadata {
    task_type: "task025_bitrix_ticket_sync",
    write_tables: &["sys_ticket", "sys_ticket_comment"],
    display_name: "Тикеты — синхронизация с Битрикс24",
    description: "Каждые 5 минут отправляет локальные тикеты и комментарии в Битрикс24, \
         получает статусы задач и новые текстовые сообщения.",
    external_apis: &[ExternalApiInfo {
        name: "Bitrix24 REST API",
        base_url: "https://sanstar.bitrix24.ru/rest/",
        rate_limit_desc:
            "Один последовательный проход; лимиты и ошибки записываются в журнал задачи",
    }],
    constraints: &[
        "Тикеты создаются только локально",
        "Из Битрикс24 принимаются только статусы и текстовые комментарии",
        "Вложения чата не синхронизируются",
        "Время хранится в UTC и отображается в UI как UTC+3",
    ],
    config_fields: &[],
    max_duration_seconds: 240,
};

pub struct Task025BitrixTicketSyncManager;

impl Task025BitrixTicketSyncManager {
    pub fn new() -> Self {
        Self
    }

    async fn sync_ticket(
        &self,
        client: &bitrix::BitrixClient,
        config: &crate::shared::config::Bitrix24Config,
        ticket_id: &str,
        session_id: &str,
        logger: &TaskLogger,
    ) -> Result<()> {
        let mut ticket = repository::get_by_id(ticket_id)
            .await?
            .context("ticket disappeared during synchronization")?;
        let mut created_or_recovered = false;

        let task_id = if let Some(id) = ticket.bitrix_task_id.clone() {
            id
        } else {
            let id = match client.find_task_by_code(&ticket.code).await? {
                Some(id) => {
                    logger.write_log(
                        session_id,
                        &format!(
                            "{}: найдена ранее созданная задача Битрикс24 #{id}",
                            ticket.code
                        ),
                    )?;
                    id
                }
                None => client.create_task(&ticket).await?,
            };
            repository::mark_bitrix_sent(&ticket.id, &id, &bitrix::utc_now()).await?;
            created_or_recovered = true;
            id
        };

        let remote = client.get_task(&task_id).await?;
        let remote_status = bitrix::status_from_bitrix(remote.status, ticket.status);
        let status_changed = remote_status != ticket.status;
        repository::apply_bitrix_status(&ticket.id, remote_status.as_str(), &bitrix::utc_now())
            .await?;
        if status_changed {
            change_token::TOKEN.bump();
        }

        ticket = repository::get_by_id(&ticket.id)
            .await?
            .context("ticket disappeared after receiving Bitrix24 status")?;
        if !created_or_recovered
            && bitrix::needs_push(&ticket.updated_at, ticket.bitrix_synced_at.as_deref())
        {
            client.update_task(&task_id, &ticket).await?;
            repository::mark_bitrix_sent(&ticket.id, &task_id, &bitrix::utc_now()).await?;
        }

        if config.sync_comments {
            if let Err(error) = self
                .sync_comments(client, &ticket.id, &task_id, remote.chat_id)
                .await
            {
                logger.write_log(
                    session_id,
                    &format!(
                        "{}: комментарии пропущены, основная синхронизация продолжена: {error:#}",
                        ticket.code
                    ),
                )?;
            }
        }
        Ok(())
    }

    async fn sync_comments(
        &self,
        client: &bitrix::BitrixClient,
        ticket_id: &str,
        task_id: &str,
        chat_id: Option<i64>,
    ) -> Result<()> {
        let mut inserted = false;
        if let Some(chat_id) = chat_id {
            let after_id = repository::last_bitrix_message_id(ticket_id).await?;
            let messages = client.get_messages(chat_id, after_id).await?;
            for message in messages {
                inserted |= repository::insert_bitrix_comment(
                    ticket_id,
                    &message.id.to_string(),
                    &message.author_id.to_string(),
                    message.author_name.as_deref(),
                    &message.text,
                    &message.date,
                )
                .await?;
            }
        }
        if inserted {
            change_token::TOKEN.bump();
        }

        for comment in repository::list_comments_pending_bitrix(ticket_id).await? {
            let message_id = client.send_comment(task_id, &comment.body).await?;
            repository::mark_comment_bitrix_sent(&comment.id, &message_id).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl TaskManager for Task025BitrixTicketSyncManager {
    fn task_type(&self) -> &'static str {
        "task025_bitrix_ticket_sync"
    }

    fn metadata(&self) -> &'static TaskMetadata {
        &METADATA
    }

    async fn run(
        &self,
        _task: &ScheduledTask,
        session_id: &str,
        logger: Arc<TaskLogger>,
    ) -> Result<TaskRunOutcome> {
        let config = crate::shared::config::get_bitrix24_config().clone();
        if !config.enabled {
            logger.write_log(
                session_id,
                "Битрикс24 отключен ([bitrix24].enabled=false) — обмен пропущен.",
            )?;
            return Ok(TaskRunOutcome::completed());
        }
        let client = match bitrix::BitrixClient::new(config.clone()) {
            Ok(client) => client,
            Err(error) => {
                logger.write_log(
                    session_id,
                    &format!("Битрикс24 не настроен — обмен пропущен: {error}"),
                )?;
                return Ok(TaskRunOutcome::completed());
            }
        };

        let tickets = repository::list_for_bitrix_sync().await?;
        logger.write_log(
            session_id,
            &format!("Битрикс24: к проверке {} тикетов", tickets.len()),
        )?;
        let mut succeeded = 0usize;
        let mut failed = 0usize;
        for ticket in tickets {
            match self
                .sync_ticket(&client, &config, &ticket.id, session_id, logger.as_ref())
                .await
            {
                Ok(()) => succeeded += 1,
                Err(error) => {
                    failed += 1;
                    logger.write_log(
                        session_id,
                        &format!("{}: ошибка синхронизации: {error:#}", ticket.code),
                    )?;
                }
            }
        }
        logger.write_log(
            session_id,
            &format!("Битрикс24: успешно {succeeded}, с ошибками {failed}"),
        )?;
        if failed == 0 {
            Ok(TaskRunOutcome::completed())
        } else {
            Ok(TaskRunOutcome::completed_with_errors())
        }
    }

    fn get_progress(&self, _session_id: &str) -> Option<TaskProgress> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_metadata_is_registered_for_expected_task_type() {
        assert_eq!(METADATA.task_type, "task025_bitrix_ticket_sync");
        assert_eq!(METADATA.max_duration_seconds, 240);
    }
}
