use anyhow::Result;
use async_trait::async_trait;
use contracts::domain::common::AggregateId;
use contracts::system::tasks::aggregate::ScheduledTask;
use contracts::system::tasks::metadata::{TaskConfigField, TaskConfigFieldType, TaskMetadata};
use contracts::system::tasks::progress::TaskProgress;
use serde::Deserialize;
use std::sync::Arc;

use crate::domain::a018_llm_chat;
use crate::system::tasks::logger::TaskLogger;
use crate::system::tasks::manager::{TaskManager, TaskRunOutcome};

static METADATA: TaskMetadata = TaskMetadata {
    task_type: "task014_kb_analyze",
    display_name: "KB — аудит качества ответов",
    description: "Регистратор знаний анализирует историю чатов и tool_trace, находит плохие \
        ответы или повторяющиеся ошибки и создаёт тикеты a031_kb_edit для улучшения бизнес-KB.",
    external_apis: &[],
    constraints: &[
        "Требует настроенного LLM-агента типа KbAdmin",
        "Не занимается общим наполнением KB — для этого используется task016_kb_intake",
        "Создаёт только концепцию правок бизнес-знаний; технические детали приложения не публикуются в Obsidian",
    ],
    config_fields: &[TaskConfigField {
        key: "lookback_days",
        label: "Период аудита (дней)",
        hint: "Сколько дней истории чатов проверять на плохие ответы",
        field_type: TaskConfigFieldType::Integer,
        required: false,
        default_value: Some("7"),
        min_value: Some(1),
        max_value: Some(90),
    }],
    max_duration_seconds: 1800,
};

#[derive(Debug, Deserialize)]
struct KbAnalyzeConfig {
    #[serde(default = "default_lookback_days")]
    lookback_days: i64,
}

fn default_lookback_days() -> i64 {
    7
}

pub struct Task014KbAnalyzeManager;

impl Task014KbAnalyzeManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TaskManager for Task014KbAnalyzeManager {
    fn task_type(&self) -> &'static str {
        "task014_kb_analyze"
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
        let config: KbAnalyzeConfig =
            serde_json::from_str(&task.config_json).unwrap_or(KbAnalyzeConfig {
                lookback_days: default_lookback_days(),
            });

        let agent = crate::domain::a031_kb_edit::service::ensure_kb_admin_agent().await?;

        logger.write_log(
            session_id,
            &format!(
                "KB quality review started: agent={}, lookback_days={}",
                agent.base.description, config.lookback_days
            ),
        )?;

        let chat_id = a018_llm_chat::service::create(a018_llm_chat::service::LlmChatDto {
            id: None,
            code: Some(format!("KB-ANALYZE-{}", session_id)),
            description: format!("KB анализ {}", session_id),
            comment: Some("Служебный чат task014_kb_analyze".to_string()),
            agent_id: agent.base.id.as_string(),
            model_name: Some(agent.model_name.clone()),
        })
        .await?;

        let trigger = format!(
            "Режим: quality_review.\n\
             Проверь историю чатов за последние {} дней на плохие ответы и точки улучшения качества.\n\
             База данных SQLite. Для истории LLM используй таблицы a018_llm_chat и a018_llm_chat_message.\n\
             Пример фильтра периода: m.created_at >= datetime('now', '-{} days').\n\
             Не используй PostgreSQL синтаксис NOW(), INTERVAL, ::interval.\n\
             Используй execute_query для анализа сообщений/tool_trace. KB-документы используй только как контекст.\n\
             Obsidian-статьи предназначены только для бизнес-знаний организации. Embedded-документы приложения \
             можно читать как технический контекст, но нельзя предлагать переносить в Obsidian.\n\
             Ищи: неверные ответы, неподтверждённые выводы, ошибки SQL, повторные неудачные tool calls, \
             ответы без нужного бизнес-контекста, противоречия между ответом и KB.\n\
             Для каждой конкретной проблемы качества создай create_kb_edit с edit_type=gap, proposal или contradiction.\n\
             В agent_summary укажи: какой ответ/паттерн плохой, почему это опасно, какое бизнес-знание организации надо добавить или уточнить. \
             target_articles указывай только для Obsidian-статей о бизнесе; для технических проблем оставляй список пустым.\n\
             analyze_task_run_id = '{}'.\n\
             Если проблем качества нет, создай один тикет edit_type=all_good с кратким отчётом.\n\
             После успешного create_kb_edit больше не вызывай инструменты: сразу дай финальный краткий отчёт.",
            config.lookback_days, config.lookback_days, session_id
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
                "KB analyze completed. assistant_msg={}, tool_trace={}",
                response.id,
                response.tool_trace.unwrap_or_else(|| "[]".to_string())
            ),
        )?;

        Ok(TaskRunOutcome::completed())
    }

    fn get_progress(&self, _session_id: &str) -> Option<TaskProgress> {
        None
    }
}
