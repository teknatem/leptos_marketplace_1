use crate::system::auth::storage;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, HtmlAnchorElement, Request, RequestInit, Response, Url};

pub async fn download_authenticated_file(url: &str, fallback_filename: &str) -> Result<(), String> {
    let access_token =
        storage::get_access_token().ok_or_else(|| "Not authenticated".to_string())?;

    let opts = RequestInit::new();
    opts.set_method("GET");

    let request = Request::new_with_str_and_init(url, &opts)
        .map_err(|e| format!("Failed to build request: {e:?}"))?;
    request
        .headers()
        .set("Authorization", &format!("Bearer {}", access_token))
        .map_err(|e| format!("Failed to set auth header: {e:?}"))?;
    request
        .headers()
        .set("Accept", "application/octet-stream")
        .map_err(|e| format!("Failed to set accept header: {e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "No window object".to_string())?;
    let response_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Request failed: {e:?}"))?;
    let response: Response = response_value
        .dyn_into()
        .map_err(|e| format!("Failed to parse response: {e:?}"))?;

    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }

    let filename = response
        .headers()
        .get("Content-Disposition")
        .ok()
        .flatten()
        .as_deref()
        .and_then(extract_filename)
        .unwrap_or_else(|| fallback_filename.to_string());

    let blob_value = JsFuture::from(
        response
            .blob()
            .map_err(|e| format!("Failed to read response body: {e:?}"))?,
    )
    .await
    .map_err(|e| format!("Failed to resolve blob: {e:?}"))?;
    let blob: Blob = blob_value
        .dyn_into()
        .map_err(|e| format!("Failed to cast blob: {e:?}"))?;

    download_blob(&blob, &filename)
}

fn extract_filename(header: &str) -> Option<String> {
    for part in header.split(';').map(str::trim) {
        if let Some(value) = part.strip_prefix("filename*=UTF-8''") {
            return urlencoding::decode(value)
                .ok()
                .map(|value| value.into_owned());
        }
        if let Some(value) = part.strip_prefix("filename=") {
            let value = value.trim_matches('"').trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn download_blob(blob: &Blob, filename: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or_else(|| "No window object".to_string())?;
    let document = window
        .document()
        .ok_or_else(|| "No document object".to_string())?;
    let body = document
        .body()
        .ok_or_else(|| "No body element".to_string())?;

    let url = Url::create_object_url_with_blob(blob)
        .map_err(|e| format!("Failed to create object URL: {e:?}"))?;

    let anchor = document
        .create_element("a")
        .map_err(|e| format!("Failed to create anchor: {e:?}"))?
        .dyn_into::<HtmlAnchorElement>()
        .map_err(|e| format!("Failed to cast anchor: {e:?}"))?;

    anchor.set_href(&url);
    anchor.set_download(filename);
    let _ = anchor.style().set_property("display", "none");

    body.append_child(&anchor)
        .map_err(|e| format!("Failed to append anchor: {e:?}"))?;
    anchor.click();
    let _ = body.remove_child(&anchor);
    let _ = Url::revoke_object_url(&url);

    Ok(())
}
