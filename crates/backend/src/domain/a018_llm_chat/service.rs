use super::repository;
use crate::domain::a017_llm_agent::repository as agent_repository;
use crate::shared::llm::openai_provider::OpenAiProvider;
use crate::shared::llm::types::{ChatMessage, LlmProvider};
use contracts::domain::a017_llm_agent::aggregate::LlmAgentId;
use contracts::domain::a018_llm_chat::aggregate::{ChatRole, LlmChat, LlmChatId, LlmChatMessage, LlmChatListItem};
use contracts::domain::common::AggregateId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmChatDto {
    pub id: Option<String>,
    pub code: Option<String>,
    pub description: String,
    pub comment: Option<String>,
    pub agent_id: String,
    pub model_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
    pub model_name: Option<String>,
}

/// Создание нового чата
pub async fn create(dto: LlmChatDto) -> anyhow::Result<Uuid> {
    let code = dto
        .code
        .clone()
        .unwrap_or_else(|| format!("CHAT-{}", Uuid::new_v4()));

    let agent_uuid =
        Uuid::parse_str(&dto.agent_id).map_err(|e| anyhow::anyhow!("Invalid agent_id: {}", e))?;
    let agent_id = LlmAgentId::new(agent_uuid);

    // Проверяем что агент существует и получаем его для model_name
    let agent = agent_repository::find_by_id(&agent_id.as_string())
        .await?
        .ok_or_else(|| anyhow::anyhow!("Agent not found: {}", dto.agent_id))?;

    let db = crate::shared::data::db::get_connection();

    // Используем модель из DTO или дефолтную из агента
    let model_name = dto.model_name.unwrap_or_else(|| agent.model_name.clone());

    let mut aggregate = LlmChat::new_for_insert(code, dto.description, agent_id, model_name);

    // Валидация
    aggregate
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    // Before write
    aggregate.before_write();

    let id = aggregate.base.id.0;

    // Сохранение через repository
    repository::insert(&db, &aggregate).await?;

    Ok(id)
}

/// Обновление чата
pub async fn update(dto: LlmChatDto) -> anyhow::Result<()> {
    let id_str = dto
        .id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("ID is required"))?;

    let chat_uuid =
        Uuid::parse_str(id_str).map_err(|e| anyhow::anyhow!("Invalid chat ID: {}", e))?;
    let chat_id = LlmChatId::new(chat_uuid);

    let db = crate::shared::data::db::get_connection();
    let mut aggregate = repository::find_by_id(&db, &chat_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Chat not found"))?;

    // Обновление полей
    if let Some(code) = dto.code {
        aggregate.base.code = code;
    }
    aggregate.base.description = dto.description;
    aggregate.base.comment = dto.comment;

    if let Some(new_agent_id_str) = Some(dto.agent_id) {
        let agent_uuid = Uuid::parse_str(&new_agent_id_str)
            .map_err(|e| anyhow::anyhow!("Invalid agent_id: {}", e))?;
        aggregate.agent_id = LlmAgentId::new(agent_uuid);
    }

    if let Some(model_name) = dto.model_name {
        aggregate.model_name = model_name;
    }

    // Валидация
    aggregate
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    // Before write
    aggregate.before_write();

    // Сохранение
    repository::update(&db, &aggregate).await?;

    Ok(())
}

/// Удаление чата (soft delete)
pub async fn delete(id: &str) -> anyhow::Result<()> {
    let chat_uuid = Uuid::parse_str(id).map_err(|e| anyhow::anyhow!("Invalid chat ID: {}", e))?;
    let chat_id = LlmChatId::new(chat_uuid);

    let db = crate::shared::data::db::get_connection();
    repository::soft_delete(&db, &chat_id).await?;

    Ok(())
}

/// Получить чат по ID
pub async fn get_by_id(id: &str) -> anyhow::Result<Option<LlmChat>> {
    let chat_uuid = Uuid::parse_str(id).map_err(|e| anyhow::anyhow!("Invalid chat ID: {}", e))?;
    let chat_id = LlmChatId::new(chat_uuid);

    let db = crate::shared::data::db::get_connection();
    let chat = repository::find_by_id(&db, &chat_id).await?;

    Ok(chat)
}

/// Получить список всех чатов
pub async fn list_all() -> anyhow::Result<Vec<LlmChat>> {
    let db = crate::shared::data::db::get_connection();
    let chats = repository::list_all(&db).await?;
    Ok(chats)
}

/// Получить список чатов с пагинацией
pub async fn list_paginated(page: u64, page_size: u64) -> anyhow::Result<(Vec<LlmChat>, u64)> {
    let db = crate::shared::data::db::get_connection();
    let (chats, total) = repository::list_paginated(&db, page, page_size).await?;
    Ok((chats, total))
}

/// Получить список чатов с подсчетом сообщений и временем последнего сообщения
pub async fn list_with_stats() -> anyhow::Result<Vec<LlmChatListItem>> {
    let db = crate::shared::data::db::get_connection();
    let chats = repository::list_with_stats(&db).await?;
    Ok(chats)
}

/// Получить все сообщения чата
pub async fn get_messages(chat_id: &str) -> anyhow::Result<Vec<LlmChatMessage>> {
    let chat_uuid =
        Uuid::parse_str(chat_id).map_err(|e| anyhow::anyhow!("Invalid chat ID: {}", e))?;
    let chat_id_obj = LlmChatId::new(chat_uuid);

    let db = crate::shared::data::db::get_connection();
    let messages = repository::find_messages_by_chat_id(&db, &chat_id_obj).await?;

    Ok(messages)
}

/// Отправить сообщение пользователя и получить ответ от LLM
pub async fn send_message(
    chat_id: &str,
    request: SendMessageRequest,
) -> anyhow::Result<LlmChatMessage> {
    let chat_uuid =
        Uuid::parse_str(chat_id).map_err(|e| anyhow::anyhow!("Invalid chat ID: {}", e))?;
    let chat_id_obj = LlmChatId::new(chat_uuid);

    let db = crate::shared::data::db::get_connection();

    // 1. Получить чат
    let chat = repository::find_by_id(&db, &chat_id_obj)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Chat not found"))?;

    // 2. Получить агента
    let agent = agent_repository::find_by_id(&chat.agent_id.as_string())
        .await?
        .ok_or_else(|| anyhow::anyhow!("Agent not found"))?;

    // Выбор модели: из запроса -> из чата -> из агента
    let model_to_use = request
        .model_name
        .as_ref()
        .map(|s| s.clone())
        .unwrap_or_else(|| chat.model_name.clone());

    // 3. Сохранить сообщение пользователя
    let user_msg = LlmChatMessage::user(chat_id_obj, request.content);
    repository::insert_message(&db, &user_msg).await?;

    // 4. Получить историю сообщений для контекста
    let history = repository::find_messages_by_chat_id(&db, &chat_id_obj).await?;

    // 5. Преобразовать историю в формат для LLM
    let mut llm_messages: Vec<ChatMessage> = Vec::new();

    // Добавить системный промпт если есть
    if let Some(system_prompt) = &agent.system_prompt {
        llm_messages.push(ChatMessage::system(system_prompt.clone()));
    }

    // Добавить историю
    for msg in &history {
        let chat_msg = ChatMessage {
            role: match msg.role {
                ChatRole::System => crate::shared::llm::types::ChatRole::System,
                ChatRole::User => crate::shared::llm::types::ChatRole::User,
                ChatRole::Assistant => crate::shared::llm::types::ChatRole::Assistant,
            },
            content: msg.content.clone(),
        };
        llm_messages.push(chat_msg);
    }

    // 6. Создать LLM провайдера с выбранной моделью
    let provider = OpenAiProvider::new_with_endpoint(
        agent.api_endpoint.clone(),
        agent.api_key.clone(),
        model_to_use.clone(),
        agent.temperature,
        agent.max_tokens,
    );

    // 7. Вызвать LLM
    let llm_response = provider
        .chat_completion(llm_messages)
        .await
        .map_err(|e| anyhow::anyhow!("LLM error: {:?}", e))?;

    // 8. Сохранить ответ ассистента с метаданными
    let assistant_msg = LlmChatMessage::new_with_metadata(
        chat_id_obj,
        ChatRole::Assistant,
        llm_response.content,
        llm_response.tokens_used,
        Some(model_to_use),
        llm_response.confidence,
    );

    repository::insert_message(&db, &assistant_msg).await?;

    Ok(assistant_msg)
}
