use super::repository;
use contracts::domain::a017_llm_agent::aggregate::{LlmAgent, LlmAgentId, LlmProviderType};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmAgentDto {
    pub id: Option<String>,
    pub code: Option<String>,
    pub description: String,
    pub comment: Option<String>,
    pub provider_type: String,
    pub api_endpoint: String,
    pub api_key: String,
    pub model_name: String,
    pub temperature: f64,
    pub max_tokens: i32,
    pub system_prompt: Option<String>,
    pub is_primary: bool,
}

/// Создание нового агента LLM
pub async fn create(dto: LlmAgentDto) -> anyhow::Result<Uuid> {
    let code = dto.code.clone().unwrap_or_else(|| format!("LLM-{}", Uuid::new_v4()));
    
    let provider_type = LlmProviderType::from_str(&dto.provider_type)
        .map_err(|e| anyhow::anyhow!("Invalid provider type: {}", e))?;

    let mut aggregate = LlmAgent::new_for_insert(
        code,
        dto.description,
        provider_type,
        dto.api_endpoint,
        dto.api_key,
        dto.model_name,
        dto.temperature,
        dto.max_tokens,
        dto.system_prompt,
        dto.is_primary,
    );

    // Валидация
    aggregate
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    // Before write
    aggregate.before_write();

    // Бизнес-логика: обеспечение единственности primary
    if aggregate.is_primary {
        repository::clear_all_primary().await?;
    }

    let id = aggregate.base.id.0;

    // Сохранение через repository
    repository::insert(&aggregate).await?;
    
    Ok(id)
}

/// Обновление существующего агента
pub async fn update(dto: LlmAgentDto) -> anyhow::Result<()> {
    let id_str = dto
        .id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("ID is required"))?;

    let mut aggregate: LlmAgent = repository::find_by_id(id_str)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Agent not found"))?;

    // Обновление полей
    if let Some(code) = dto.code {
        aggregate.base.code = code;
    }
    aggregate.base.description = dto.description;
    aggregate.base.comment = dto.comment;
    
    aggregate.provider_type = LlmProviderType::from_str(&dto.provider_type)
        .map_err(|e| anyhow::anyhow!("Invalid provider type: {}", e))?;
    aggregate.api_endpoint = dto.api_endpoint;
    aggregate.api_key = dto.api_key;
    aggregate.model_name = dto.model_name;
    aggregate.temperature = dto.temperature;
    aggregate.max_tokens = dto.max_tokens;
    aggregate.system_prompt = dto.system_prompt;
    aggregate.is_primary = dto.is_primary;

    // Валидация
    aggregate
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    // Before write
    aggregate.before_write();

    // Бизнес-логика: обеспечение единственности primary
    if aggregate.is_primary {
        repository::clear_all_primary().await?;
    }

    // Сохранение
    repository::update(&aggregate).await
}

/// Мягкое удаление агента
pub async fn delete(id: &str) -> anyhow::Result<()> {
    repository::soft_delete(id).await
}

/// Получение агента по ID
pub async fn get_by_id(id: &str) -> anyhow::Result<Option<LlmAgent>> {
    repository::find_by_id(id).await
}

/// Получение списка всех агентов
pub async fn list_all() -> anyhow::Result<Vec<LlmAgent>> {
    repository::list_all().await
}

/// Получение пагинированного списка
pub async fn list_paginated(
    limit: u64,
    offset: u64,
    sort_by: &str,
    sort_desc: bool,
) -> anyhow::Result<(Vec<LlmAgent>, u64)> {
    repository::list_paginated(limit, offset, sort_by, sort_desc).await
}

/// Получение основного агента
pub async fn get_primary() -> anyhow::Result<Option<LlmAgent>> {
    repository::find_primary().await
}
