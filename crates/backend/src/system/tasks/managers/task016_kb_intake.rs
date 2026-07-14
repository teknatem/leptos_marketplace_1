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
    task_type: "task016_kb_intake",
    display_name: "KB — сбор бизнес-знаний",
    description: "Регистратор знаний выбирает небольшой пробел в понимании работы фирмы \
        и создаёт a031_kb_edit с наводящими вопросами пользователю.",
    external_apis: &[],
    constraints: &[
        "Не публикует статьи напрямую",
        "Создаёт небольшие порции вопросов, чтобы постепенно описать процессы фирмы",
        "Собирает только бизнес-знания организации; технические детали приложения не относятся к Obsidian",
        "Ответы пользователя затем публикуются через task015_kb_post после утверждения",
    ],
    config_fields: &[
        TaskConfigField {
            key: "max_tickets",
            label: "Максимум тикетов",
            hint: "Сколько небольших вопросных тикетов создать за запуск",
            field_type: TaskConfigFieldType::Integer,
            required: false,
            default_value: Some("1"),
            min_value: Some(1),
            max_value: Some(3),
        },
        TaskConfigField {
            key: "questions_per_ticket",
            label: "Вопросов в тикете",
            hint: "Сколько наводящих вопросов задавать в одной порции",
            field_type: TaskConfigFieldType::Integer,
            required: false,
            default_value: Some("5"),
            min_value: Some(3),
            max_value: Some(7),
        },
    ],
    max_duration_seconds: 1800,
};

#[derive(Debug, Deserialize)]
struct KbIntakeConfig {
    #[serde(default = "default_max_tickets")]
    max_tickets: i64,
    #[serde(default = "default_questions_per_ticket")]
    questions_per_ticket: i64,
}

fn default_max_tickets() -> i64 {
    1
}

fn default_questions_per_ticket() -> i64 {
    5
}

pub struct Task016KbIntakeManager;

impl Task016KbIntakeManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TaskManager for Task016KbIntakeManager {
    fn task_type(&self) -> &'static str {
        "task016_kb_intake"
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
        let config: KbIntakeConfig =
            serde_json::from_str(&task.config_json).unwrap_or(KbIntakeConfig {
                max_tickets: default_max_tickets(),
                questions_per_ticket: default_questions_per_ticket(),
            });
        let max_tickets = config.max_tickets.clamp(1, 3);
        let questions_per_ticket = config.questions_per_ticket.clamp(3, 7);

        let agent = crate::domain::a031_kb_edit::service::ensure_kb_admin_agent().await?;

        logger.write_log(
            session_id,
            &format!(
                "KB intake started: agent={}, max_tickets={}, questions_per_ticket={}",
                agent.base.description, max_tickets, questions_per_ticket
            ),
        )?;

        let chat_id = a018_llm_chat::service::create(a018_llm_chat::service::LlmChatDto {
            id: None,
            code: Some(format!("KB-INTAKE-{}", session_id)),
            description: format!("KB сбор бизнес-знаний {}", session_id),
            comment: Some("Служебный чат task016_kb_intake".to_string()),
            agent_id: agent.base.id.as_string(),
            model_name: Some(agent.model_name.clone()),
        }, None)
        .await?;

        let trigger = format!(
            "Режим: knowledge_intake.\n\
             Цель базы знаний: дать бизнес-аналитику полное понимание работы фирмы — процессы, роли, \
             правила, исключения, источники данных, терминологию, работу с 1С и маркетплейсами.\n\
             Obsidian-база предназначена только для бизнес-знаний организации. Не создавай вопросы \
             про техническое устройство приложения, SQL-схемы, API, DataView или внутренние инструменты.\n\
             Изучи список KB-документов через list_kb_documents и при необходимости прочитай 1-3 статьи.\n\
             Embedded-документы приложения можно читать только как технический контекст, но не предлагай \
             их как target_articles для публикации в Obsidian.\n\
             Выбери самый полезный небольшой пробел, который можно закрыть вопросами пользователю.\n\
             Создай не больше {} тикетов create_kb_edit с edit_type=question.\n\
             В каждом тикете обязательно передай параметр questions: массив из {} конкретных наводящих вопросов.\n\
             Вопросы должны быть понятными, пронумерованными по смыслу и лёгкими для ответа.\n\
             Не проси пользователя написать большой документ. Не утверждай факты о фирме без подтверждения.\n\
             В agent_summary не дублируй вопросы, а укажи: зачем эти ответы нужны бизнес-аналитику, \
             какие знания будут добавлены, и предложи target_articles для будущего обновления.\n\
             analyze_task_run_id = '{}'.\n\
             После успешного create_kb_edit больше не вызывай инструменты: сразу дай финальный краткий отчёт.",
            max_tickets, questions_per_ticket, session_id
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
                "KB intake completed. assistant_msg={}, tool_trace={}",
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
