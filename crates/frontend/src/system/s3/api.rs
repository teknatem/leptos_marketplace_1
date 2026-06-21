use contracts::system::s3::{S3FileCategory, S3FileListResponse, S3UploadResponse};
use gloo_net::http::Request as GlooRequest;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{FormData, Request, RequestInit, RequestMode, Response};

use crate::shared::api_utils::api_base;
use crate::shared::auth_download::download_authenticated_file;
use crate::system::auth::storage;

fn auth_header() -> Result<String, String> {
    storage::get_access_token()
        .map(|token| format!("Bearer {}", token))
        .ok_or_else(|| "Not authenticated".to_string())
}

pub async fn fetch_files(category: Option<S3FileCategory>) -> Result<S3FileListResponse, String> {
    let mut url = format!("{}/api/sys/s3/files", api_base());
    if let Some(category) = category {
        url.push_str(&format!("?category={}", category.as_str()));
    }

    let response = GlooRequest::get(&url)
        .header("Authorization", &auth_header()?)
        .header("Cache-Control", "no-cache")
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.ok() {
        return Err(format!(
            "Failed to fetch S3 files: HTTP {}",
            response.status()
        ));
    }

    response
        .json::<S3FileListResponse>()
        .await
        .map_err(|e| format!("Failed to parse S3 files: {}", e))
}

pub async fn upload_file(
    category: S3FileCategory,
    file: web_sys::File,
) -> Result<S3UploadResponse, String> {
    let form_data = FormData::new().map_err(|e| format!("{e:?}"))?;
    form_data
        .append_with_str("category", category.as_str())
        .map_err(|e| format!("{e:?}"))?;
    form_data
        .append_with_blob("file", &file)
        .map_err(|e| format!("{e:?}"))?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    opts.set_body(&form_data);

    let request =
        Request::new_with_str_and_init(&format!("{}/api/sys/s3/files", api_base()), &opts)
            .map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Authorization", &auth_header()?)
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "No window object".to_string())?;
    let response_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Upload request failed: {e:?}"))?;
    let response: Response = response_value
        .dyn_into()
        .map_err(|e| format!("Invalid upload response: {e:?}"))?;

    if !response.ok() {
        return Err(format!("Upload failed: HTTP {}", response.status()));
    }

    let text = JsFuture::from(response.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?
        .as_string()
        .ok_or_else(|| "Upload response is not text".to_string())?;
    serde_json::from_str::<S3UploadResponse>(&text)
        .map_err(|e| format!("Failed to parse upload response: {}", e))
}

pub async fn delete_file(id: &str) -> Result<(), String> {
    let response = GlooRequest::delete(&format!("{}/api/sys/s3/files/{}", api_base(), id))
        .header("Authorization", &auth_header()?)
        .send()
        .await
        .map_err(|e| format!("Failed to send delete request: {}", e))?;

    if !response.ok() {
        return Err(format!(
            "Failed to delete S3 file: HTTP {}",
            response.status()
        ));
    }

    Ok(())
}

pub async fn download_file(id: &str, fallback_filename: &str) -> Result<(), String> {
    let url = format!("{}/api/sys/s3/files/{}/download", api_base(), id);
    download_authenticated_file(&url, fallback_filename).await
}
