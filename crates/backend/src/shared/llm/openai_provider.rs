use super::types::{ChatMessage, ChatRole, LlmError, LlmProvider, LlmResponse};
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use async_trait::async_trait;

/// OpenAI провайдер
pub struct OpenAiProvider {
    client: Client<OpenAIConfig>,
    model: String,
    temperature: f32,
    max_tokens: u32,
}

impl OpenAiProvider {
    /// Создать новый OpenAI провайдер
    pub fn new(api_key: String, model: String, temperature: f64, max_tokens: i32) -> Self {
        let config = OpenAIConfig::new().with_api_key(api_key);
        let client = Client::with_config(config);

        Self {
            client,
            model,
            temperature: temperature as f32,
            max_tokens: max_tokens as u32,
        }
    }

    /// Создать с кастомным endpoint (для совместимых API)
    pub fn new_with_endpoint(
        api_endpoint: String,
        api_key: String,
        model: String,
        temperature: f64,
        max_tokens: i32,
    ) -> Self {
        let config = OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(api_endpoint);
        let client = Client::with_config(config);

        Self {
            client,
            model,
            temperature: temperature as f32,
            max_tokens: max_tokens as u32,
        }
    }

    /// Конвертировать наши сообщения в формат OpenAI
    fn convert_messages(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<Vec<ChatCompletionRequestMessage>, LlmError> {
        let mut openai_messages = Vec::new();

        for msg in messages {
            let openai_msg = match msg.role {
                ChatRole::System => ChatCompletionRequestSystemMessageArgs::default()
                    .content(msg.content)
                    .build()
                    .map_err(|e| LlmError::InvalidRequest(e.to_string()))?
                    .into(),
                ChatRole::User => ChatCompletionRequestUserMessageArgs::default()
                    .content(msg.content)
                    .build()
                    .map_err(|e| LlmError::InvalidRequest(e.to_string()))?
                    .into(),
                ChatRole::Assistant => ChatCompletionRequestAssistantMessageArgs::default()
                    .content(msg.content)
                    .build()
                    .map_err(|e| LlmError::InvalidRequest(e.to_string()))?
                    .into(),
            };
            openai_messages.push(openai_msg);
        }

        Ok(openai_messages)
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn chat_completion(&self, messages: Vec<ChatMessage>) -> Result<LlmResponse, LlmError> {
        let openai_messages = self.convert_messages(messages)?;

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(openai_messages)
            .temperature(self.temperature)
            .max_tokens(self.max_tokens)
            .build()
            .map_err(|e| LlmError::InvalidRequest(e.to_string()))?;

        let response = self.client.chat().create(request).await.map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("401") || err_str.contains("authentication") {
                LlmError::AuthError(err_str)
            } else if err_str.contains("429") || err_str.contains("rate limit") {
                LlmError::RateLimitExceeded
            } else {
                LlmError::ApiError(err_str)
            }
        })?;

        let choice = response
            .choices
            .first()
            .ok_or_else(|| LlmError::ApiError("No response from API".to_string()))?;

        let content = choice.message.content.clone().unwrap_or_default();

        let tokens_used = response.usage.map(|u| u.total_tokens as i32);
        let finish_reason = choice.finish_reason.as_ref().map(|r| format!("{:?}", r));

        Ok(LlmResponse {
            content,
            tokens_used,
            model: response.model.clone(),
            finish_reason,
        })
    }

    async fn test_connection(&self) -> Result<(), LlmError> {
        // Простой тест - отправляем минимальный запрос
        let messages = vec![ChatMessage::user("Hello")];

        self.chat_completion(messages).await?;

        Ok(())
    }

    fn provider_name(&self) -> &str {
        "OpenAI"
    }
}
