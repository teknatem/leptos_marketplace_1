use super::repository;
use crate::domain::a017_llm_agent::repository as agent_repository;
use crate::shared::llm::types::{ChatMessage, ChatRole as LlmChatRole};
use crate::shared::llm::{create_provider, execute_tool_call};
use axum::extract::Multipart;
use contracts::domain::a017_llm_agent::aggregate::LlmAgentId;
use contracts::domain::a018_llm_chat::aggregate::{
    ChatRole, LlmChat, LlmChatAttachment, LlmChatDetail, LlmChatId, LlmChatListItem, LlmChatMessage,
};
use contracts::domain::a018_llm_chat::context::ContextPackageSummary;
use contracts::domain::a019_llm_artifact::aggregate::LlmArtifactId;
use contracts::domain::common::AggregateId;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// Обрезать строку до `max_bytes` байт, не разрывая UTF-8 символы.
fn utf8_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut boundary = max_bytes;
    while boundary > 0 && !s.is_char_boundary(boundary) {
        boundary -= 1;
    }
    &s[..boundary]
}

/// Максимальное число итераций tool calling в одном запросе.
/// Увеличено до 6: knowledge-flow требует до 3 шагов
/// (search_knowledge → get_knowledge → get_entity_schema) перед финальным ответом.
const MAX_TOOL_ITERATIONS: usize = 10;

/// Системные промпты вынесены в реестр навыков (см. shared/llm/skills.rs):
/// базовый core-промпт + промпт-фрагменты активных навыков.

/// Максимальное число не-системных сообщений в контексте (sliding window)
const MAX_HISTORY_MESSAGES: usize = 20;

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

/// Получить чат по ID с именем агента
pub async fn get_by_id(id: &str) -> anyhow::Result<Option<LlmChatDetail>> {
    let chat_uuid = Uuid::parse_str(id).map_err(|e| anyhow::anyhow!("Invalid chat ID: {}", e))?;
    let chat_id = LlmChatId::new(chat_uuid);

    let db = crate::shared::data::db::get_connection();
    let chat = match repository::find_by_id(&db, &chat_id).await? {
        Some(c) => c,
        None => return Ok(None),
    };

    let agent_name = agent_repository::find_by_id(&chat.agent_id.as_string())
        .await
        .ok()
        .flatten()
        .map(|a| a.base.description.clone());

    Ok(Some(LlmChatDetail { chat, agent_name }))
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

            // Найти вложение по его id (а не сканировать все nil-вложения глобально)
            if let Some(attachment) = repository::find_attachment_by_id(&db, &att_uuid).await? {
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

    // 5. Привязать вложения к сохранённому сообщению пользователя точечным UPDATE
    // по id вложения (без удаления чужих незавершённых загрузок).
    for att_id_str in &request.attachment_ids {
        if let Ok(att_uuid) = Uuid::parse_str(att_id_str) {
            if let Err(e) =
                repository::bind_attachment_to_message(&db, &att_uuid, &user_msg.id).await
            {
                tracing::warn!("Failed to bind attachment {}: {}", att_id_str, e);
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

    // Sliding window: system-сообщения сохраняются полностью,
    // не-системные обрезаются до MAX_HISTORY_MESSAGES (последние N)
    if history.len() > MAX_HISTORY_MESSAGES {
        let system_msgs: Vec<_> = history
            .iter()
            .filter(|m| m.role == ChatRole::System)
            .cloned()
            .collect();
        let mut non_system: Vec<_> = history
            .iter()
            .filter(|m| m.role != ChatRole::System)
            .cloned()
            .collect();
        if non_system.len() > MAX_HISTORY_MESSAGES {
            non_system.drain(0..non_system.len() - MAX_HISTORY_MESSAGES);
        }
        history = system_msgs.into_iter().chain(non_system).collect();
    }

    // 7. Преобразовать историю в формат для LLM
    let mut llm_messages: Vec<ChatMessage> = Vec::new();

    // Системный промпт и набор инструментов собираются из АКТИВНЫХ навыков (skills).
    use crate::shared::llm::skills;
    let allowed = skills::allowed_skills_for(&agent.agent_type);
    // Быстрая (rule-based) предактивация навыка по интенту сообщения; полный LLM-роутер
    // идёт конкурентно ниже, а модель может добрать навыки через use_skill.
    let quick = crate::shared::llm::router::quick_intent(
        utf8_truncate(&content_with_attachments, 2000),
        &agent.agent_type,
    );
    let mut active_skills: Vec<&'static str> = match skills::skill_for_intent(&quick) {
        Some(s) if allowed.contains(&s.id) => vec![s.id],
        _ => skills::default_skills_for(&agent.agent_type)
            .into_iter()
            .filter(|id| allowed.contains(id))
            .collect(),
    };

    // Базовый промпт: кастомный из агента, иначе role-agnostic core. Далее — фрагменты навыков.
    let base_prompt = agent
        .system_prompt
        .clone()
        .unwrap_or_else(|| skills::core_prompt().to_string());
    let now = chrono::Local::now();
    let mut system_text = format!(
        "{}\n\n---\nСегодня: {}. Текущий месяц: {}. Текущий год: {}.",
        base_prompt,
        now.format("%Y-%m-%d"),
        now.format("%m.%Y"),
        now.format("%Y")
    );
    for id in &active_skills {
        if let Some(sk) = skills::skill_by_id(id) {
            system_text.push_str("\n\n---\n");
            system_text.push_str(sk.prompt);
        }
    }
    tracing::info!(
        "[skills] chat_id='{}' quick_intent='{}' active={:?}",
        chat_id,
        quick,
        active_skills
    );
    llm_messages.push(ChatMessage::system(system_text));

    // Контекст прикреплённых страниц (если есть) — отдельным system-сообщением.
    // Привязка хранится в БД (context_package.chat_id), поэтому доступна каждый вызов.
    if let Ok(ctxs) = repository::list_context_by_chat(&db, chat_id).await {
        if !ctxs.is_empty() {
            // Дедуп: один и тот же объект/страница мог прикрепляться несколько раз —
            // оставляем только новейший пакет на ключ (entity_id, иначе page_key).
            // Глобальный потолок суммарного объёма: не раздувать контекст на множестве
            // крупных пакетов (каждый rendered_text — до 24KB).
            const MAX_CONTEXT_BLOCK_BYTES: usize = 48_000;
            use std::collections::HashSet;
            let mut seen: HashSet<String> = HashSet::new();
            let mut chosen: Vec<&_> = Vec::new();
            // Список отсортирован старые→новые; идём с конца, чтобы оставить новейший.
            for c in ctxs.iter().rev() {
                let dedup_key = c
                    .entity_id
                    .clone()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| c.page_key.clone());
                if seen.insert(dedup_key) {
                    chosen.push(c);
                }
            }
            chosen.reverse(); // восстановить хронологический порядок

            let mut block = String::from(
                "Контекст прикреплённых страниц приложения (данные текущих объектов/отчётов). \
                 Опирайся на него при ответе:\n\n",
            );
            let mut total = 0usize;
            let mut omitted = false;
            for c in &chosen {
                if total + c.rendered_text.len() > MAX_CONTEXT_BLOCK_BYTES {
                    omitted = true;
                    continue;
                }
                block.push_str(&c.rendered_text);
                block.push_str("\n---\n\n");
                total += c.rendered_text.len();
            }
            if omitted {
                block.push_str("…[часть прикреплённого контекста опущена из-за объёма]\n");
            }
            llm_messages.push(ChatMessage::system(block));
        }
    }

    // Добавить историю
    for msg in &history {
        let chat_msg = ChatMessage {
            role: match msg.role {
                ChatRole::System => LlmChatRole::System,
                ChatRole::User => LlmChatRole::User,
                ChatRole::Assistant => LlmChatRole::Assistant,
            },
            content: Some(msg.content.clone()),
            tool_calls: None,
            tool_call_id: None,
        };
        llm_messages.push(chat_msg);
    }

    // 8. Создать LLM провайдера с выбранной моделью
    let provider = create_provider(&agent, Some(&model_to_use))
        .map_err(|e| anyhow::anyhow!("LLM provider error: {}", e))?;

    // 8.5 Роутер интентов (Фаза 0): классифицируем запрос пользователя для
    // метаданных/аналитики. Поведение пайплайна (tools/промпт) пока НЕ меняем.
    // Запускаем КОНКУРЕНТНО с основным tool-циклом (tokio::join! ниже), чтобы
    // классификация не добавляла серийную задержку к каждому сообщению.
    let router_input = utf8_truncate(&content_with_attachments, 2000);
    let router_fut = crate::shared::llm::router::classify_intent(
        provider.as_ref(),
        router_input,
        "",
        &agent.agent_type,
    );

    // 9. Tool calling цикл: набор инструментов = core ∪ активные навыки.
    // tool_defs/active_tools/active_skills мутабельны — модель может активировать навыки
    // через use_skill (progressive disclosure), и набор пересобирается на лету.
    let mut tool_defs = skills::assemble_tools(&active_skills);
    let mut active_tools = skills::active_tool_names(&active_skills);
    let start = std::time::Instant::now();

    tracing::info!(
        "[llm_loop] chat_id='{}' model='{}' history_msgs={} tools={}",
        chat_id,
        model_to_use,
        llm_messages.len(),
        tool_defs.len()
    );

    // Основной цикл — отдельный future, чтобы выполняться конкурентно с роутером.
    // Блок захватывает `llm_messages` по &mut (только этот future его меняет),
    // а провайдер/инструменты — по общей ссылке (их же конкурентно читает роутер).
    let agent_id_str = chat.agent_id.as_string();
    let loop_fut = async {
        let mut final_response: Option<crate::shared::llm::types::LlmResponse> = None;
        let mut artifact_to_attach: Option<LlmArtifactId> = None;
        let mut tool_trace: Vec<serde_json::Value> = Vec::new();

        for iteration in 0..MAX_TOOL_ITERATIONS {
            let response = provider
                .chat_completion_with_tools(&llm_messages, &tool_defs)
                .await
                .map_err(|e| anyhow::anyhow!("LLM error: {:?}", e))?;

            tracing::info!(
                "[llm_iter] iter={} has_tool_calls={} finish_reason={:?} tokens={:?}",
                iteration + 1,
                response.has_tool_calls(),
                response.finish_reason,
                response.tokens_used
            );

            if !response.has_tool_calls() {
                // Финальный ответ — LLM завершил работу
                final_response = Some(response);
                break;
            }

            tracing::debug!(
                "Tool calling iteration {}: {} calls",
                iteration + 1,
                response.tool_calls.len()
            );

            // Добавить ответ ассистента с tool_calls в историю сообщений
            llm_messages.push(ChatMessage::assistant_with_tool_calls(
                response.tool_calls.clone(),
            ));

            // Навыки, активированные на этой итерации (применяем после всех tool-результатов,
            // чтобы не разрывать пару assistant(tool_calls) → tool_result).
            let mut newly_activated: Vec<&'static skills::Skill> = Vec::new();

            // Выполнить каждый tool call и добавить результаты
            for tool_call in &response.tool_calls {
                let call_start = std::time::Instant::now();
                tracing::info!(
                    "[tool_call] iter={} tool='{}' args={}",
                    iteration + 1,
                    tool_call.name,
                    utf8_truncate(&tool_call.arguments, 200)
                );
                let result = execute_tool_call(
                    tool_call,
                    chat_id,
                    &agent_id_str,
                    &agent.agent_type,
                    &active_tools,
                )
                .await;
                let call_ms = call_start.elapsed().as_millis() as u64;

                tracing::info!(
                    "[tool_result] tool='{}' ms={} ok={} preview={}",
                    tool_call.name,
                    call_ms,
                    !result.contains("\"error\""),
                    utf8_truncate(&result, 300)
                );

                // Разобрать результат для трассировки
                let parsed = serde_json::from_str::<serde_json::Value>(&result).ok();
                let is_ok = parsed
                    .as_ref()
                    .map(|v| v.get("_ok").and_then(|b| b.as_bool()).unwrap_or(true))
                    .unwrap_or(true);

                // Если tool call создал артефакт — запомнить его ID
                if let Some(ref v) = parsed {
                    if let Some(id_str) = v.get("artifact_id").and_then(|v| v.as_str()) {
                        if let Ok(uid) = Uuid::parse_str(id_str) {
                            artifact_to_attach = Some(LlmArtifactId::new(uid));
                            tracing::info!("Tool call produced artifact: {}", id_str);
                        }
                    }
                }

                // Активация навыка через use_skill (_activate_skill) — progressive disclosure.
                if let Some(ref v) = parsed {
                    if let Some(sid) = v.get("_activate_skill").and_then(|x| x.as_str()) {
                        if let Some(sk) = skills::skill_by_id(sid) {
                            if allowed.contains(&sk.id) && !active_skills.contains(&sk.id) {
                                active_skills.push(sk.id);
                                newly_activated.push(sk);
                            }
                        }
                    }
                }

                // Краткое описание результата для трассировки
                let summary = if !is_ok {
                    parsed
                        .as_ref()
                        .and_then(|v| v.get("error").and_then(|e| e.as_str()))
                        .unwrap_or("error")
                        .chars()
                        .take(120)
                        .collect::<String>()
                } else {
                    parsed
                        .as_ref()
                        .and_then(|v| {
                            // Подбираем лучшее краткое описание в зависимости от инструмента
                            v.get("row_count")
                                .and_then(|n| n.as_u64())
                                .map(|n| format!("{} rows", n))
                                .or_else(|| {
                                    v.get("total")
                                        .and_then(|n| n.as_u64())
                                        .map(|n| format!("{} items", n))
                                })
                                .or_else(|| {
                                    v.get("session_id")
                                        .and_then(|s| s.as_str())
                                        .map(|_| "artifact created".to_string())
                                })
                                .or_else(|| {
                                    v.get("artifact_id")
                                        .and_then(|s| s.as_str())
                                        .map(|_| "artifact created".to_string())
                                })
                        })
                        .unwrap_or_else(|| "ok".to_string())
                };

                tool_trace.push(serde_json::json!({
                    "tool":    tool_call.name,
                    "ok":      is_ok,
                    "ms":      call_ms,
                    "summary": summary,
                }));

                llm_messages.push(ChatMessage::tool_result(tool_call.id.clone(), result));
            }

            // Применить активированные навыки: пересобрать инструменты и дописать
            // их инструкции (после всех tool-результатов — пара assistant/tool не разорвана).
            if !newly_activated.is_empty() {
                tool_defs = skills::assemble_tools(&active_skills);
                active_tools = skills::active_tool_names(&active_skills);
                for sk in newly_activated {
                    tracing::info!("[skill] activated '{}' (chat='{}')", sk.id, chat_id);
                    llm_messages.push(ChatMessage::system(format!(
                        "Активирован навык «{}». Его инструменты и инструкции:\n\n{}",
                        sk.title, sk.prompt
                    )));
                }
            }
        }

        Ok::<_, anyhow::Error>((final_response, artifact_to_attach, tool_trace))
    };

    // Конкурентный запуск: роутер не добавляет серийную задержку.
    let (intent_result, loop_result) = tokio::join!(router_fut, loop_fut);
    let (final_response, artifact_to_attach, tool_trace) = loop_result?;

    tracing::info!(
        "[router] chat_id='{}' intent='{}' confidence={:.2} source={}",
        chat_id,
        intent_result.intent,
        intent_result.confidence,
        intent_result.source
    );

    let duration_ms = start.elapsed().as_millis() as i64;

    let llm_response = final_response.ok_or_else(|| {
        anyhow::anyhow!(
            "LLM exceeded maximum tool calling iterations ({})",
            MAX_TOOL_ITERATIONS
        )
    })?;

    // 10. Сохранить ответ ассистента с метаданными
    let mut assistant_msg = LlmChatMessage::new_with_metadata(
        chat_id_obj,
        ChatRole::Assistant,
        llm_response.content,
        llm_response.tokens_used,
        Some(model_to_use),
        llm_response.confidence,
        Some(duration_ms),
    );

    if let Some(artifact_id) = artifact_to_attach {
        assistant_msg.artifact_id = Some(artifact_id);
        assistant_msg.artifact_action =
            Some(contracts::domain::a018_llm_chat::aggregate::ArtifactAction::Created);
    }

    if !tool_trace.is_empty() {
        assistant_msg.tool_trace = serde_json::to_string(&tool_trace).ok();
    }

    // Метка интента от роутера (Фаза 0): для аналитики и UI-бейджа.
    assistant_msg.intent = Some(intent_result.intent.clone());

    repository::insert_message(&db, &assistant_msg).await?;

    Ok(assistant_msg)
}

/// Собрать контекст текущей страницы и привязать его к чату.
pub async fn add_chat_context(
    chat_id: &str,
    page_key: &str,
    label: Option<&str>,
) -> anyhow::Result<ContextPackageSummary> {
    let db = crate::shared::data::db::get_connection();
    let built = super::context::build_for_page_key(page_key, label).await;

    let id = Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();

    repository::insert_context_package(
        &db,
        repository::NewContextPackage {
            id: id.clone(),
            chat_id: Some(chat_id.to_string()),
            page_key: page_key.to_string(),
            page_type: built.page_type.clone(),
            entity_index: built.entity_index.clone(),
            entity_id: built.entity_id.clone(),
            title: built.title.clone(),
            context_json: built.context_json.to_string(),
            rendered_text: built.rendered_text.clone(),
            created_at: created_at.clone(),
        },
    )
    .await?;

    Ok(ContextPackageSummary {
        id,
        chat_id: Some(chat_id.to_string()),
        page_key: page_key.to_string(),
        page_type: built.page_type,
        title: built.title,
        created_at,
        rendered_text: built.rendered_text,
    })
}

/// Получить один пакет контекста по id (для details-страницы контекста).
pub async fn get_context_by_id(id: &str) -> anyhow::Result<Option<ContextPackageSummary>> {
    let db = crate::shared::data::db::get_connection();
    let row = repository::find_context_by_id(&db, id).await?;
    Ok(row.map(|m| ContextPackageSummary {
        id: m.id,
        chat_id: m.chat_id,
        page_key: m.page_key,
        page_type: m.page_type,
        title: m.title,
        created_at: m.created_at,
        rendered_text: m.rendered_text,
    }))
}

/// Список пакетов контекста, привязанных к чату.
pub async fn list_chat_context(chat_id: &str) -> anyhow::Result<Vec<ContextPackageSummary>> {
    let db = crate::shared::data::db::get_connection();
    let rows = repository::list_context_by_chat(&db, chat_id).await?;
    Ok(rows
        .into_iter()
        .map(|m| ContextPackageSummary {
            id: m.id,
            chat_id: m.chat_id,
            page_key: m.page_key,
            page_type: m.page_type,
            title: m.title,
            created_at: m.created_at,
            rendered_text: m.rendered_text,
        })
        .collect())
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
