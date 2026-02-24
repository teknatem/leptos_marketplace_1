use super::types::{ChatMessage, ChatRole, LlmError, LlmProvider, LlmResponse, ToolCall, ToolDefinition};
use async_openai::{
    config::OpenAIConfig,
    types::chat::{
        ChatChoice, ChatCompletionMessageToolCall, ChatCompletionMessageToolCalls,
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestToolMessageArgs,
        ChatCompletionRequestUserMessageArgs, ChatCompletionTool, ChatCompletionTools,
        CreateChatCompletionRequestArgs, FunctionCall, FunctionObject,
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
                    .content(msg.content_str())
                    .build()
                    .map_err(|e| LlmError::InvalidRequest(e.to_string()))?
                    .into(),
                ChatRole::User => ChatCompletionRequestUserMessageArgs::default()
                    .content(msg.content_str())
                    .build()
                    .map_err(|e| LlmError::InvalidRequest(e.to_string()))?
                    .into(),
                ChatRole::Assistant => {
                    let mut builder = ChatCompletionRequestAssistantMessageArgs::default();
                    if let Some(content) = &msg.content {
                        builder.content(content.as_str());
                    }
                    if let Some(tool_calls) = &msg.tool_calls {
                        let openai_tool_calls: Vec<ChatCompletionMessageToolCalls> = tool_calls
                            .iter()
                            .map(|tc| ChatCompletionMessageToolCalls::Function(
                                ChatCompletionMessageToolCall {
                                    id: tc.id.clone(),
                                    function: FunctionCall {
                                        name: tc.name.clone(),
                                        arguments: tc.arguments.clone(),
                                    },
                                }
                            ))
                            .collect();
                        builder.tool_calls(openai_tool_calls);
                    }
                    builder
                        .build()
                        .map_err(|e| LlmError::InvalidRequest(e.to_string()))?
                        .into()
                }
                ChatRole::Tool => {
                    let tool_call_id = msg
                        .tool_call_id
                        .as_deref()
                        .unwrap_or("")
                        .to_string();
                    ChatCompletionRequestToolMessageArgs::default()
                        .content(msg.content_str())
                        .tool_call_id(tool_call_id)
                        .build()
                        .map_err(|e| LlmError::InvalidRequest(e.to_string()))?
                        .into()
                }
            };
            openai_messages.push(openai_msg);
        }

        Ok(openai_messages)
    }

    /// Конвертировать определения инструментов в формат OpenAI
    fn convert_tools(&self, tools: Vec<ToolDefinition>) -> Vec<ChatCompletionTools> {
        tools
            .into_iter()
            .map(|t| ChatCompletionTools::Function(ChatCompletionTool {
                function: FunctionObject {
                    name: t.name,
                    description: Some(t.description),
                    parameters: Some(t.parameters),
                    strict: None,
                },
            }))
            .collect()
    }

    /// Извлечь tool calls из ответа OpenAI
    fn extract_tool_calls(&self, choice: &ChatChoice) -> Vec<ToolCall> {
        let Some(tool_calls) = &choice.message.tool_calls else {
            return vec![];
        };
        tool_calls
            .iter()
            .filter_map(|tc| match tc {
                ChatCompletionMessageToolCalls::Function(f) => Some(ToolCall {
                    id: f.id.clone(),
                    name: f.function.name.clone(),
                    arguments: f.function.arguments.clone(),
                }),
                ChatCompletionMessageToolCalls::Custom(_) => None,
            })
            .collect()
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn chat_completion(&self, messages: Vec<ChatMessage>) -> Result<LlmResponse, LlmError> {
        self.chat_completion_with_tools(messages, vec![]).await
    }

    async fn chat_completion_with_tools(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolDefinition>,
    ) -> Result<LlmResponse, LlmError> {
        let openai_messages = self.convert_messages(messages)?;
        let has_tools = !tools.is_empty();

        // Создаём базовый запрос
        let mut request_builder = CreateChatCompletionRequestArgs::default();
        request_builder
            .model(&self.model)
            .messages(openai_messages);

        // Добавляем инструменты если есть
        if has_tools {
            request_builder.tools(self.convert_tools(tools));
        }

        // Добавляем расширенные параметры только для поддерживающих моделей
        if Self::supports_advanced_params(&self.model) {
            request_builder
                .temperature(self.temperature)
                .max_completion_tokens(self.max_tokens);
            // logprobs несовместимы с tool calling
            if !has_tools {
                request_builder.logprobs(true).top_logprobs(1);
            }
        }

        let request = request_builder
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
        let tool_calls = self.extract_tool_calls(choice);
        let tokens_used = response.usage.map(|u| u.total_tokens as i32);
        let finish_reason = choice.finish_reason.as_ref().map(|r| format!("{:?}", r));

        // Вычислить confidence из logprobs (только если нет tool calls)
        let confidence = if tool_calls.is_empty() {
            choice.logprobs.as_ref().and_then(|logprobs| {
                if let Some(content_logprobs) = &logprobs.content {
                    if content_logprobs.is_empty() {
                        return None;
                    }
                    let sum: f64 = content_logprobs
                        .iter()
                        .map(|token| (token.logprob as f64).exp())
                        .sum();
                    let count = content_logprobs.len();
                    if count > 0 { Some(sum / count as f64) } else { None }
                } else {
                    None
                }
            })
        } else {
            None
        };

        Ok(LlmResponse {
            content,
            tool_calls,
            tokens_used,
            model: response.model.clone(),
            finish_reason,
            confidence,
        })
    }

    async fn test_connection(&self) -> Result<(), LlmError> {
        let messages = vec![ChatMessage::user("Hello")];
        self.chat_completion(messages).await?;
        Ok(())
    }

    fn provider_name(&self) -> &str {
        "OpenAI"
    }
}

impl OpenAiProvider {
    /// Проверяет, поддерживает ли модель расширенные параметры (temperature, logprobs, max_tokens)
    /// 
    /// GPT-5 и o1/o3 модели имеют ограниченный API:
    /// - Не поддерживают кастомный temperature (только дефолт 1.0)
    /// - Не поддерживают logprobs для расчета confidence
    /// - Не поддерживают max_completion_tokens
    fn supports_advanced_params(model_id: &str) -> bool {
        let is_restricted = model_id.starts_with("gpt-5")
            || model_id.starts_with("o1-")
            || model_id.starts_with("o3-");
        
        !is_restricted
    }
    
    /// Проверяет, является ли модель подходящей для chat completion
    fn is_chat_model(model_id: &str) -> bool {
        // Включаем chat-модели
        let is_chat = model_id.starts_with("gpt-5")
            || model_id.starts_with("gpt-4")
            || model_id.starts_with("gpt-3.5")
            || model_id.starts_with("o1-")
            || model_id.starts_with("o3-")
            || model_id.starts_with("chatgpt-");
        
        // Исключаем специализированные модели
        let is_excluded = model_id.starts_with("text-embedding-")
            || model_id.starts_with("whisper-")
            || model_id.starts_with("tts-")
            || model_id.starts_with("dall-e-")
            || model_id.starts_with("text-moderation-")
            || model_id.starts_with("text-davinci-")
            || model_id.starts_with("text-curie-")
            || model_id.starts_with("text-babbage-")
            || model_id.starts_with("text-ada-")
            || model_id.starts_with("davinci-")
            || model_id.starts_with("curie-")
            || model_id.starts_with("babbage-")
            || model_id.starts_with("ada-")
            || model_id.contains("embedding")
            || model_id.contains("search")
            || model_id.contains("similarity")
            || model_id.contains("edit")
            || model_id.contains("insert")
            || model_id.contains(":ft-"); // fine-tuned модели
        
        is_chat && !is_excluded
    }
    
    /// Получить список доступных моделей для chat completion от OpenAI
    pub async fn list_models(&self) -> Result<Vec<serde_json::Value>, LlmError> {
        let response = self.client.models()
            .list()
            .await
            .map_err(|e| LlmError::ApiError(e.to_string()))?;
        
        let models: Vec<serde_json::Value> = response.data
            .into_iter()
            .filter(|m| Self::is_chat_model(&m.id))
            .map(|m| serde_json::json!({
                "id": m.id,
                "created": m.created,
                "owned_by": m.owned_by
            }))
            .collect();
        
        Ok(models)
    }
}
