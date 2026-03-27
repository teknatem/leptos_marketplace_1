//! LLM Chat Details - Model (API functions)

use super::view_model::FileInfo;
use crate::shared::api_utils::api_base;
use contracts::domain::a018_llm_chat::aggregate::{LlmChatDetail, LlmChatMessage};
use serde::Deserialize;

/// Response from GET /api/a018-llm-chat/jobs/:job_id
#[derive(Debug, Clone, Deserialize)]
pub struct JobStatusResponse {
    pub status: String, // "pending" | "done" | "error"
    pub message: Option<LlmChatMessage>,
    pub error: Option<String>,
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

/// Poll job until done/error or max_attempts reached (each attempt waits interval_ms).
pub async fn poll_until_done(
    job_id: &str,
    max_attempts: u32,
    interval_ms: i32,
) -> Result<LlmChatMessage, String> {
    for _ in 0..max_attempts {
        sleep_ms(interval_ms).await;
        let resp = poll_job(job_id).await?;
        match resp.status.as_str() {
            "done" => {
                return resp
                    .message
                    .ok_or_else(|| "done status but no message".to_string())
            }
            "error" => {
                return Err(resp
                    .error
                    .unwrap_or_else(|| "Unknown LLM error".to_string()))
            }
            _ => {} // "pending" — continue polling
        }
    }
    Err(format!(
        "Timeout: LLM did not respond after {} attempts",
        max_attempts
    ))
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

/// Отправить сообщение. Возвращает job_id для последующего polling.
/// Бэкенд сразу возвращает 202 Accepted и обрабатывает LLM в фоне.
pub async fn send_message(
    chat_id: &str,
    content: &str,
    attachment_ids: Vec<String>,
) -> Result<String, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a018-llm-chat/{}/messages", api_base(), chat_id);
    let dto = serde_json::json!({
        "content": content,
        "attachment_ids": attachment_ids
    });
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
