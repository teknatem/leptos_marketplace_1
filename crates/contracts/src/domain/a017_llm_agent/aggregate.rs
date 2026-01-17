use crate::domain::common::{
    AggregateId, AggregateRoot, BaseAggregate, EntityMetadata, EventStore, Origin,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// ID типа для агрегата LLM Agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LlmAgentId(pub Uuid);

impl LlmAgentId {
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

impl AggregateId for LlmAgentId {
    fn as_string(&self) -> String {
        self.0.to_string()
    }
    fn from_string(s: &str) -> Result<Self, String> {
        Uuid::parse_str(s)
            .map(LlmAgentId::new)
            .map_err(|e| format!("Invalid UUID: {}", e))
    }
}

/// Тип провайдера LLM
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LlmProviderType {
    OpenAI,
    Anthropic,
    Ollama,
}

impl LlmProviderType {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "OpenAI" => Ok(LlmProviderType::OpenAI),
            "Anthropic" => Ok(LlmProviderType::Anthropic),
            "Ollama" => Ok(LlmProviderType::Ollama),
            _ => Err(format!("Unknown provider type: {}", s)),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            LlmProviderType::OpenAI => "OpenAI",
            LlmProviderType::Anthropic => "Anthropic",
            LlmProviderType::Ollama => "Ollama",
        }
    }
}

/// Агрегат LLM Agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmAgent {
    #[serde(flatten)]
    pub base: BaseAggregate<LlmAgentId>,

    /// Тип провайдера
    pub provider_type: LlmProviderType,

    /// API Endpoint
    pub api_endpoint: String,

    /// API ключ (зашифрованный)
    pub api_key: String,

    /// Название модели
    pub model_name: String,

    /// Temperature (0.0-2.0)
    pub temperature: f64,

    /// Max tokens
    pub max_tokens: i32,

    /// Системный промпт
    pub system_prompt: Option<String>,

    /// Флаг основного агента
    pub is_primary: bool,
}

impl LlmAgent {
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
    ) -> Self {
        let base = BaseAggregate::new(LlmAgentId::new_v4(), code, description);
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
        }
    }

    pub fn new_with_id(
        id: LlmAgentId,
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
        format!("{}...{}", &key[..4], &key[key.len()-4..])
    }
}

impl AggregateRoot for LlmAgent {
    type Id = LlmAgentId;

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
        "a017"
    }

    fn collection_name() -> &'static str {
        "llm_agent"
    }

    fn element_name() -> &'static str {
        "Агент LLM"
    }

    fn list_name() -> &'static str {
        "Агенты LLM"
    }

    fn origin() -> Origin {
        Origin::Self_
    }
}
