use super::openai_provider::OpenAiProvider;
use super::types::{ChatMessage, LlmError, LlmProvider, LlmResponse, ToolDefinition};
use async_trait::async_trait;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;

pub const OPENROUTER_DEFAULT_ENDPOINT: &str = "https://openrouter.ai/api/v1";

pub struct OpenRouterProvider {
    inner: OpenAiProvider,
    api_endpoint: String,
    api_key: String,
}

impl OpenRouterProvider {
    pub fn new(
        api_endpoint: String,
        api_key: String,
        model: String,
        temperature: f64,
        max_tokens: i32,
    ) -> Self {
        let endpoint = normalize_endpoint(api_endpoint);
        let inner = OpenAiProvider::new_compatible(
            "OpenRouter",
            endpoint.clone(),
            api_key.clone(),
            model,
            temperature,
            max_tokens,
            true,
            false,
        );

        Self {
            inner,
            api_endpoint: endpoint,
            api_key,
        }
    }

    pub async fn list_models(&self) -> Result<Vec<serde_json::Value>, LlmError> {
        let url = format!("{}/models", self.api_endpoint);
        let response = reqwest::Client::new()
            .get(url)
            .header(AUTHORIZATION, format!("Bearer {}", self.api_key))
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
                "OpenRouter models request failed: HTTP {} {}",
                status, body
            )));
        }

        let payload = response
            .json::<OpenRouterModelsResponse>()
            .await
            .map_err(|e| LlmError::ApiError(e.to_string()))?;

        Ok(payload
            .data
            .into_iter()
            .map(|m| {
                serde_json::json!({
                    "id": m.id,
                    "name": m.name,
                    "context_length": m.context_length,
                    "pricing": m.pricing,
                    "supported_parameters": m.supported_parameters,
                })
            })
            .collect())
    }
}

#[async_trait]
impl LlmProvider for OpenRouterProvider {
    async fn chat_completion(&self, messages: Vec<ChatMessage>) -> Result<LlmResponse, LlmError> {
        self.inner.chat_completion(messages).await
    }

    async fn chat_completion_with_tools(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolDefinition>,
    ) -> Result<LlmResponse, LlmError> {
        self.inner.chat_completion_with_tools(messages, tools).await
    }

    async fn test_connection(&self) -> Result<(), LlmError> {
        self.inner.test_connection().await
    }

    fn provider_name(&self) -> &str {
        "OpenRouter"
    }
}

#[derive(Debug, Deserialize)]
struct OpenRouterModelsResponse {
    data: Vec<OpenRouterModel>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterModel {
    id: String,
    name: Option<String>,
    context_length: Option<i64>,
    pricing: Option<serde_json::Value>,
    supported_parameters: Option<Vec<String>>,
}

fn normalize_endpoint(endpoint: String) -> String {
    let trimmed = endpoint.trim().trim_end_matches('/').to_string();
    if trimmed.is_empty() {
        OPENROUTER_DEFAULT_ENDPOINT.to_string()
    } else {
        trimmed
    }
}
