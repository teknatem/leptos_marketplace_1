use anyhow::Result;
use async_trait::async_trait;
use contracts::domain::common::AggregateId;
use contracts::system::tasks::aggregate::ScheduledTask;
use contracts::system::tasks::metadata::TaskMetadata;
use contracts::system::tasks::progress::TaskProgress;
use std::sync::Arc;

use crate::domain::a018_llm_chat;
use crate::system::tasks::logger::TaskLogger;
use crate::system::tasks::manager::{TaskManager, TaskRunOutcome};

static METADATA: TaskMetadata = TaskMetadata {
    task_type: "task015_kb_post",
    write_tables: &["a031_kb_edit", "a018_llm_chat", "a018_llm_chat_message", "sys_tool_trace"],
    display_name: "KB — публикация правок",
    description: "Администратор базы знаний обрабатывает утверждённые a031_kb_edit: \
        читает целевые бизнес-статьи, готовит финальные редакции и записывает markdown в Obsidian-каталог KB.",
    external_apis: &[],
    constraints: &[
        "Обрабатывает только a031_kb_edit со статусом Approved",
        "Требует настроенного LLM-агента типа KbAdmin",
        "Публикует только бизнес-знания организации; техническая документация приложения должна быть embedded",
        "После записи документов перезагружает in-memory индекс KB",
    ],
    config_fields: &[],
    max_duration_seconds: 3600,
};

pub struct Task015KbPostManager;

impl Task015KbPostManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TaskManager for Task015KbPostManager {
    fn task_type(&self) -> &'static str {
        "task015_kb_post"
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
        let agent = crate::domain::a031_kb_edit::service::ensure_kb_admin_agent().await?;

        let edits = crate::domain::a031_kb_edit::service::list_by_status(
            contracts::domain::a031_kb_edit::aggregate::KbEditStatus::Approved,
        )
        .await?;

        logger.write_log(
            session_id,
            &format!("KB post started: approved edits={}", edits.len()),
        )?;

        let mut processed = 0usize;
        let mut article_count = 0usize;

        for edit in edits {
            let edit_id = edit.base.id.as_string();
            crate::domain::a031_kb_edit::service::mark_processing(
                &edit_id,
                Some(session_id.to_string()),
            )
            .await?;

            let chat_id = a018_llm_chat::service::create(
                a018_llm_chat::service::LlmChatDto {
                    id: None,
                    code: Some(format!("KB-POST-{}-{}", session_id, &edit_id[..8])),
                    description: format!("KB публикация: {}", edit.title),
                    comment: Some(format!("Служебный чат task015_kb_post для {}", edit_id)),
                    agent_id: agent.base.id.as_string(),
                    model_name: Some(agent.model_name.clone()),
                },
                None,
            )
            .await?;

            let trigger = format!(
                "Режим: публикация бизнес-базы знаний организации.\n\
                 Обработай утверждённый тикет a031_kb_edit.\n\
                 id: {}\n\
                 title: {}\n\
                 agent_summary: {}\n\
                 target_articles: {}\n\
                 \n\
                 Публикуй только Obsidian-статьи о функционировании бизнеса организации: процессы, роли, \
                 регламенты, термины, исключения, правила работы с 1С и маркетплейсами. \
                 Не записывай в Obsidian технические детали приложения: SQL-схемы, DataView/API инструкции, \
                 агрегаты, внутреннюю реализацию или tool usage. Такие сведения должны оставаться embedded.\n\
                 Для каждой бизнес-статьи прочитай текущий контент через get_kb_document, \
                 подготовь полный обновлённый markdown и запиши через write_kb_document. \
                 Если target_articles содержит техническую статью, не записывай её и укажи это в отчёте. \
                 Сохрани смысл существующей статьи и добавь ссылки/контекст по тикету, если это уместно.",
                edit_id,
                edit.title,
                edit.agent_summary,
                serde_json::to_string(&edit.target_articles).unwrap_or_else(|_| "[]".to_string())
            );

            let response = a018_llm_chat::service::send_message(
                &chat_id.to_string(),
                a018_llm_chat::service::SendMessageRequest {
                    content: trigger,
                    model_name: Some(agent.model_name.clone()),
                    attachment_ids: Vec::new(),
                    request_id: None,
                },
                None,
            )
            .await?;

            logger.write_log(
                session_id,
                &format!(
                    "KB post edit {} completed. assistant_msg={}, tool_trace={}",
                    edit_id,
                    response.id,
                    response
                        .tool_trace
                        .clone()
                        .unwrap_or_else(|| "[]".to_string())
                ),
            )?;

            crate::domain::a031_kb_edit::service::mark_closed(
                &edit_id,
                edit.target_articles.clone(),
                Some(session_id.to_string()),
            )
            .await?;

            processed += 1;
            article_count += edit.target_articles.len();
        }

        crate::shared::llm::knowledge_base::reload_knowledge_base()?;
        logger.write_log(
            session_id,
            &format!(
                "KB post completed: processed_edits={}, target_articles={}",
                processed, article_count
            ),
        )?;

        Ok(TaskRunOutcome::completed())
    }

    fn get_progress(&self, _session_id: &str) -> Option<TaskProgress> {
        None
    }
}
