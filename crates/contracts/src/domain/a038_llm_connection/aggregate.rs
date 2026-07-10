use crate::domain::common::{
    AggregateId, AggregateRoot, BaseAggregate, EntityMetadata, EventStore, Origin,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Переиспользуем enum'ы провайдера и типа/роли из a017 — семантика та же, дублировать не нужно.
// TODO(a017-retire): когда a017 будет удалён, перенести эти enum'ы сюда (в a038).
pub use crate::domain::a017_llm_agent::aggregate::{AgentType, LlmProviderType};

/// ID типа для агрегата LLM Connection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LlmConnectionId(pub Uuid);

impl LlmConnectionId {
    pub fn new(value: Uuid) -> Self {
        Self(value)
    }
    pub fn new_v4() -> Self {
        Self(Uuid::new_v4())
    }
    pub fn value(&self) -> Uuid {
        self.0
    }
}

impl AggregateId for LlmConnectionId {
    fn as_string(&self) -> String {
        self.0.to_string()
    }
    fn from_string(s: &str) -> Result<Self, String> {
        Uuid::parse_str(s)
            .map(LlmConnectionId::new)
            .map_err(|e| format!("Invalid UUID: {}", e))
    }
}

/// Агрегат LLM Connection — «Подключение LLM».
///
/// Первоклассная сущность «провайдер + креды + модели + персона». Отличается от a017 наличием
/// `allowed_models` — курируемого короткого списка технически совместимых/целесообразных моделей,
/// из которых можно выбирать в рамках чата.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConnection {
    #[serde(flatten)]
    pub base: BaseAggregate<LlmConnectionId>,

    /// Тип провайдера
    pub provider_type: LlmProviderType,

    /// API Endpoint
    pub api_endpoint: String,

    /// API ключ (зашифрованный)
    pub api_key: String,

    /// Название модели (по умолчанию)
    pub model_name: String,

    /// Temperature (0.0-2.0)
    pub temperature: f64,

    /// Max tokens
    pub max_tokens: i32,

    /// Системный промпт
    pub system_prompt: Option<String>,

    /// Флаг основного подключения
    pub is_primary: bool,

    /// Полный список моделей из API провайдера (JSON, кэш fetch-models)
    pub available_models: Option<String>,

    /// Курируемое подмножество разрешённых моделей (JSON-массив model_id).
    /// Именно из него можно выбирать модель в чате. Подмножество `available_models`.
    pub allowed_models: Option<String>,

    /// Тип/роль (персона) — определяет набор навыков/инструментов
    pub agent_type: AgentType,
}

impl LlmConnection {
    #[allow(clippy::too_many_arguments)]
    pub fn new_for_insert(
        code: String,
        description: String,
        provider_type: LlmProviderType,
        api_endpoint: String,
        api_key: String,
        model_name: String,
        temperature: f64,
        max_tokens: i32,
        system_prompt: Option<String>,
        is_primary: bool,
        available_models: Option<String>,
        allowed_models: Option<String>,
    ) -> Self {
        let base = BaseAggregate::new(LlmConnectionId::new_v4(), code, description);
        Self {
            base,
            provider_type,
            api_endpoint,
            api_key,
            model_name,
            temperature,
            max_tokens,
            system_prompt,
            is_primary,
            available_models,
            allowed_models,
            agent_type: AgentType::default(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_id(
        id: LlmConnectionId,
        code: String,
        description: String,
        provider_type: LlmProviderType,
        api_endpoint: String,
        api_key: String,
        model_name: String,
        temperature: f64,
        max_tokens: i32,
        system_prompt: Option<String>,
        is_primary: bool,
        available_models: Option<String>,
        allowed_models: Option<String>,
    ) -> Self {
        let base = BaseAggregate::new(id, code, description);
        Self {
            base,
            provider_type,
            api_endpoint,
            api_key,
            model_name,
            temperature,
            max_tokens,
            system_prompt,
            is_primary,
            available_models,
            allowed_models,
            agent_type: AgentType::default(),
        }
    }

    pub fn to_string_id(&self) -> String {
        self.base.id.as_string()
    }

    pub fn touch_updated(&mut self) {
        self.base.touch();
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.base.description.trim().is_empty() {
            return Err("Описание не может быть пустым".into());
        }
        if self.base.code.trim().is_empty() {
            return Err("Код не может быть пустым".into());
        }
        if self.api_endpoint.trim().is_empty() {
            return Err("API Endpoint обязателен".into());
        }
        if self.api_key.trim().is_empty() {
            return Err("API ключ обязателен".into());
        }
        if self.model_name.trim().is_empty() {
            return Err("Название модели обязательно".into());
        }
        if !(0.0..=2.0).contains(&self.temperature) {
            return Err("Temperature должна быть в диапазоне 0.0-2.0".into());
        }
        if self.max_tokens < 256 || self.max_tokens > 128000 {
            return Err("Max tokens должен быть в диапазоне 256-128000".into());
        }
        Ok(())
    }

    pub fn before_write(&mut self) {
        self.touch_updated();
    }

    /// Маскирование API ключа для отображения
    pub fn masked_api_key(&self) -> String {
        let key = &self.api_key;
        if key.len() <= 8 {
            return "****".to_string();
        }
        format!("{}...{}", &key[..4], &key[key.len() - 4..])
    }

    /// Разобрать `allowed_models` (JSON-массив строк) в вектор model_id.
    /// Пустой/невалидный JSON → пустой вектор (модель не ограничена курированием на бэке).
    pub fn allowed_models_list(&self) -> Vec<String> {
        self.allowed_models
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
            .unwrap_or_default()
    }
}

impl AggregateRoot for LlmConnection {
    type Id = LlmConnectionId;

    fn id(&self) -> Self::Id {
        self.base.id
    }

    fn code(&self) -> &str {
        &self.base.code
    }

    fn description(&self) -> &str {
        &self.base.description
    }

    fn metadata(&self) -> &EntityMetadata {
        &self.base.metadata
    }

    fn metadata_mut(&mut self) -> &mut EntityMetadata {
        &mut self.base.metadata
    }

    fn events(&self) -> &EventStore {
        &self.base.events
    }

    fn events_mut(&mut self) -> &mut EventStore {
        &mut self.base.events
    }

    fn aggregate_index() -> &'static str {
        "a038"
    }

    fn collection_name() -> &'static str {
        "llm_connection"
    }

    fn element_name() -> &'static str {
        "Подключение LLM"
    }

    fn list_name() -> &'static str {
        "Подключения LLM"
    }

    fn origin() -> Origin {
        Origin::Self_
    }
}
