//! LLM Chat Details - Model (API functions)

use super::view_model::FileInfo;
use crate::shared::api_utils::api_base;
use contracts::domain::a018_llm_chat::aggregate::{LlmChatDetail, LlmChatMessage};
use leptos::prelude::*;
use serde::Deserialize;

/// Текущий этап выполнения фоновой LLM-задачи (для индикатора прогресса).
#[derive(Debug, Clone, Deserialize)]
pub struct JobProgress {
    /// Номер итерации tool-calling (0 = подготовка/финал — без номера в UI).
    pub step: u32,
    /// Человекочитаемая подпись этапа.
    pub stage: String,
    /// Частичный текст ответа модели (стриминг) — рендерится ещё до завершения job'а.
    #[serde(default)]
    pub partial_text: Option<String>,
}

/// Response from GET /api/a018-llm-chat/jobs/:job_id
#[derive(Debug, Clone, Deserialize)]
pub struct JobStatusResponse {
    pub status: String, // "pending" | "done" | "error"
    pub message: Option<LlmChatMessage>,
    pub error: Option<String>,
    #[serde(default)]
    pub progress: Option<JobProgress>,
}

/// Async sleep in WASM using setTimeout.
async fn sleep_ms(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
            .unwrap();
    });
    wasm_bindgen_futures::JsFuture::from(promise).await.ok();
}

/// Чем закончился опрос задачи.
pub enum PollOutcome {
    /// Ассистент ответил — сообщение уже в БД (вызывающий перезагружает ленту сам).
    Done,
    /// Бэкенд сообщил об ошибке выполнения задачи.
    Error(String),
    /// Истёк бюджет ожидания на клиенте. Задача на сервере, скорее всего, ещё
    /// выполняется и допишет ответ в чат — это НЕ ошибка, а «ещё не готово».
    /// `waited_secs` — сколько ждали (для понятного сообщения пользователю).
    StillRunning { waited_secs: u32 },
}

/// Опрашивать задачу, пока она не завершится (done/error) или не выйдет бюджет ожидания.
/// `Err` — только инфраструктурный сбой самого опроса (сеть/HTTP), не статус задачи.
pub async fn poll_until_done(
    job_id: &str,
    max_attempts: u32,
    interval_ms: i32,
    progress: RwSignal<Option<JobProgress>>,
) -> Result<PollOutcome, String> {
    for _ in 0..max_attempts {
        sleep_ms(interval_ms).await;
        let resp = poll_job(job_id).await?;
        match resp.status.as_str() {
            "done" => {
                return if resp.message.is_some() {
                    Ok(PollOutcome::Done)
                } else {
                    Err("done status but no message".to_string())
                }
            }
            "error" => {
                return Ok(PollOutcome::Error(
                    resp.error
                        .unwrap_or_else(|| "Unknown LLM error".to_string()),
                ))
            }
            _ => {
                // "pending" — обновить текущий этап и продолжить опрос
                progress.set(resp.progress);
            }
        }
    }
    let waited_secs = (max_attempts.saturating_mul(interval_ms.max(0) as u32)) / 1000;
    Ok(PollOutcome::StillRunning { waited_secs })
}

/// Get current job status from backend.
async fn poll_job(job_id: &str) -> Result<JobStatusResponse, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a018-llm-chat/jobs/{}", api_base(), job_id);
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;

    if resp.status() == 404 {
        return Err("job not found".to_string());
    }
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    let data: JobStatusResponse = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}

/// Остановить фоновую LLM-задачу. POST /jobs/:job_id/cancel.
pub async fn cancel_job(job_id: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a018-llm-chat/jobs/{}/cancel", api_base(), job_id);
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    Ok(())
}

/// Получить чат по ID (с именем агента)
pub async fn fetch_chat(id: &str) -> Result<LlmChatDetail, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a018-llm-chat/{}", api_base(), id);
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    let data: LlmChatDetail = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;

    Ok(data)
}

/// Получить пакеты контекста, привязанные к чату.
pub async fn fetch_chat_context(
    chat_id: &str,
) -> Result<Vec<contracts::domain::a018_llm_chat::context::ContextPackageSummary>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a018-llm-chat/{}/context", api_base(), chat_id);
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("{e}"))
}

/// Получить сообщения чата
pub async fn fetch_messages(chat_id: &str) -> Result<Vec<LlmChatMessage>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a018-llm-chat/{}/messages", api_base(), chat_id);
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    let data: Vec<LlmChatMessage> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;

    Ok(data)
}

/// Получить полный журнал вызовов инструментов для сообщения ассистента.
pub async fn fetch_tool_trace(
    message_id: &str,
) -> Result<Vec<contracts::domain::a018_llm_chat::aggregate::ToolTraceEntry>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!(
        "{}/api/a018-llm-chat/message/{}/tool-trace",
        api_base(),
        message_id
    );
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("{e}"))
}

/// Отправить сообщение. Возвращает job_id для последующего polling.
/// Бэкенд сразу возвращает 202 Accepted и обрабатывает LLM в фоне.
pub async fn send_message(
    chat_id: &str,
    content: &str,
    attachment_ids: Vec<String>,
    model_name: Option<String>,
) -> Result<String, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a018-llm-chat/{}/messages", api_base(), chat_id);
    let mut dto = serde_json::json!({
        "content": content,
        "attachment_ids": attachment_ids,
        "request_id": format!("ui-{}", js_sys::Date::now() as u64)
    });
    // Переключатель модели в чате: если выбрана модель — прокидываем её на сообщение
    // (бэкенд: request.model_name -> иначе chat.model_name -> connection.model_name).
    if let Some(m) = model_name.filter(|m| !m.trim().is_empty()) {
        dto["model_name"] = serde_json::Value::String(m);
    }
    let body = wasm_bindgen::JsValue::from_str(&dto.to_string());
    opts.set_body(&body);

    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    let parsed: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    let job_id = parsed["job_id"]
        .as_str()
        .ok_or_else(|| "no job_id in response".to_string())?
        .to_string();

    Ok(job_id)
}

/// Загрузить курируемый список моделей (allowed_models) подключения по его id.
/// Пустой список — если подключение недоступно или курирование не задано.
pub async fn fetch_connection_allowed_models(connection_id: &str) -> Result<Vec<String>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a038-llm-connection/{}", api_base(), connection_id);
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    let connection: contracts::domain::a038_llm_connection::aggregate::LlmConnection =
        serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(connection.allowed_models_list())
}

/// Установить/снять оценку чата (1..5, либо None чтобы снять). POST /:id/rating.
pub async fn set_rating(chat_id: &str, rating: Option<i32>) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a018-llm-chat/{}/rating", api_base(), chat_id);
    let dto = serde_json::json!({ "rating": rating });
    let body = wasm_bindgen::JsValue::from_str(&dto.to_string());
    opts.set_body(&body);

    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    Ok(())
}

/// Удалить чат (soft delete). DELETE /:id.
pub async fn delete_chat(chat_id: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("DELETE");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a018-llm-chat/{}", api_base(), chat_id);
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    Ok(())
}

/// Загрузить файл
pub async fn upload_file(chat_id: &str, file: web_sys::File) -> Result<FileInfo, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{FormData, Request, RequestInit, RequestMode, Response};

    let form_data = FormData::new().map_err(|e| format!("{e:?}"))?;
    form_data
        .append_with_blob("file", &file)
        .map_err(|e| format!("{e:?}"))?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    opts.set_body(&form_data);

    let url = format!("{}/api/a018-llm-chat/{}/upload", api_base(), chat_id);
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    let data: FileInfo = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;

    Ok(data)
}
