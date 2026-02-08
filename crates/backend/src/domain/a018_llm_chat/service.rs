use super::repository;
use crate::domain::a017_llm_agent::repository as agent_repository;
use crate::shared::llm::openai_provider::OpenAiProvider;
use crate::shared::llm::types::{ChatMessage, LlmProvider};
use axum::extract::Multipart;
use contracts::domain::a017_llm_agent::aggregate::LlmAgentId;
use contracts::domain::a018_llm_chat::aggregate::{
    ChatRole, LlmChat, LlmChatAttachment, LlmChatId, LlmChatListItem, LlmChatMessage,
};
use contracts::domain::common::AggregateId;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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
    #[serde(default)]
    pub attachment_ids: Vec<String>,
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

    // 3. Обработать вложения если есть
    let mut content_with_attachments = request.content.clone();

    if !request.attachment_ids.is_empty() {
        let mut attachment_contents = Vec::new();

        for att_id_str in &request.attachment_ids {
            let att_uuid = Uuid::parse_str(att_id_str)
                .map_err(|e| anyhow::anyhow!("Invalid attachment ID: {}", e))?;

            // Найти вложение
            let attachments = repository::find_attachments_by_message_id(&db, &Uuid::nil()).await?;
            if let Some(attachment) = attachments.into_iter().find(|a| a.id == att_uuid) {
                // Прочитать содержимое файла
                match read_text_file(&attachment.filepath).await {
                    Ok(file_content) => {
                        attachment_contents
                            .push(format!("--- {} ---\n{}", attachment.filename, file_content));
                    }
                    Err(e) => {
                        tracing::warn!("Failed to read attachment {}: {}", attachment.filename, e);
                    }
                }
            }
        }

        if !attachment_contents.is_empty() {
            content_with_attachments.push_str("\n\nПрикрепленные файлы:\n");
            content_with_attachments.push_str(&attachment_contents.join("\n\n"));
        }
    }

    // 4. Сохранить сообщение пользователя
    let user_msg = LlmChatMessage::user(chat_id_obj, request.content);
    repository::insert_message(&db, &user_msg).await?;

    // 5. Обновить вложения, чтобы они были привязаны к сообщению пользователя
    for att_id_str in &request.attachment_ids {
        if let Ok(att_uuid) = Uuid::parse_str(att_id_str) {
            // Удалить старую запись и создать новую с правильным message_id
            let attachments = repository::find_attachments_by_message_id(&db, &Uuid::nil()).await?;
            if let Some(mut attachment) = attachments.into_iter().find(|a| a.id == att_uuid) {
                repository::delete_attachments_by_message_id(&db, &Uuid::nil()).await?;
                attachment.message_id = user_msg.id;
                repository::insert_attachment(&db, &attachment).await?;
            }
        }
    }

    // 6. Получить историю сообщений для контекста
    let mut history = repository::find_messages_by_chat_id(&db, &chat_id_obj).await?;

    // Заменить последнее сообщение (user_msg) контентом с вложениями
    if let Some(last_msg) = history.last_mut() {
        if last_msg.id == user_msg.id {
            last_msg.content = content_with_attachments.clone();
        }
    }

    // 7. Преобразовать историю в формат для LLM
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

    // 8. Создать LLM провайдера с выбранной моделью
    let provider = OpenAiProvider::new_with_endpoint(
        agent.api_endpoint.clone(),
        agent.api_key.clone(),
        model_to_use.clone(),
        agent.temperature,
        agent.max_tokens,
    );

    // 9. Вызвать LLM и измерить время выполнения
    let start = std::time::Instant::now();
    let llm_response = provider
        .chat_completion(llm_messages)
        .await
        .map_err(|e| anyhow::anyhow!("LLM error: {:?}", e))?;
    let duration_ms = start.elapsed().as_millis() as i64;

    // 10. Сохранить ответ ассистента с метаданными
    let assistant_msg = LlmChatMessage::new_with_metadata(
        chat_id_obj,
        ChatRole::Assistant,
        llm_response.content,
        llm_response.tokens_used,
        Some(model_to_use),
        llm_response.confidence,
        Some(duration_ms),
    );

    repository::insert_message(&db, &assistant_msg).await?;

    Ok(assistant_msg)
}

/// Загрузить файл-вложение для чата
pub async fn upload_attachment(
    chat_id: &str,
    multipart: &mut Multipart,
) -> anyhow::Result<LlmChatAttachment> {
    // Проверить что чат существует
    let chat_uuid =
        Uuid::parse_str(chat_id).map_err(|e| anyhow::anyhow!("Invalid chat ID: {}", e))?;
    let chat_id_obj = LlmChatId::new(chat_uuid);

    let db = crate::shared::data::db::get_connection();
    repository::find_by_id(&db, &chat_id_obj)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Chat not found"))?;

    // Получить файл из multipart
    let field = multipart
        .next_field()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read multipart field: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("No file in multipart"))?;

    let filename = field
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("No filename in multipart"))?
        .to_string();

    let content_type = field
        .content_type()
        .unwrap_or("application/octet-stream")
        .to_string();

    let data = field
        .bytes()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read file bytes: {}", e))?;

    let file_size = data.len() as i64;

    // Создать директорию для вложений если не существует
    let upload_dir = PathBuf::from("uploads")
        .join("chat_attachments")
        .join(chat_id);
    tokio::fs::create_dir_all(&upload_dir)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create upload directory: {}", e))?;

    // Сохранить файл с уникальным именем
    let file_id = Uuid::new_v4();
    let file_ext = std::path::Path::new(&filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let stored_filename = if file_ext.is_empty() {
        format!("{}", file_id)
    } else {
        format!("{}.{}", file_id, file_ext)
    };

    let filepath = upload_dir.join(&stored_filename);
    tokio::fs::write(&filepath, &data)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to write file: {}", e))?;

    // Создать запись о вложении (без привязки к сообщению пока)
    let attachment = LlmChatAttachment::new(
        Uuid::nil(), // Пока без message_id, привяжем при отправке сообщения
        filename,
        filepath.to_string_lossy().to_string(),
        content_type,
        file_size,
    );

    // Сохранить временную запись (с nil UUID для message_id)
    // Позже при отправке сообщения обновим message_id
    repository::insert_attachment(&db, &attachment).await?;

    Ok(attachment)
}

/// Загрузить содержимое текстового файла
async fn read_text_file(filepath: &str) -> anyhow::Result<String> {
    tokio::fs::read_to_string(filepath)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))
}
