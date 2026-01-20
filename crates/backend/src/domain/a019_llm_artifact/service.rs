use super::repository;
use crate::domain::a017_llm_agent::repository as agent_repository;
use crate::domain::a018_llm_chat::repository as chat_repository;
use contracts::domain::a017_llm_agent::aggregate::LlmAgentId;
use contracts::domain::a018_llm_chat::aggregate::LlmChatId;
use contracts::domain::a019_llm_artifact::aggregate::{LlmArtifact, LlmArtifactId};
use contracts::domain::common::AggregateId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmArtifactDto {
    pub id: Option<String>,
    pub code: Option<String>,
    pub description: String,
    pub comment: Option<String>,
    pub chat_id: String,
    pub agent_id: String,
    pub sql_query: String,
    pub query_params: Option<String>,
    pub visualization_config: Option<String>,
}

/// Создание нового артефакта
pub async fn create(dto: LlmArtifactDto) -> anyhow::Result<Uuid> {
    let code = dto
        .code
        .clone()
        .unwrap_or_else(|| format!("ARTIFACT-{}", Uuid::new_v4()));

    let chat_uuid = Uuid::parse_str(&dto.chat_id)
        .map_err(|e| anyhow::anyhow!("Invalid chat_id: {}", e))?;
    let chat_id = LlmChatId::new(chat_uuid);

    let agent_uuid = Uuid::parse_str(&dto.agent_id)
        .map_err(|e| anyhow::anyhow!("Invalid agent_id: {}", e))?;
    let agent_id = LlmAgentId::new(agent_uuid);

    let db = crate::shared::data::db::get_connection();

    // Проверяем что чат существует
    let _chat = chat_repository::find_by_id(&db, &chat_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Chat not found: {}", dto.chat_id))?;

    // Проверяем что агент существует
    let _agent = agent_repository::find_by_id(&agent_id.as_string())
        .await?
        .ok_or_else(|| anyhow::anyhow!("Agent not found: {}", dto.agent_id))?;

    let mut aggregate = LlmArtifact::new_for_insert(
        code,
        dto.description,
        chat_id,
        agent_id,
        dto.sql_query,
    );

    // Установить дополнительные поля
    aggregate.base.comment = dto.comment;
    aggregate.query_params = dto.query_params;
    aggregate.visualization_config = dto.visualization_config;

    // Валидация
    aggregate
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    // Before write
    aggregate.before_write();

    let id = aggregate.base.id.0;

    // Сохранение через repository
    repository::insert(&db, &aggregate).await?;

    Ok(id)
}

/// Обновление артефакта
pub async fn update(dto: LlmArtifactDto) -> anyhow::Result<()> {
    let id_str = dto
        .id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("ID is required"))?;

    let artifact_uuid = Uuid::parse_str(id_str)
        .map_err(|e| anyhow::anyhow!("Invalid artifact ID: {}", e))?;
    let artifact_id = LlmArtifactId::new(artifact_uuid);

    let db = crate::shared::data::db::get_connection();
    let mut aggregate = repository::find_by_id(&db, &artifact_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact not found"))?;

    // Обновление полей
    if let Some(code) = dto.code {
        aggregate.base.code = code;
    }
    aggregate.base.description = dto.description;
    aggregate.base.comment = dto.comment;
    aggregate.sql_query = dto.sql_query;
    aggregate.query_params = dto.query_params;
    aggregate.visualization_config = dto.visualization_config;

    // Обновление связей (если указаны)
    if let Ok(new_chat_uuid) = Uuid::parse_str(&dto.chat_id) {
        aggregate.chat_id = LlmChatId::new(new_chat_uuid);
    }

    if let Ok(new_agent_uuid) = Uuid::parse_str(&dto.agent_id) {
        aggregate.agent_id = LlmAgentId::new(new_agent_uuid);
    }

    // Валидация
    aggregate
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    // Before write
    aggregate.before_write();

    // Сохранение
    repository::update(&db, &aggregate).await?;

    Ok(())
}

/// Удаление артефакта (soft delete)
pub async fn delete(id: &str) -> anyhow::Result<()> {
    let artifact_uuid = Uuid::parse_str(id)
        .map_err(|e| anyhow::anyhow!("Invalid artifact ID: {}", e))?;
    let artifact_id = LlmArtifactId::new(artifact_uuid);

    let db = crate::shared::data::db::get_connection();
    repository::soft_delete(&db, &artifact_id).await?;

    Ok(())
}

/// Получить артефакт по ID
pub async fn get_by_id(id: &str) -> anyhow::Result<Option<LlmArtifact>> {
    let artifact_uuid = Uuid::parse_str(id)
        .map_err(|e| anyhow::anyhow!("Invalid artifact ID: {}", e))?;
    let artifact_id = LlmArtifactId::new(artifact_uuid);

    let db = crate::shared::data::db::get_connection();
    let artifact = repository::find_by_id(&db, &artifact_id).await?;

    Ok(artifact)
}

/// Получить список всех артефактов
pub async fn list_all() -> anyhow::Result<Vec<LlmArtifact>> {
    let db = crate::shared::data::db::get_connection();
    let artifacts = repository::list_all(&db).await?;
    Ok(artifacts)
}

/// Получить артефакты конкретного чата
pub async fn list_by_chat_id(chat_id: &str) -> anyhow::Result<Vec<LlmArtifact>> {
    let chat_uuid = Uuid::parse_str(chat_id)
        .map_err(|e| anyhow::anyhow!("Invalid chat ID: {}", e))?;
    let chat_id_obj = LlmChatId::new(chat_uuid);

    let db = crate::shared::data::db::get_connection();
    let artifacts = repository::list_by_chat_id(&db, &chat_id_obj).await?;

    Ok(artifacts)
}

/// Получить список артефактов с пагинацией
pub async fn list_paginated(page: u64, page_size: u64) -> anyhow::Result<(Vec<LlmArtifact>, u64)> {
    let db = crate::shared::data::db::get_connection();
    let (artifacts, total) = repository::list_paginated(&db, page, page_size).await?;
    Ok((artifacts, total))
}
