use super::repository;
use contracts::domain::a038_llm_connection::aggregate::{
    AgentType, LlmConnection, LlmProviderType,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConnectionDto {
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
    pub available_models: Option<String>,
    /// Курируемый короткий список разрешённых моделей (JSON-массив model_id).
    pub allowed_models: Option<String>,
    /// Тип/роль (персона): business_analyst | system_admin | general | kb_admin | plugin_admin.
    /// None → не менять (по умолчанию business_analyst).
    #[serde(default)]
    pub agent_type: Option<String>,
}

/// Создание нового подключения LLM
pub async fn create(dto: LlmConnectionDto) -> anyhow::Result<Uuid> {
    let code = dto
        .code
        .clone()
        .unwrap_or_else(|| format!("LLM-{}", Uuid::new_v4()));

    let provider_type = LlmProviderType::from_str(&dto.provider_type)
        .map_err(|e| anyhow::anyhow!("Invalid provider type: {}", e))?;

    let mut aggregate = LlmConnection::new_for_insert(
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
        dto.available_models,
        dto.allowed_models,
    );

    if let Some(ref at) = dto.agent_type {
        aggregate.agent_type = AgentType::from_str(at);
    }

    aggregate
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    aggregate.before_write();

    // Бизнес-логика: обеспечение единственности primary
    if aggregate.is_primary {
        repository::clear_all_primary().await?;
    }

    let id = aggregate.base.id.0;
    repository::insert(&aggregate).await?;

    Ok(id)
}

/// Обновление существующего подключения
pub async fn update(dto: LlmConnectionDto) -> anyhow::Result<()> {
    let id_str = dto
        .id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("ID is required"))?;

    let mut aggregate: LlmConnection = repository::find_by_id(id_str)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Connection not found"))?;

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
    aggregate.allowed_models = dto.allowed_models;
    if let Some(ref at) = dto.agent_type {
        aggregate.agent_type = AgentType::from_str(at);
    }
    // available_models не обновляется через update, только через fetch_models endpoint

    aggregate
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    aggregate.before_write();

    if aggregate.is_primary {
        repository::clear_all_primary().await?;
    }

    repository::update(&aggregate).await
}

/// Мягкое удаление подключения
pub async fn delete(id: &str) -> anyhow::Result<()> {
    repository::soft_delete(id).await
}

/// Получение подключения по ID
pub async fn get_by_id(id: &str) -> anyhow::Result<Option<LlmConnection>> {
    repository::find_by_id(id).await
}

/// Получение списка всех подключений
pub async fn list_all() -> anyhow::Result<Vec<LlmConnection>> {
    repository::list_all().await
}

/// Получение пагинированного списка
pub async fn list_paginated(
    limit: u64,
    offset: u64,
    sort_by: &str,
    sort_desc: bool,
) -> anyhow::Result<(Vec<LlmConnection>, u64)> {
    repository::list_paginated(limit, offset, sort_by, sort_desc).await
}

/// Получение основного подключения
pub async fn get_primary() -> anyhow::Result<Option<LlmConnection>> {
    repository::find_primary().await
}

/// Первое подключение указанного типа агента.
pub async fn find_by_agent_type(
    agent_type: &AgentType,
) -> anyhow::Result<Option<LlmConnection>> {
    repository::find_by_agent_type(agent_type.as_str()).await
}

/// Вернуть подключение нужного типа агента; если его нет — клонировать основное
/// (или первое доступное) в новое подключение с этим типом. Используется почтовым
/// конвейером для маршрутизации письма к нужному специалисту. Обобщает
/// `a031_kb_edit::service::ensure_kb_admin_agent`, но работает над a038 —
/// именно против него чат a018 резолвит `agent_id`.
pub async fn ensure_connection_for(agent_type: AgentType) -> anyhow::Result<LlmConnection> {
    if let Some(c) = repository::find_by_agent_type(agent_type.as_str()).await? {
        return Ok(c);
    }

    let source = match repository::find_primary().await? {
        Some(c) => c,
        None => repository::list_all()
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| {
                anyhow::anyhow!("Нет ни одного LLM-подключения (a038). Настройте подключение.")
            })?,
    };

    let mut c = LlmConnection::new_for_insert(
        format!("conn-{}", agent_type.as_str()),
        format!("{} (авто)", agent_type.display_name()),
        source.provider_type.clone(),
        source.api_endpoint.clone(),
        source.api_key.clone(),
        source.model_name.clone(),
        source.temperature,
        source.max_tokens,
        None,
        false,
        source.available_models.clone(),
        source.allowed_models.clone(),
    );
    c.agent_type = agent_type;
    c.base.comment = Some(format!(
        "Создано автоматически для почтового конвейера на основе '{}'.",
        source.base.description
    ));
    c.validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;
    c.before_write();
    repository::insert(&c).await?;
    Ok(c)
}
