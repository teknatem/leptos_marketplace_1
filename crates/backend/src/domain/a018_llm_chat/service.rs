use super::job_store;
use super::repository;
// Чат работает через «Подключение LLM» (a038). Связь хранится как UUID (chat.agent_id);
// после миграции a017→a038 (те же id) существующие чаты резолвятся против a038.
use crate::domain::a038_llm_connection::repository as agent_repository;
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
/// При исчерпании лимит обрабатывается мягко: делается финальный запрос без
/// инструментов, чтобы модель подытожила проделанную работу (см. ниже).
const MAX_TOOL_ITERATIONS: usize = 10;
const MAX_TOOL_FAILURES: usize = 6;

/// Системные промпты вынесены в реестр навыков (см. shared/llm/skills.rs):
/// базовый core-промпт + промпт-фрагменты активных навыков.

/// Токен-бюджет истории диалога (грубая оценка ~3 символа на токен для RU/EN-микса).
/// При превышении старая часть истории СУММИРУЕТСЯ (компакция) и заменяется сводкой,
/// которая сохраняется в чате (a018_llm_chat.summary_text/summary_upto).
const HISTORY_TOKEN_BUDGET: usize = 24_000;
/// Сколько «живой» (несжатой) истории оставляем после компакции.
const KEEP_RECENT_TOKENS: usize = 12_000;

/// Грубая оценка числа токенов в строке (~3 символа/токен).
fn estimate_tokens(s: &str) -> usize {
    s.chars().count() / 3 + 1
}

/// Оценка «веса» сообщения в контексте: контент + tool trace (он дописывается
/// к assistant-сообщениям при сборке контекста, cap 12KB — см. MAX_HISTORY_TRACE_BYTES).
fn message_tokens(m: &LlmChatMessage) -> usize {
    estimate_tokens(&m.content)
        + m.tool_trace
            .as_deref()
            .map(|t| estimate_tokens(t).min(4_000))
            .unwrap_or(0)
}

/// Индекс разреза для компакции: history[..idx] уходит в сводку, history[idx..]
/// остаётся живым хвостом (≈ KEEP_RECENT_TOKENS по оценке; system-сообщения не
/// считаются — они сохраняются отдельно). Последнее сообщение (текущий вопрос
/// пользователя) не компактится никогда. 0 — резать нечего.
fn compaction_cut_index(history: &[LlmChatMessage], total_tokens: usize) -> usize {
    if history.len() < 2 {
        return 0;
    }
    let mut running = 0usize;
    for (i, m) in history.iter().enumerate().take(history.len() - 1) {
        if m.role == ChatRole::System {
            continue;
        }
        running += message_tokens(m);
        if total_tokens - running <= KEEP_RECENT_TOKENS {
            return i + 1;
        }
    }
    // Даже один последний хвост больше KEEP — компактим всё, кроме текущего сообщения.
    history.len() - 1
}

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
    /// Client-generated idempotency key for background execution retries.
    #[serde(default)]
    pub request_id: Option<String>,
}

/// Создание нового чата. `owner_user_id` — текущий пользователь (владелец чата);
/// `None` для системных чатов (планировщик/автоматика), которые не привязаны к пользователю.
pub async fn create(dto: LlmChatDto, owner_user_id: Option<String>) -> anyhow::Result<Uuid> {
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

    let mut aggregate =
        LlmChat::new_for_insert(code, dto.description, agent_id, model_name, owner_user_id);

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

/// Установить пользовательскую оценку чата (1..5; None — снять оценку).
pub async fn set_rating(id: &str, rating: Option<i32>) -> anyhow::Result<()> {
    if let Some(r) = rating {
        if !(1..=5).contains(&r) {
            return Err(anyhow::anyhow!("Rating must be between 1 and 5"));
        }
    }

    let chat_uuid = Uuid::parse_str(id).map_err(|e| anyhow::anyhow!("Invalid chat ID: {}", e))?;
    let chat_id = LlmChatId::new(chat_uuid);

    let db = crate::shared::data::db::get_connection();
    let mut aggregate = repository::find_by_id(&db, &chat_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Chat not found"))?;

    aggregate.rating = rating;
    aggregate.before_write();
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

/// Получить список чатов с подсчетом сообщений и временем последнего сообщения.
/// Фильтрация по доступу: `is_admin` видит все, обычный пользователь — свои + общие.
pub async fn list_with_stats(
    viewer_id: &str,
    is_admin: bool,
) -> anyhow::Result<Vec<LlmChatListItem>> {
    let db = crate::shared::data::db::get_connection();
    let chats = repository::list_with_stats(&db, viewer_id, is_admin).await?;
    Ok(chats)
}

/// Установить признак «Общий доступ» у чата.
/// Менять статус может только владелец чата или superadmin (`is_admin`).
pub async fn set_shared(
    id: &str,
    is_shared: bool,
    requester_id: &str,
    is_admin: bool,
) -> anyhow::Result<()> {
    let chat_uuid = Uuid::parse_str(id).map_err(|e| anyhow::anyhow!("Invalid chat ID: {}", e))?;
    let chat_id = LlmChatId::new(chat_uuid);

    let db = crate::shared::data::db::get_connection();
    let mut aggregate = repository::find_by_id(&db, &chat_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Chat not found"))?;

    // Ownership-guard: только владелец или админ.
    if !is_admin && aggregate.owner_user_id.as_deref() != Some(requester_id) {
        return Err(anyhow::anyhow!("Forbidden: not the owner of this chat"));
    }

    aggregate.is_shared = is_shared;
    aggregate.before_write();
    repository::update(&db, &aggregate).await?;

    Ok(())
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

/// Полный журнал вызовов инструментов для сообщения ассистента (sys_tool_trace).
pub async fn get_tool_trace(
    message_id: &str,
) -> anyhow::Result<Vec<contracts::domain::a018_llm_chat::aggregate::ToolTraceEntry>> {
    let db = crate::shared::data::db::get_connection();
    let entries = repository::find_tool_trace_by_message(&db, message_id).await?;
    Ok(entries)
}

/// Человекочитаемая подпись инструмента для индикатора прогресса (показывается
/// пользователю, не модели). Неизвестные имена показываем как есть.
fn human_tool_label(name: &str) -> String {
    let label = match name {
        "find_data_sources" => "Поиск источника данных",
        "preview_data" => "Проверка данных",
        "build_chart" => "Создание графика",
        "build_table" => "Создание таблицы",
        "list_data_sources" => "Просмотр источников данных",
        "query_data_schema" => "Запрос данных по схеме",
        "run_data_view_scalar" | "run_data_view_drilldown" => "Расчёт по DataView",
        "execute_query" => "SQL-запрос",
        "list_entities" => "Просмотр каталога объектов",
        "get_entity_schema" => "Чтение схемы объекта",
        "get_join_hint" => "Поиск связи таблиц",
        "get_architecture_overview" => "Обзор архитектуры",
        "search_knowledge" => "Поиск в базе знаний",
        "get_knowledge" => "Чтение базы знаний",
        "chart_template" | "chart_examples" | "get_chart_ui_contract" => "Подготовка графика",
        "plugin_validate" => "Проверка плагина",
        "plugin_smoke_test" => "Проверка графика/таблицы",
        "plugin_upsert" => "Сохранение плагина",
        "plugin_invoke" => "Запуск плагина",
        "plugin_runs" => "История запусков плагина",
        "list_skills" | "use_skill" => "Подбор навыка",
        "check_system_health" => "Проверка состояния системы",
        "get_performance_stats" => "Метрики производительности",
        "list_background_jobs" => "Фоновые задачи",
        "get_data_integrity_report" => "Проверка целостности данных",
        "list_scheduled_tasks" => "Просмотр регламентных заданий",
        "describe_task_types" => "Справка по типам заданий",
        other => return format!("Инструмент: {other}"),
    };
    label.to_string()
}

/// Стабильный этап workflow для диагностики и UI. Имя инструмента остается техническим
/// идентификатором, а stage описывает его место в конвейере.
fn tool_stage(name: &str) -> &'static str {
    match name {
        "find_data_sources" => "discovery",
        "preview_data" => "preview",
        "build_chart" | "build_table" => "publish",
        _ => "tool",
    }
}

fn bounded_trace_value(value: serde_json::Value, max_bytes: usize) -> serde_json::Value {
    let encoded = serde_json::to_string(&value).unwrap_or_default();
    if encoded.len() <= max_bytes {
        value
    } else {
        serde_json::json!({
            "truncated": true,
            "preview": utf8_truncate(&encoded, max_bytes),
        })
    }
}

/// Вход этапа сохраняется отдельно от выхода. Это делает trace пригодным не только
/// для таймингов, но и для проверки фактического контракта между этапами.
fn trace_input(arguments: &str) -> serde_json::Value {
    let value = serde_json::from_str(arguments)
        .unwrap_or_else(|_| serde_json::json!({ "raw": utf8_truncate(arguments, 2_000) }));
    bounded_trace_value(value, 6_000)
}

/// Выход trace намеренно компактный: строки preview могут занимать сотни килобайт,
/// а для диагностики контракта нужны колонки, готовность, IDs и структурированная ошибка.
fn trace_output(value: Option<&serde_json::Value>) -> serde_json::Value {
    let Some(value) = value else {
        return serde_json::json!({ "error": "Tool returned non-JSON output" });
    };
    let mut out = serde_json::Map::new();
    for key in [
        "ok",
        "stage",
        "error_code",
        "error",
        "recommended_fix",
        "row_count",
        "total",
        "build_ready",
        "truncated",
        "source",
        "columns",
        "effective_context",
        "next_step",
        "artifact_id",
        "plugin_id",
        "revision_id",
        "session_id",
        "status",
        "data_modes",
        "presentation",
    ] {
        if let Some(item) = value.get(key) {
            out.insert(key.to_string(), item.clone());
        }
    }
    if let Some(sources) = value.get("sources").and_then(serde_json::Value::as_array) {
        let compact = sources
            .iter()
            .take(5)
            .map(|source| {
                serde_json::json!({
                    "id": source.get("id"),
                    "kind": source.get("kind"),
                    "name": source.get("name"),
                    "table": source.get("table"),
                    "source_template": source.get("source_template"),
                })
            })
            .collect::<Vec<_>>();
        out.insert("sources".to_string(), serde_json::Value::Array(compact));
    }
    if out.is_empty() {
        out.insert("result".to_string(), value.clone());
    }
    bounded_trace_value(serde_json::Value::Object(out), 8_000)
}

/// Записать текущий этап выполнения в job_store (если задача отслеживается).
async fn report_progress(job_id: Option<&str>, step: u32, stage: impl Into<String>) {
    if let Some(id) = job_id {
        job_store::set_progress(id, job_store::JobProgress::new(step, stage)).await;
    }
}

/// Отправить сообщение пользователя и получить ответ от LLM.
/// `job_id` — идентификатор фоновой задачи для отчёта о прогрессе (None для
/// синхронных вызовов из фоновых тасков, где прогресс не нужен).
pub async fn send_message(
    chat_id: &str,
    request: SendMessageRequest,
    job_id: Option<&str>,
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

    // Провайдер создаётся сразу: нужен и для компакции истории (ниже), и для цикла.
    let provider = create_provider(&agent, Some(&model_to_use))
        .map_err(|e| anyhow::anyhow!("LLM provider error: {}", e))?;

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

    // === Управление контекстом: персистентная сводка + токен-бюджет ===
    // 6.1 Ранее скомпактированная часть уже заменена сводкой — исключаем её из истории.
    let mut chat_summary: Option<String> = None;
    match repository::get_chat_summary(&db, &chat_id_obj).await {
        Ok(Some((text, upto))) => {
            if let Ok(upto_ts) = chrono::DateTime::parse_from_rfc3339(&upto) {
                let upto_utc = upto_ts.with_timezone(&chrono::Utc);
                history.retain(|m| m.role == ChatRole::System || m.created_at > upto_utc);
            }
            chat_summary = Some(text);
        }
        Ok(None) => {}
        Err(e) => tracing::warn!("get_chat_summary failed for chat {}: {}", chat_id, e),
    }

    // 6.2 Токен-бюджет: при превышении старая часть суммируется LLM-вызовом,
    // сводка сохраняется в чате, в контексте остаётся сводка + свежий хвост.
    let total_tokens: usize = history
        .iter()
        .filter(|m| m.role != ChatRole::System)
        .map(message_tokens)
        .sum();
    if total_tokens > HISTORY_TOKEN_BUDGET {
        report_progress(job_id, 0, "Сжимаю контекст диалога…").await;

        // Точка разреза: свежий хвост ≤ KEEP_RECENT_TOKENS остаётся живым.
        let cut = compaction_cut_index(&history, total_tokens);

        if cut > 0 {
            let recent = history.split_off(cut);
            let old_part = history;
            let old_system: Vec<_> = old_part
                .iter()
                .filter(|m| m.role == ChatRole::System)
                .cloned()
                .collect();
            let to_compact: Vec<_> = old_part
                .into_iter()
                .filter(|m| m.role != ChatRole::System)
                .collect();

            if let Some(last_old) = to_compact.last() {
                let upto_rfc = last_old.created_at.to_rfc3339();
                let mut payload = String::new();
                if let Some(prev) = &chat_summary {
                    payload.push_str("Предыдущая сводка (объедини её с новой информацией):\n");
                    payload.push_str(prev);
                    payload.push_str("\n\n");
                }
                payload.push_str("Сообщения для сжатия:\n");
                for m in &to_compact {
                    let role = match m.role {
                        ChatRole::User => "Пользователь",
                        ChatRole::Assistant => "Ассистент",
                        ChatRole::System => "Система",
                    };
                    payload.push_str(&format!(
                        "[{}] {}\n\n",
                        role,
                        utf8_truncate(&m.content, 4_000)
                    ));
                }
                let payload = utf8_truncate(&payload, 120_000).to_string();
                let sum_messages = vec![
                    ChatMessage::system(
                        "Ты сжимаешь историю диалога, чтобы ассистент продолжил работу с меньшим \
                         контекстом. Составь компактную сводку (до 400 слов): цель пользователя; \
                         ключевые факты, данные и принятые решения; созданные/изменённые объекты и \
                         артефакты с их id; незакрытые задачи и договорённости. Пиши фактами, без \
                         воды. Ответ — только текст сводки.",
                    ),
                    ChatMessage::user(payload),
                ];
                match provider.chat_completion(&sum_messages).await {
                    Ok(resp) if !resp.content.trim().is_empty() => {
                        let new_summary = resp.content.trim().to_string();
                        if let Err(e) = repository::set_chat_summary(
                            &db,
                            &chat_id_obj,
                            &new_summary,
                            &upto_rfc,
                        )
                        .await
                        {
                            tracing::warn!("set_chat_summary failed for chat {}: {}", chat_id, e);
                        }
                        tracing::info!(
                            "[compaction] chat='{}' compacted {} msgs (~{} tok) -> summary {} chars",
                            chat_id,
                            to_compact.len(),
                            total_tokens,
                            new_summary.len()
                        );
                        chat_summary = Some(new_summary);
                    }
                    Ok(_) => tracing::warn!(
                        "[compaction] пустая сводка — жёсткая обрезка без сводки на этот ход"
                    ),
                    Err(e) => tracing::warn!(
                        "[compaction] суммаризация не удалась ({:?}) — жёсткая обрезка на этот ход",
                        e
                    ),
                }
            }
            history = old_system.into_iter().chain(recent).collect();
        }
    }

    // Дописать содержимое вложений к user-сообщениям истории. В БД content хранит
    // только оригинальный ввод пользователя, поэтому текст файлов дочитывается с диска
    // при каждой сборке контекста — иначе модель теряла файлы на follow-up ходах.
    for msg in history.iter_mut() {
        if msg.role == ChatRole::User {
            append_attachments_text(&db, msg).await;
        }
    }

    // 7. Преобразовать историю в формат для LLM
    let mut llm_messages: Vec<ChatMessage> = Vec::new();

    // Системный промпт и набор инструментов собираются из АКТИВНЫХ навыков (skills).
    use crate::shared::llm::skills;
    let allowed = skills::allowed_skills_for(&agent.agent_type);
    // Прогрессивное раскрытие: стартуем с ТОНКОЙ базы (только core-инструменты). Домен-навык
    // модель активирует сама первым шагом через use_skill, увидев каталог ниже. Это экономит
    // токены (полные схемы инструментов не висят каждый ход) и не «роняет» навык на follow-up.
    // quick_intent оставляем как МЯГКИЙ fallback: если модель за первый ход навык не выбрала —
    // подстрахуем его в цикле ниже.
    let quick = crate::shared::llm::router::quick_intent(
        utf8_truncate(&content_with_attachments, 2000),
        &agent.agent_type,
    );
    let fallback_skill: Option<&'static skills::Skill> = match skills::skill_for_intent(&quick) {
        Some(s) if allowed.contains(&s.id) => Some(s),
        _ => skills::default_skills_for(&agent.agent_type)
            .into_iter()
            .find(|id| allowed.contains(id))
            .and_then(skills::skill_by_id),
    };
    let mut active_skills: Vec<&'static str> = fallback_skill
        .filter(|skill| matches!(skill.id, "chart-builder" | "table-builder"))
        .map(|skill| vec![skill.id])
        .unwrap_or_default();

    // Базовый промпт: кастомный из агента, иначе role-agnostic core.
    // ВАЖНО для кэша префикса у провайдеров (OpenAI/DeepSeek кэшируют префикс
    // автоматически): системный промпт держим байт-стабильным — без даты и прочих
    // меняющихся значений. Дата добавляется ОТДЕЛЬНЫМ system-сообщением после истории.
    let mut system_text = agent
        .system_prompt
        .clone()
        .unwrap_or_else(|| skills::core_prompt().to_string());
    // Краткий каталог доступных навыков (id + описание) — чтобы модель выбрала и активировала
    // нужный через use_skill БЕЗ отдельного round-trip на list_skills. Полные инструменты и
    // инструкции навыка подгружаются только после активации.
    if active_skills.is_empty() && !allowed.is_empty() {
        system_text.push_str(
            "\n\n---\nДоступные навыки (активируй ОДИН подходящий первым шагом — use_skill(\"<id>\")):\n",
        );
        for &id in &allowed {
            if let Some(sk) = skills::skill_by_id(id) {
                system_text.push_str(&format!("- {} — {}\n", sk.id, sk.description));
            }
        }
    }
    for id in &active_skills {
        if let Some(skill) = skills::skill_by_id(id) {
            system_text.push_str("\n\n---\n");
            system_text.push_str(skill.prompt);
        }
    }
    tracing::info!(
        "[skills] chat_id='{}' quick_intent='{}' fallback={:?} (thin base)",
        chat_id,
        quick,
        fallback_skill.map(|s| s.id)
    );
    llm_messages.push(ChatMessage::system(system_text));

    // Сводка ранней части диалога (компакция): заменяет сообщения старше summary_upto.
    if let Some(summary) = &chat_summary {
        llm_messages.push(ChatMessage::system(format!(
            "Сводка ранней части этого диалога (более старые сообщения заменены ею):\n\n{}",
            summary
        )));
    }

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
        let mut content = msg.content.clone();
        if msg.role == ChatRole::Assistant {
            if let Some(trace) = msg.tool_trace.as_deref().filter(|trace| !trace.is_empty()) {
                const MAX_HISTORY_TRACE_BYTES: usize = 12_000;
                content.push_str(
                    "\n\n[Служебная трассировка выполнения предыдущего ответа; используй её для точного продолжения и диагностики:]\n",
                );
                content.push_str(utf8_truncate(trace, MAX_HISTORY_TRACE_BYTES));
            }
        }
        let chat_msg = ChatMessage {
            role: match msg.role {
                ChatRole::System => LlmChatRole::System,
                ChatRole::User => LlmChatRole::User,
                ChatRole::Assistant => LlmChatRole::Assistant,
            },
            content: Some(content),
            tool_calls: None,
            tool_call_id: None,
        };
        llm_messages.push(chat_msg);
    }

    // Текущая дата — в конце (после истории), чтобы смена дня не инвалидировала
    // кэш префикса (system prompt + история) у провайдера.
    let now = chrono::Local::now();
    llm_messages.push(ChatMessage::system(format!(
        "Сегодня: {}. Текущий месяц: {}. Текущий год: {}.",
        now.format("%Y-%m-%d"),
        now.format("%m.%Y"),
        now.format("%Y")
    )));

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
    let chart_workflow = active_skills.as_slice() == ["chart-builder"];
    let table_workflow = active_skills.as_slice() == ["table-builder"];
    let builder_workflow = chart_workflow || table_workflow;
    let mut builder_next_tool = builder_workflow.then_some("find_data_sources");
    let mut tool_defs = skills::assemble_tools(&active_skills);
    if let Some(next) = builder_next_tool {
        tool_defs.retain(|tool| tool.name == next);
    }
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
        let mut total_tokens_used: i64 = 0;
        let mut tool_failures = 0usize;

        for iteration in 0..MAX_TOOL_ITERATIONS {
            if let Some(id) = job_id {
                if job_store::is_cancelled(id).await {
                    return Err(anyhow::anyhow!("LLM job cancelled"));
                }
            }
            let step = (iteration + 1) as u32;
            report_progress(job_id, step, "Модель думает…").await;

            // Стриминг: дельты текста складываются в job_store (in-memory),
            // фронт показывает их через poll/SSE ещё до завершения ответа.
            let response = match job_id {
                Some(id) => {
                    // Разделитель между текстом разных итераций (narration → финальный ответ)
                    if iteration > 0 && job_store::get_partial(id).is_some() {
                        job_store::append_partial(id, "\n\n");
                    }
                    let id_owned = id.to_string();
                    let sink = move |delta: &str| job_store::append_partial(&id_owned, delta);
                    provider
                        .chat_completion_with_tools_streaming(&llm_messages, &tool_defs, &sink)
                        .await
                }
                None => {
                    provider
                        .chat_completion_with_tools(&llm_messages, &tool_defs)
                        .await
                }
            }
            .map_err(|e| anyhow::anyhow!("LLM error: {:?}", e))?;
            total_tokens_used += response.tokens_used.unwrap_or(0) as i64;

            tracing::info!(
                "[llm_iter] iter={} has_tool_calls={} finish_reason={:?} tokens={:?}",
                iteration + 1,
                response.has_tool_calls(),
                response.finish_reason,
                response.tokens_used
            );

            if !response.has_tool_calls() {
                // Финальный ответ — LLM завершил работу
                report_progress(job_id, step, "Формирую ответ…").await;
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
            let tool_count = response.tool_calls.len();
            for (tool_idx, tool_call) in response.tool_calls.iter().enumerate() {
                if let Some(id) = job_id {
                    if job_store::is_cancelled(id).await {
                        return Err(anyhow::anyhow!("LLM job cancelled"));
                    }
                }
                let stage = if tool_count > 1 {
                    format!(
                        "{} ({}/{})",
                        human_tool_label(&tool_call.name),
                        tool_idx + 1,
                        tool_count
                    )
                } else {
                    human_tool_label(&tool_call.name)
                };
                report_progress(job_id, step, stage).await;

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

                // Разобрать результат: единственный источник истины об успехе — поле `_ok`
                // (его проставляет tool_executor каждому результату).
                let parsed = serde_json::from_str::<serde_json::Value>(&result).ok();
                let is_ok = parsed
                    .as_ref()
                    .map(|v| v.get("_ok").and_then(|b| b.as_bool()).unwrap_or(true))
                    .unwrap_or(true);
                if !is_ok {
                    tool_failures += 1;
                }

                tracing::info!(
                    "[tool_result] tool='{}' ms={} ok={} preview={}",
                    tool_call.name,
                    call_ms,
                    is_ok,
                    utf8_truncate(&result, 300)
                );

                // Детерминированная передача управления для builder-workflow. Модель выбирает
                // конкретный source/spec, но не может бесконечно возвращаться к discovery после
                // успешного preview или пропустить publish.
                if builder_workflow {
                    builder_next_tool = match tool_call.name.as_str() {
                        "find_data_sources" if is_ok => Some("preview_data"),
                        "find_data_sources" => Some("find_data_sources"),
                        "preview_data"
                            if is_ok
                                && (!chart_workflow
                                    || parsed.as_ref().and_then(|v| {
                                        v.get("build_ready").and_then(|ready| ready.as_bool())
                                    }) == Some(true)) =>
                        {
                            Some(if chart_workflow {
                                "build_chart"
                            } else {
                                "build_table"
                            })
                        }
                        "preview_data" => Some("preview_data"),
                        "build_chart" | "build_table" if is_ok => None,
                        "build_chart" | "build_table" => Some("preview_data"),
                        _ => builder_next_tool,
                    };
                }

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
                    "iteration": iteration + 1,
                    "call":     tool_idx + 1,
                    "stage":    tool_stage(&tool_call.name),
                    "tool":    tool_call.name,
                    "ok":      is_ok,
                    "ms":      call_ms,
                    "summary": summary,
                    "input":   trace_input(&tool_call.arguments),
                    "output":  trace_output(parsed.as_ref()),
                }));

                llm_messages.push(ChatMessage::tool_result(tool_call.id.clone(), result));
            }

            // Мягкий fallback: модель за этот ход не активировала навык и ни один ещё не активен —
            // подстрахуем по quick_intent, чтобы появились доменные инструменты (а не только core).
            // Срабатывает один раз (после активации active_skills уже непустой).
            if active_skills.is_empty() && newly_activated.is_empty() {
                if let Some(sk) = fallback_skill {
                    active_skills.push(sk.id);
                    newly_activated.push(sk);
                    tracing::info!(
                        "[skill] fallback-activated '{}' (quick_intent, chat='{}')",
                        sk.id,
                        chat_id
                    );
                }
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

            if let Some(next) = builder_next_tool {
                tool_defs = skills::assemble_tools(&active_skills);
                tool_defs.retain(|tool| tool.name == next);
            }

            // Публикация является терминальным выходом builder-workflow. После получения
            // artifact_id больше не даём модели запускать discovery/preview повторно.
            if builder_workflow && artifact_to_attach.is_some() {
                break;
            }

            if tool_failures >= MAX_TOOL_FAILURES {
                tracing::warn!(
                    "[llm_loop] stopped after {} failed tool calls (chat='{}')",
                    tool_failures,
                    chat_id
                );
                llm_messages.push(ChatMessage::system(
                    "Лимит технических ошибок инструментов исчерпан. Не вызывай инструменты снова; кратко сообщи пользователю, какой контракт аргументов не удалось выполнить."
                        .to_string(),
                ));
                break;
            }
        }

        if builder_workflow && artifact_to_attach.is_none() {
            tool_trace.push(serde_json::json!({
                "iteration": MAX_TOOL_ITERATIONS + 1,
                "call": 0,
                "stage": "publish",
                "tool": "workflow",
                "ok": false,
                "ms": 0,
                "summary": if chart_workflow { "График не создан" } else { "Таблица не создана" },
                "input": { "expected": if chart_workflow { "build_chart" } else { "build_table" } },
                "output": {
                    "error_code": "workflow_incomplete",
                    "error": "Builder workflow завершился без artifact_id",
                    "next_step": builder_next_tool,
                },
            }));
        }

        // Запрашиваем финальный ТЕКСТОВЫЙ ответ, если:
        //  - исчерпан лимит итераций (модель всё ещё звала инструменты), ИЛИ
        //  - модель завершила работу, но вернула ПУСТОЙ текст (наблюдается у DeepSeek: после
        //    серии tool-call'ов отдаёт пустой content — пользователь иначе получит пустое
        //    сообщение). В обоих случаях просим связный итог без инструментов.
        let final_is_blank = final_response
            .as_ref()
            .map(|r| r.content.trim().is_empty())
            .unwrap_or(true);
        if final_is_blank {
            tracing::warn!(
                "[llm_iter] финальный ответ пуст/не получен (лимит {} итераций?) — запрашиваю текстовый итог",
                MAX_TOOL_ITERATIONS
            );
            llm_messages.push(ChatMessage::system(
                "Заверши диалог: больше инструменты НЕ вызывай и НЕ возвращай пустой ответ. \
                 Подведи краткий итог пользователю на естественном языке: что уже сделано и его \
                 статус (полученные данные, созданные/обновлённые объекты и артефакты) и предложи \
                 следующий шаг."
                    .to_string(),
            ));
            let summary_result = match job_id {
                Some(id) => {
                    if job_store::get_partial(id).is_some() {
                        job_store::append_partial(id, "\n\n");
                    }
                    let id_owned = id.to_string();
                    let sink = move |delta: &str| job_store::append_partial(&id_owned, delta);
                    provider
                        .chat_completion_with_tools_streaming(&llm_messages, &[], &sink)
                        .await
                }
                None => provider.chat_completion(&llm_messages).await,
            };
            match summary_result {
                Ok(resp) => {
                    total_tokens_used += resp.tokens_used.unwrap_or(0) as i64;
                    final_response = Some(resp);
                }
                Err(e) => tracing::warn!(
                    "[llm_iter] финальный ответ без инструментов не удался: {:?}",
                    e
                ),
            }
        }

        if let Some(response) = final_response.as_mut() {
            response.tokens_used = Some(total_tokens_used.min(i32::MAX as i64) as i32);
        }
        let fallback_text = if artifact_to_attach.is_some() {
            "Результат создан, но модель не сформировала итоговый текст. Откройте карточку артефакта ниже."
        } else {
            "Не удалось завершить построение: модель исчерпала лимит технических попыток. Повторите запрос — диагностические детали сохранены в tool trace."
        };
        match final_response.as_mut() {
            Some(response) if response.content.trim().is_empty() => {
                response.content = fallback_text.to_string();
            }
            None => {
                final_response = Some(crate::shared::llm::types::LlmResponse {
                    content: fallback_text.to_string(),
                    tool_calls: vec![],
                    tokens_used: Some(total_tokens_used.min(i32::MAX as i64) as i32),
                    model: model_to_use.clone(),
                    finish_reason: Some("local_fallback".to_string()),
                    confidence: None,
                });
            }
            _ => {}
        }
        Ok::<_, anyhow::Error>((final_response, artifact_to_attach, tool_trace))
    };

    // Конкурентный запуск: роутер не добавляет серийную задержку.
    let (intent_result, loop_result) = tokio::join!(router_fut, loop_fut);
    let (mut final_response, artifact_to_attach, tool_trace) = loop_result?;
    if let Some(response) = final_response.as_mut() {
        let combined =
            response.tokens_used.unwrap_or(0) as i64 + intent_result.tokens_used.max(0) as i64;
        response.tokens_used = Some(combined.min(i32::MAX as i64) as i32);
    }

    tracing::info!(
        "[router] chat_id='{}' intent='{}' confidence={:.2} source={}",
        chat_id,
        intent_result.intent,
        intent_result.confidence,
        intent_result.source
    );

    let duration_ms = start.elapsed().as_millis() as i64;

    if builder_workflow {
        let stage = match (chart_workflow, artifact_to_attach.is_some()) {
            (true, true) => "График создан",
            (true, false) => "График не создан",
            (false, true) => "Таблица создана",
            (false, false) => "Таблица не создана",
        };
        report_progress(job_id, MAX_TOOL_ITERATIONS as u32 + 1, stage).await;
    }

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

    // Трасса вызовов инструментов теперь ведётся в двух местах с разной ролью:
    //  - tool_trace_json на сообщении — МИНИМУМ для пилюль (tool, ok, ms);
    //  - sys_tool_trace — полная запись на каждый вызов (вход/выход/summary),
    //    для детальной карточки в UI и кросс-чат аналитики.
    if !tool_trace.is_empty() {
        let pills: Vec<serde_json::Value> = tool_trace
            .iter()
            .map(|e| {
                serde_json::json!({
                    "tool": e.get("tool"),
                    "ok": e.get("ok"),
                    "ms": e.get("ms"),
                })
            })
            .collect();
        assistant_msg.tool_trace = serde_json::to_string(&pills).ok();
    }

    // Метка интента от роутера (Фаза 0): для аналитики и UI-бейджа.
    assistant_msg.intent = Some(intent_result.intent.clone());

    repository::insert_message(&db, &assistant_msg).await?;

    // Полный журнал вызовов — отдельной пачкой, уже с известным message_id.
    if !tool_trace.is_empty() {
        let message_id = assistant_msg.id.to_string();
        let created_at = assistant_msg.created_at;
        let rows: Vec<contracts::domain::a018_llm_chat::aggregate::ToolTraceEntry> = tool_trace
            .iter()
            .map(
                |e| contracts::domain::a018_llm_chat::aggregate::ToolTraceEntry {
                    id: Uuid::new_v4().to_string(),
                    chat_id: chat_id.to_string(),
                    message_id: message_id.clone(),
                    iteration: e.get("iteration").and_then(|v| v.as_i64()).unwrap_or(0),
                    call_index: e.get("call").and_then(|v| v.as_i64()).unwrap_or(0),
                    stage: e
                        .get("stage")
                        .and_then(|v| v.as_str())
                        .unwrap_or("tool")
                        .to_string(),
                    tool: e
                        .get("tool")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    ok: e.get("ok").and_then(|v| v.as_bool()).unwrap_or(true),
                    ms: e.get("ms").and_then(|v| v.as_i64()).unwrap_or(0),
                    summary: e
                        .get("summary")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    input: e.get("input").cloned(),
                    output: e.get("output").cloned(),
                    created_at,
                },
            )
            .collect();
        if let Err(err) = repository::insert_tool_trace_batch(&db, &rows).await {
            // Журнал вспомогательный — его сбой не должен рушить ответ ассистента.
            tracing::warn!("[tool_trace] failed to persist sys_tool_trace: {}", err);
        }
    }

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

/// Потолок на текст одного вложения в контексте LLM — чтобы один большой файл
/// не вытеснил остальную историю.
const MAX_ATTACHMENT_FILE_BYTES: usize = 64_000;

/// Дописать к тексту сообщения содержимое его вложений (для контекста LLM).
async fn append_attachments_text(
    db: &sea_orm::DatabaseConnection,
    msg: &mut LlmChatMessage,
) {
    let atts = match repository::find_attachments_by_message_id(db, &msg.id).await {
        Ok(atts) => atts,
        Err(e) => {
            tracing::warn!("Failed to load attachments for message {}: {}", msg.id, e);
            return;
        }
    };
    if atts.is_empty() {
        return;
    }
    let mut parts = Vec::new();
    for att in &atts {
        match read_text_file(&att.filepath).await {
            Ok(content) => {
                let truncated = utf8_truncate(&content, MAX_ATTACHMENT_FILE_BYTES);
                let suffix = if truncated.len() < content.len() {
                    "\n…[файл обрезан]"
                } else {
                    ""
                };
                parts.push(format!("--- {} ---\n{}{}", att.filename, truncated, suffix));
            }
            Err(e) => {
                tracing::warn!("Failed to read attachment {}: {}", att.filename, e);
            }
        }
    }
    if !parts.is_empty() {
        msg.content.push_str("\n\nПрикрепленные файлы:\n");
        msg.content.push_str(&parts.join("\n\n"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(chars: usize) -> LlmChatMessage {
        LlmChatMessage::user(LlmChatId::new(Uuid::nil()), "я".repeat(chars))
    }

    #[test]
    fn estimate_tokens_rough_thirds() {
        assert_eq!(estimate_tokens(""), 1);
        assert_eq!(estimate_tokens(&"a".repeat(300)), 101);
        // Кириллица считается по символам, не по байтам (UTF-8: 2 байта/символ).
        assert_eq!(estimate_tokens(&"я".repeat(300)), 101);
    }

    #[test]
    fn compaction_keeps_recent_tail() {
        // 4 сообщения по ~9000 токенов: total ~36k > бюджета 24k.
        // Живым остаётся хвост ≤ 12k — последнее сообщение, cut = 3.
        let history: Vec<_> = (0..4).map(|_| msg(27_000)).collect();
        let total: usize = history.iter().map(message_tokens).sum();
        assert!(total > HISTORY_TOKEN_BUDGET);
        assert_eq!(compaction_cut_index(&history, total), 3);
    }

    #[test]
    fn compaction_never_cuts_last_message() {
        // Единственное гигантское сообщение (текущий вопрос) не компактится.
        let history = vec![msg(120_000)];
        let total: usize = history.iter().map(message_tokens).sum();
        assert_eq!(compaction_cut_index(&history, total), 0);
    }

    #[test]
    fn compaction_cuts_all_but_last_when_tail_is_huge() {
        // Оба сообщения больше KEEP: компактим всё, кроме текущего вопроса.
        let history = vec![msg(60_000), msg(60_000)];
        let total: usize = history.iter().map(message_tokens).sum();
        assert_eq!(compaction_cut_index(&history, total), 1);
    }
}
