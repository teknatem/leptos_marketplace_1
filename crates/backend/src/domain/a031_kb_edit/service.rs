use super::repository;
use crate::domain::a017_llm_agent::repository as agent_repository;
use crate::domain::a018_llm_chat;
use crate::shared::data::db::get_connection;
use contracts::domain::a017_llm_agent::aggregate::{AgentType, LlmAgent, LlmAgentId};
use contracts::domain::a018_llm_chat::aggregate::{ChatRole, LlmChatId, LlmChatMessage};
use contracts::domain::a031_kb_edit::aggregate::{KbEdit, KbEditId, KbEditStatus, KbEditType};
use contracts::domain::common::AggregateId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KbEditDto {
    pub id: Option<String>,
    pub code: Option<String>,
    pub title: String,
    pub comment: Option<String>,
    pub edit_type: Option<String>,
    pub status: Option<String>,
    pub agent_summary: String,
    #[serde(default)]
    pub target_articles: Vec<String>,
    #[serde(default)]
    pub applied_articles: Vec<String>,
    #[serde(default)]
    pub source_chat_ids: Vec<String>,
    pub agent_id: Option<String>,
    pub chat_id: Option<String>,
    pub analyze_task_run_id: Option<String>,
    pub post_task_run_id: Option<String>,
}

pub async fn create(dto: KbEditDto) -> anyhow::Result<Uuid> {
    let agent_id = parse_optional_uuid(dto.agent_id.as_deref()).map(LlmAgentId::new);
    let chat_id = parse_optional_uuid(dto.chat_id.as_deref()).map(LlmChatId::new);
    let edit_type = dto
        .edit_type
        .as_deref()
        .map(KbEditType::from_str)
        .unwrap_or(KbEditType::Proposal);

    let mut item = KbEdit::new_for_insert(
        edit_type,
        dto.title,
        dto.agent_summary,
        dto.target_articles,
        dto.source_chat_ids,
        agent_id,
        chat_id,
        dto.analyze_task_run_id,
    );
    if let Some(code) = dto.code {
        item.base.code = code;
    }
    item.base.comment = dto.comment;
    if let Some(status) = dto.status.as_deref() {
        item.status = KbEditStatus::from_str(status);
    }
    item.applied_articles = dto.applied_articles;
    item.post_task_run_id = dto.post_task_run_id;

    item.validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;
    item.before_write();
    let id = item.base.id.0;
    repository::insert(get_connection(), &item).await?;
    Ok(id)
}

pub async fn create_with_chat(
    title: String,
    agent_summary: String,
    edit_type: KbEditType,
    target_articles: Vec<String>,
    source_chat_ids: Vec<String>,
    agent_id: LlmAgentId,
    analyze_task_run_id: Option<String>,
) -> anyhow::Result<Uuid> {
    let agent = agent_repository::find_by_id(&agent_id.as_string())
        .await?
        .ok_or_else(|| anyhow::anyhow!("KbAdmin agent not found: {}", agent_id.as_string()))?;

    let chat_uuid = a018_llm_chat::service::create(a018_llm_chat::service::LlmChatDto {
        id: None,
        code: None,
        description: format!("KB: {}", title),
        comment: Some("Диалог по редактированию базы знаний".to_string()),
        agent_id: agent_id.as_string(),
        model_name: Some(agent.model_name.clone()),
    }, None)
    .await?;
    let chat_id = LlmChatId::new(chat_uuid);

    let mut item = KbEdit::new_for_insert(
        edit_type,
        title.clone(),
        agent_summary.clone(),
        target_articles.clone(),
        source_chat_ids,
        Some(agent_id),
        Some(chat_id),
        analyze_task_run_id,
    );
    item.validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;
    item.before_write();
    let id = item.base.id.0;
    repository::insert(get_connection(), &item).await?;

    let intro = if matches!(item.edit_type, KbEditType::Question) {
        "Пожалуйста, ответьте на вопросы регистратора в этом диалоге. Ответы будут использованы как подтверждённая основа для пополнения базы знаний."
    } else {
        "Предложение регистратора для обсуждения."
    };

    let content = format!(
        "## {}\n\n{}\n\n{}\n\nПредлагаемые статьи:\n{}",
        title,
        intro,
        agent_summary,
        if target_articles.is_empty() {
            "- список статей пока не определён".to_string()
        } else {
            target_articles
                .iter()
                .map(|path| format!("- `{}`", path))
                .collect::<Vec<_>>()
                .join("\n")
        }
    );
    insert_chat_message(chat_id, ChatRole::Assistant, content).await?;

    Ok(id)
}

pub async fn update(dto: KbEditDto) -> anyhow::Result<()> {
    let id_str = dto
        .id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("ID is required for update"))?;
    let id = KbEditId::new(Uuid::parse_str(id_str)?);
    let mut item = repository::find_by_id(get_connection(), &id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("KB edit not found: {}", id_str))?;

    if let Some(code) = dto.code {
        item.base.code = code;
    }
    item.title = dto.title;
    item.agent_summary = dto.agent_summary;
    item.base.comment = dto.comment;
    if let Some(edit_type) = dto.edit_type.as_deref() {
        item.edit_type = KbEditType::from_str(edit_type);
    }
    if let Some(status) = dto.status.as_deref() {
        item.status = KbEditStatus::from_str(status);
    }
    item.target_articles = dto.target_articles;
    item.applied_articles = dto.applied_articles;
    item.source_chat_ids = dto.source_chat_ids;
    item.agent_id = parse_optional_uuid(dto.agent_id.as_deref()).map(LlmAgentId::new);
    item.chat_id = parse_optional_uuid(dto.chat_id.as_deref()).map(LlmChatId::new);
    item.analyze_task_run_id = dto.analyze_task_run_id;
    item.post_task_run_id = dto.post_task_run_id;

    item.validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;
    item.before_write();
    repository::update(get_connection(), &item).await?;
    Ok(())
}

pub async fn get_by_id(id: &str) -> anyhow::Result<Option<KbEdit>> {
    let uuid = Uuid::parse_str(id)?;
    repository::find_by_id(get_connection(), &KbEditId::new(uuid))
        .await
        .map_err(Into::into)
}

pub async fn list_paginated(
    page: u64,
    page_size: u64,
    sort_by: &str,
    sort_desc: bool,
    status: Option<&str>,
    q: Option<&str>,
) -> anyhow::Result<(Vec<KbEdit>, u64)> {
    repository::list_paginated(
        get_connection(),
        page,
        page_size,
        sort_by,
        sort_desc,
        status,
        q,
    )
    .await
    .map_err(Into::into)
}

pub async fn list_by_status(status: KbEditStatus) -> anyhow::Result<Vec<KbEdit>> {
    repository::list_by_status(get_connection(), status)
        .await
        .map_err(Into::into)
}

pub async fn approve(id: &str) -> anyhow::Result<()> {
    set_status(id, KbEditStatus::Approved, None).await
}

pub async fn cancel(id: &str) -> anyhow::Result<()> {
    set_status(id, KbEditStatus::Cancelled, None).await
}

pub async fn mark_in_dialog(id: &str) -> anyhow::Result<()> {
    set_status(id, KbEditStatus::InDialog, None).await
}

pub async fn mark_processing(id: &str, run_id: Option<String>) -> anyhow::Result<()> {
    set_status(id, KbEditStatus::Processing, run_id).await
}

pub async fn mark_closed(
    id: &str,
    applied_articles: Vec<String>,
    run_id: Option<String>,
) -> anyhow::Result<()> {
    let uuid = Uuid::parse_str(id)?;
    let mut item = repository::find_by_id(get_connection(), &KbEditId::new(uuid))
        .await?
        .ok_or_else(|| anyhow::anyhow!("KB edit not found: {}", id))?;
    item.status = KbEditStatus::Closed;
    item.applied_articles = applied_articles;
    item.post_task_run_id = run_id;
    item.before_write();
    repository::update(get_connection(), &item).await?;
    Ok(())
}

pub async fn update_target_articles(id: &str, articles: Vec<String>) -> anyhow::Result<()> {
    let uuid = Uuid::parse_str(id)?;
    let mut item = repository::find_by_id(get_connection(), &KbEditId::new(uuid))
        .await?
        .ok_or_else(|| anyhow::anyhow!("KB edit not found: {}", id))?;
    item.target_articles = articles;
    if matches!(item.status, KbEditStatus::Pending) {
        item.status = KbEditStatus::InDialog;
    }
    item.before_write();
    repository::update(get_connection(), &item).await?;
    Ok(())
}

pub async fn delete(id: &str) -> anyhow::Result<()> {
    let uuid = Uuid::parse_str(id)?;
    repository::soft_delete(get_connection(), &KbEditId::new(uuid)).await?;
    Ok(())
}

pub async fn find_kb_admin_agent() -> anyhow::Result<Option<LlmAgent>> {
    let agents = agent_repository::list_all().await?;
    Ok(agents
        .into_iter()
        .find(|agent| matches!(agent.agent_type, AgentType::KbAdmin)))
}

pub async fn ensure_kb_admin_agent() -> anyhow::Result<LlmAgent> {
    if let Some(agent) = find_kb_admin_agent().await? {
        return Ok(agent);
    }

    let source_agent = if let Some(agent) = agent_repository::find_primary().await? {
        agent
    } else {
        agent_repository::list_all()
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| {
                anyhow::anyhow!("No LLM agent found. Configure an agent in a017_llm_agent first.")
            })?
    };

    let mut agent = LlmAgent::new_for_insert(
        "kb-admin".to_string(),
        "Администратор базы знаний".to_string(),
        source_agent.provider_type.clone(),
        source_agent.api_endpoint.clone(),
        source_agent.api_key.clone(),
        source_agent.model_name.clone(),
        0.2,
        source_agent.max_tokens,
        None,
        false,
        source_agent.available_models.clone(),
    );
    agent.agent_type = AgentType::KbAdmin;
    agent.base.comment = Some(format!(
        "Создан автоматически для задач базы знаний на основе агента '{}'.",
        source_agent.base.description
    ));
    agent
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;
    agent.before_write();
    agent_repository::insert(&agent).await?;
    Ok(agent)
}

pub async fn insert_chat_message(
    chat_id: LlmChatId,
    role: ChatRole,
    content: String,
) -> anyhow::Result<()> {
    let message = LlmChatMessage::new(chat_id, role, content);
    a018_llm_chat::repository::insert_message(get_connection(), &message).await?;
    Ok(())
}

async fn set_status(
    id: &str,
    status: KbEditStatus,
    post_run_id: Option<String>,
) -> anyhow::Result<()> {
    let uuid = Uuid::parse_str(id)?;
    let mut item = repository::find_by_id(get_connection(), &KbEditId::new(uuid))
        .await?
        .ok_or_else(|| anyhow::anyhow!("KB edit not found: {}", id))?;
    item.status = status;
    if let Some(run_id) = post_run_id {
        item.post_task_run_id = Some(run_id);
    }
    item.before_write();
    repository::update(get_connection(), &item).await?;
    Ok(())
}

fn parse_optional_uuid(value: Option<&str>) -> Option<Uuid> {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| Uuid::parse_str(s).ok())
}
