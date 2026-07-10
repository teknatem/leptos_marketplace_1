//! HTTP client for the plugin subsystem.

use contracts::plugins::{
    PluginBundle, PluginCatalog, PluginDefinition, PluginInvokeRequest, PluginListItem,
    PluginPublishResult, PluginRunBrief, PluginRunContext, PluginSmokeReport, PluginSmokeRequest,
    PluginStats, PluginUpdateStatus, PluginUpsert, PluginValidateReport,
};
use contracts::shared::drilldown::DrilldownResponse;
use gloo_net::http::{Request, RequestBuilder, Response};
use serde::de::DeserializeOwned;
use serde::Serialize;

const API_BASE: &str = "/api/plugin";

async fn send_builder(request: RequestBuilder) -> Result<Response, String> {
    request
        .send()
        .await
        .map_err(|error| format!("Request failed: {error}"))
}

async fn send_request(request: Request) -> Result<Response, String> {
    request
        .send()
        .await
        .map_err(|error| format!("Request failed: {error}"))
}

async fn error_message(resp: Response) -> String {
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    let message = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .and_then(|error| error.as_str())
                .map(str::to_string)
        })
        .filter(|message| !message.trim().is_empty())
        .unwrap_or(body);
    format!("{status} - {message}")
}

async fn json_body<T: DeserializeOwned>(resp: Response) -> Result<T, String> {
    resp.json()
        .await
        .map_err(|error| format!("Failed to parse response: {error}"))
}

async fn expect_json<T: DeserializeOwned>(resp: Response) -> Result<T, String> {
    if !resp.ok() {
        return Err(error_message(resp).await);
    }
    json_body(resp).await
}

async fn expect_ok(resp: Response) -> Result<(), String> {
    if !resp.ok() {
        return Err(error_message(resp).await);
    }
    Ok(())
}

async fn get_json<T: DeserializeOwned>(url: &str) -> Result<T, String> {
    expect_json(send_builder(Request::get(url)).await?).await
}

async fn post_json<T: DeserializeOwned, B: Serialize>(url: &str, body: &B) -> Result<T, String> {
    let request = Request::post(url)
        .json(body)
        .map_err(|error| format!("Failed to serialize: {error}"))?;
    expect_json(send_request(request).await?).await
}

async fn post_empty(url: &str) -> Result<(), String> {
    expect_ok(send_builder(Request::post(url)).await?).await
}

pub async fn upsert(dto: &PluginUpsert) -> Result<String, String> {
    let value: serde_json::Value = post_json(API_BASE, dto).await?;
    Ok(value
        .get("id")
        .and_then(|id| id.as_str())
        .unwrap_or_default()
        .to_string())
}

pub async fn list_enabled() -> Result<Vec<PluginListItem>, String> {
    get_json(API_BASE).await
}

pub async fn list_all() -> Result<Vec<PluginListItem>, String> {
    get_json(&format!("{API_BASE}/all")).await
}

pub async fn list_connections() -> Result<serde_json::Value, String> {
    get_json("/api/connection_mp").await
}

pub async fn list_marketplaces() -> Result<serde_json::Value, String> {
    get_json("/api/marketplace").await
}

pub async fn insert_test_data() -> Result<(), String> {
    post_empty(&format!("{API_BASE}/testdata")).await
}

pub async fn get_by_id(id: &str) -> Result<PluginDefinition, String> {
    get_json(&format!("{API_BASE}/{id}")).await
}

/// Установить/снять оценку плагина (1..5, либо None чтобы снять). POST /:id/rating.
pub async fn set_rating(id: &str, rating: Option<i32>) -> Result<(), String> {
    let _: serde_json::Value = post_json(
        &format!("{API_BASE}/{id}/rating"),
        &serde_json::json!({ "rating": rating }),
    )
    .await?;
    Ok(())
}

pub async fn invoke(id: &str, request: &PluginInvokeRequest) -> Result<serde_json::Value, String> {
    post_json(&format!("{API_BASE}/{id}/invoke"), request).await
}

pub async fn dev_invoke(
    id: &str,
    request: &PluginInvokeRequest,
) -> Result<serde_json::Value, String> {
    post_json(&format!("{API_BASE}/{id}/dev-invoke"), request).await
}

pub async fn validate(bundle: &PluginBundle) -> Result<PluginValidateReport, String> {
    post_json(&format!("{API_BASE}/validate"), bundle).await
}

pub async fn smoke_test(request: &PluginSmokeRequest) -> Result<PluginSmokeReport, String> {
    post_json(&format!("{API_BASE}/smoke-test"), request).await
}

pub async fn invoke_raw(
    id: &str,
    request: &PluginInvokeRequest,
) -> Result<serde_json::Value, String> {
    let request = Request::post(&format!("{API_BASE}/{id}/invoke"))
        .json(request)
        .map_err(|error| format!("Failed to serialize: {error}"))?;
    json_body(send_request(request).await?).await
}

pub async fn get_stats(id: &str, days: i64) -> Result<PluginStats, String> {
    get_json(&format!("{API_BASE}/{id}/stats?days={days}")).await
}

pub async fn runs_summary(days: i64) -> Result<Vec<PluginRunBrief>, String> {
    get_json(&format!("{API_BASE}/runs/summary?days={days}")).await
}

pub async fn import_archive(bytes: Vec<u8>) -> Result<serde_json::Value, String> {
    let array = js_sys::Uint8Array::from(bytes.as_slice());
    let request = Request::post(&format!("{API_BASE}/import"))
        .header("Content-Type", "application/zip")
        .body(array)
        .map_err(|error| format!("Failed to build request: {error}"))?;
    expect_json(send_request(request).await?).await
}

pub async fn run_data(id: &str, ctx: &PluginRunContext) -> Result<DrilldownResponse, String> {
    post_json(&format!("{API_BASE}/{id}/data"), ctx).await
}

/// Опубликовать текущую сохранённую версию плагина в S3. POST /:id/publish.
pub async fn publish(id: &str) -> Result<PluginPublishResult, String> {
    post_json(&format!("{API_BASE}/{id}/publish"), &serde_json::json!({})).await
}

/// Сравнить локальные версии плагинов с каталогом в S3. GET /updates.
pub async fn check_updates() -> Result<Vec<PluginUpdateStatus>, String> {
    get_json(&format!("{API_BASE}/updates")).await
}

/// Скачать и применить опубликованную версию плагина из S3. POST /:id/apply-update.
pub async fn apply_update(id: &str, expected_remote_version: Option<i32>) -> Result<(), String> {
    let _: serde_json::Value = post_json(
        &format!("{API_BASE}/{id}/apply-update"),
        &serde_json::json!({ "expected_remote_version": expected_remote_version }),
    )
    .await?;
    Ok(())
}

/// Весь каталог S3 (code -> последняя опубликованная версия). GET /catalog.
pub async fn get_catalog() -> Result<PluginCatalog, String> {
    get_json(&format!("{API_BASE}/catalog")).await
}

/// Скачать и установить локально плагин из каталога S3. POST /catalog/:code/install.
pub async fn install_from_catalog(code: &str) -> Result<(), String> {
    let _: serde_json::Value = post_json(
        &format!("{API_BASE}/catalog/{code}/install"),
        &serde_json::json!({}),
    )
    .await?;
    Ok(())
}

/// Текущий номер применённой миграции БД (для сверки с `built_for_migration`). GET /migration-version.
pub async fn migration_version() -> Result<i64, String> {
    let value: serde_json::Value = get_json(&format!("{API_BASE}/migration-version")).await?;
    Ok(value.get("version").and_then(|v| v.as_i64()).unwrap_or(0))
}
