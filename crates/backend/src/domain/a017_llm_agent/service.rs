use super::repository;
use contracts::domain::a017_llm_agent::aggregate::{AgentType, LlmAgent, LlmProviderType};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmAgentDto {
    pub id: Option<String>,
    pub code: Option<String>,
    pub description: String,
    pub comment: Option<String>,
    /// Технические поля — вестигиальны (источник истины в подключении a038). Приходят
    /// пустыми из новой формы сотрудника; оставлены для обратной совместимости.
    #[serde(default)]
    pub provider_type: String,
    #[serde(default)]
    pub api_endpoint: String,
    #[serde(default)]
    pub api_key: String,
    /// Закреплённая сотрудником модель (пусто → дефолт подключения).
    #[serde(default)]
    pub model_name: String,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: i32,
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub is_primary: bool,
    pub available_models: Option<String>,
    /// Специализация (business_analyst | sales_analyst | marketer | financier |
    /// system_admin | kb_admin | plugin_admin | coordinator_admin). None → не менять.
    #[serde(default)]
    pub agent_type: Option<String>,
    /// Техническое подключение a038 (UUID), через которое работает сотрудник.
    #[serde(default)]
    pub connection_id: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub schedule_cron: Option<String>,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

fn default_temperature() -> f64 {
    0.7
}

fn default_max_tokens() -> i32 {
    4096
}

/// Создание нового агента LLM
pub async fn create(dto: LlmAgentDto) -> anyhow::Result<Uuid> {
    let code = dto
        .code
        .clone()
        .unwrap_or_else(|| format!("LLM-{}", Uuid::new_v4()));

    // Провайдер вестигиален у сотрудника (источник истины — подключение a038); пустое
    // значение из новой формы трактуем как OpenAI, чтобы не падать на конструкторе.
    let provider_type = if dto.provider_type.trim().is_empty() {
        LlmProviderType::OpenAI
    } else {
        LlmProviderType::from_str(&dto.provider_type)
            .map_err(|e| anyhow::anyhow!("Invalid provider type: {}", e))?
    };

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
        dto.available_models,
    );

    // Специализация определяет набор навыков/инструментов.
    if let Some(ref at) = dto.agent_type {
        aggregate.agent_type = AgentType::from_str(at);
    }
    // Персона сотрудника поверх технического подключения.
    aggregate.connection_id = dto.connection_id.filter(|s| !s.trim().is_empty());
    aggregate.avatar = dto.avatar.filter(|s| !s.trim().is_empty());
    aggregate.email = dto.email.filter(|s| !s.trim().is_empty());
    aggregate.schedule_cron = dto.schedule_cron.filter(|s| !s.trim().is_empty());
    aggregate.is_active = dto.is_active;

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

    // Технические поля вестигиальны: обновляем только если форма реально их прислала
    // (непустыми). Иначе сохраняем прежние значения (источник истины — подключение a038).
    if !dto.provider_type.trim().is_empty() {
        aggregate.provider_type = LlmProviderType::from_str(&dto.provider_type)
            .map_err(|e| anyhow::anyhow!("Invalid provider type: {}", e))?;
    }
    if !dto.api_endpoint.trim().is_empty() {
        aggregate.api_endpoint = dto.api_endpoint;
    }
    if !dto.api_key.trim().is_empty() {
        aggregate.api_key = dto.api_key;
    }
    // model_name переосмыслен как закреплённая модель сотрудника (пусто = сбросить закрепление).
    aggregate.model_name = dto.model_name;
    aggregate.system_prompt = dto.system_prompt;
    aggregate.is_primary = dto.is_primary;
    if let Some(ref at) = dto.agent_type {
        aggregate.agent_type = AgentType::from_str(at);
    }
    aggregate.connection_id = dto.connection_id.filter(|s| !s.trim().is_empty());
    aggregate.avatar = dto.avatar.filter(|s| !s.trim().is_empty());
    aggregate.email = dto.email.filter(|s| !s.trim().is_empty());
    aggregate.schedule_cron = dto.schedule_cron.filter(|s| !s.trim().is_empty());
    aggregate.is_active = dto.is_active;
    // available_models не обновляется через update, только через fetch_models endpoint

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

/// Вернуть активного сотрудника нужной специализации; если его нет — создать нового
/// на основе основного технического подключения a038. Используется почтовым конвейером
/// для маршрутизации письма к нужному специалисту. Именно против a017 чат a018 резолвит
/// персону (специализация + обязанности), а техника берётся из связанного подключения.
pub async fn ensure_employee_for(agent_type: AgentType) -> anyhow::Result<LlmAgent> {
    if let Some(a) = repository::find_active_by_agent_type(agent_type.as_str()).await? {
        return Ok(a);
    }

    // Новому сотруднику нужно техническое подключение: берём основное (или первое).
    let connection = match crate::domain::a038_llm_connection::service::get_primary().await? {
        Some(c) => c,
        None => crate::domain::a038_llm_connection::service::list_all()
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| {
                anyhow::anyhow!("Нет ни одного LLM-подключения (a038). Настройте подключение.")
            })?,
    };

    let mut emp = LlmAgent::new_for_insert(
        format!("emp-{}", agent_type.as_str()),
        format!("{} (авто)", agent_type.display_name()),
        connection.provider_type.clone(),
        connection.api_endpoint.clone(),
        connection.api_key.clone(),
        String::new(), // закреплённая модель пуста → наследуется дефолт подключения
        connection.temperature,
        connection.max_tokens,
        None,
        false,
        connection.available_models.clone(),
    );
    emp.agent_type = agent_type;
    emp.connection_id = Some(connection.to_string_id());
    emp.is_active = true;
    emp.base.comment = Some(format!(
        "Создан автоматически для почтового конвейера на основе подключения '{}'.",
        connection.base.description
    ));
    emp.validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;
    emp.before_write();
    repository::insert(&emp).await?;
    Ok(emp)
}
