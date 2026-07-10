use super::openai_provider::OpenAiProvider;
use super::openrouter_provider::{OpenRouterProvider, OPENROUTER_DEFAULT_ENDPOINT};
use super::types::{LlmError, LlmProvider};
use contracts::domain::a017_llm_agent::aggregate::{LlmAgent, LlmProviderType};
use contracts::domain::a038_llm_connection::aggregate::LlmConnection;

/// Лёгкая borrowed-конфигурация провайдера, общая для a017 (Agent) и a038 (Connection).
/// Позволяет фабрике не зависеть от конкретного агрегата.
pub struct ProviderSettings<'a> {
    pub provider_type: &'a LlmProviderType,
    pub api_endpoint: &'a str,
    pub api_key: &'a str,
    pub model_name: &'a str,
    pub temperature: f64,
    pub max_tokens: i32,
}

impl<'a> From<&'a LlmAgent> for ProviderSettings<'a> {
    fn from(a: &'a LlmAgent) -> Self {
        Self {
            provider_type: &a.provider_type,
            api_endpoint: &a.api_endpoint,
            api_key: &a.api_key,
            model_name: &a.model_name,
            temperature: a.temperature,
            max_tokens: a.max_tokens,
        }
    }
}

impl<'a> From<&'a LlmConnection> for ProviderSettings<'a> {
    fn from(c: &'a LlmConnection) -> Self {
        Self {
            provider_type: &c.provider_type,
            api_endpoint: &c.api_endpoint,
            api_key: &c.api_key,
            model_name: &c.model_name,
            temperature: c.temperature,
            max_tokens: c.max_tokens,
        }
    }
}

pub fn create_provider<'a>(
    settings: impl Into<ProviderSettings<'a>>,
    model_override: Option<&str>,
) -> Result<Box<dyn LlmProvider>, LlmError> {
    let s = settings.into();
    let model = model_override
        .filter(|m| !m.trim().is_empty())
        .unwrap_or(s.model_name)
        .to_string();

    match s.provider_type {
        LlmProviderType::OpenAI => Ok(Box::new(OpenAiProvider::new_with_endpoint(
            s.api_endpoint.to_string(),
            s.api_key.to_string(),
            model,
            s.temperature,
            s.max_tokens,
        ))),
        LlmProviderType::OpenRouter => Ok(Box::new(OpenRouterProvider::new(
            openrouter_endpoint(s.api_endpoint),
            s.api_key.to_string(),
            model,
            s.temperature,
            s.max_tokens,
        ))),
        other => Err(LlmError::UnsupportedProvider(other.as_str().to_string())),
    }
}

pub async fn list_models<'a>(
    settings: impl Into<ProviderSettings<'a>>,
) -> Result<Vec<serde_json::Value>, LlmError> {
    let s = settings.into();
    match s.provider_type {
        LlmProviderType::OpenAI => {
            let provider = OpenAiProvider::new_with_endpoint(
                s.api_endpoint.to_string(),
                s.api_key.to_string(),
                s.model_name.to_string(),
                s.temperature,
                s.max_tokens,
            );
            provider.list_models().await
        }
        LlmProviderType::OpenRouter => {
            let provider = OpenRouterProvider::new(
                openrouter_endpoint(s.api_endpoint),
                s.api_key.to_string(),
                s.model_name.to_string(),
                s.temperature,
                s.max_tokens,
            );
            provider.list_models().await
        }
        other => Err(LlmError::UnsupportedProvider(other.as_str().to_string())),
    }
}

fn openrouter_endpoint(api_endpoint: &str) -> String {
    if api_endpoint.trim().is_empty() {
        OPENROUTER_DEFAULT_ENDPOINT.to_string()
    } else {
        api_endpoint.to_string()
    }
}
