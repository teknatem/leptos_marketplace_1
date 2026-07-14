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
        LlmProviderType::DeepSeek => Ok(Box::new(OpenAiProvider::new_compatible(
            "DeepSeek",
            deepseek_endpoint(s.api_endpoint),
            s.api_key.to_string(),
            model,
            s.temperature,
            s.max_tokens,
            true,  // DeepSeek ждёт поле `max_tokens`, а не `max_completion_tokens`
            false, // logprobs выключены — как в проверенной OpenRouter-совместимой конфигурации
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
        LlmProviderType::DeepSeek => {
            // DeepSeek /models возвращает {id, object, owned_by} без поля `created`,
            // на котором типизированный клиент async-openai падает при десериализации.
            // Поэтому тянем список сырым запросом с толерантными к отсутствию полями.
            deepseek_list_models(&deepseek_endpoint(s.api_endpoint), s.api_key).await
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

const DEEPSEEK_DEFAULT_ENDPOINT: &str = "https://api.deepseek.com";

fn deepseek_endpoint(api_endpoint: &str) -> String {
    if api_endpoint.trim().is_empty() {
        DEEPSEEK_DEFAULT_ENDPOINT.to_string()
    } else {
        api_endpoint.to_string()
    }
}

/// Список моделей DeepSeek: сырой GET `{endpoint}/models` с толерантным разбором
/// (в ответе нет `created`, на котором падает типизированный клиент async-openai).
async fn deepseek_list_models(
    endpoint: &str,
    api_key: &str,
) -> Result<Vec<serde_json::Value>, LlmError> {
    use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};

    #[derive(serde::Deserialize)]
    struct DeepSeekModelsResponse {
        data: Vec<DeepSeekModel>,
    }
    #[derive(serde::Deserialize)]
    struct DeepSeekModel {
        id: String,
        #[serde(default)]
        owned_by: Option<String>,
    }

    let url = format!("{}/models", endpoint.trim_end_matches('/'));
    let response = reqwest::Client::new()
        .get(url)
        .header(AUTHORIZATION, format!("Bearer {}", api_key))
        .header(CONTENT_TYPE, "application/json")
        .send()
        .await
        .map_err(|e| LlmError::NetworkError(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err(LlmError::AuthError(body));
        }
        if status.as_u16() == 429 {
            return Err(LlmError::RateLimitExceeded);
        }
        return Err(LlmError::ApiError(format!(
            "DeepSeek models request failed: HTTP {} {}",
            status, body
        )));
    }

    let payload = response
        .json::<DeepSeekModelsResponse>()
        .await
        .map_err(|e| LlmError::ApiError(e.to_string()))?;

    Ok(payload
        .data
        .into_iter()
        .map(|m| {
            serde_json::json!({
                "id": m.id,
                "owned_by": m.owned_by,
            })
        })
        .collect())
}
