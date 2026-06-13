//! HTTP-клиент подсистемы Plugins. Относительные URL проксируются на бэкенд;
//! Authorization-заголовок добавляется глобальным fetch-перехватчиком.

use contracts::plugins::{PluginDefinition, PluginListItem, PluginRunContext, PluginUpsert};
use contracts::shared::drilldown::DrilldownResponse;
use gloo_net::http::Request;

const API_BASE: &str = "/api/plugin";

/// Сохранить (создать/обновить) плагин. Возвращает id.
pub async fn upsert(dto: &PluginUpsert) -> Result<String, String> {
    let resp = Request::post(API_BASE)
        .json(dto)
        .map_err(|e| format!("Failed to serialize: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    if !resp.ok() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        let msg = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
            .unwrap_or(body);
        return Err(format!("{} — {}", status, msg));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    Ok(v.get("id")
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string())
}

/// Список включённых плагинов (для меню).
pub async fn list_enabled() -> Result<Vec<PluginListItem>, String> {
    let resp = Request::get(API_BASE)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }
    resp.json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Все плагины (для страницы-реестра) — включая выключенные.
pub async fn list_all() -> Result<Vec<PluginListItem>, String> {
    let url = format!("{}/all", API_BASE);
    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }
    resp.json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Сырой JSON-список (массив) по произвольному GET-эндпоинту — для инъекции
/// справочных данных в client_script (connections, marketplaces …).
async fn get_json_array(url: &str) -> Result<serde_json::Value, String> {
    let resp = Request::get(url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }
    resp.json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Все подключения к маркетплейсам (a006) — для инъекции в client_script.
pub async fn list_connections() -> Result<serde_json::Value, String> {
    get_json_array("/api/connection_mp").await
}

/// Справочник маркетплейсов (a005) — для инъекции в client_script.
pub async fn list_marketplaces() -> Result<serde_json::Value, String> {
    get_json_array("/api/marketplace").await
}

/// Вставить демонстрационный плагин (POST /api/plugin/testdata).
pub async fn insert_test_data() -> Result<(), String> {
    let url = format!("{}/testdata", API_BASE);
    let resp = Request::post(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }
    Ok(())
}

/// Полное определение плагина (бандл + исходники).
pub async fn get_by_id(id: &str) -> Result<PluginDefinition, String> {
    let url = format!("{}/{}", API_BASE, id);
    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }
    resp.json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Исполнить server_script плагина (Rhai на сервере). Возвращает JSON-результат.
pub async fn run_script(
    id: &str,
    ctx: &PluginRunContext,
) -> Result<serde_json::Value, String> {
    let url = format!("{}/{}/run", API_BASE, id);
    let resp = Request::post(&url)
        .json(ctx)
        .map_err(|e| format!("Failed to serialize: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    if !resp.ok() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        let msg = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
            .unwrap_or(body);
        return Err(format!("{} — {}", status, msg));
    }
    resp.json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Выполнить декларативную привязку данных плагина (DataView drilldown).
pub async fn run_data(
    id: &str,
    ctx: &PluginRunContext,
) -> Result<DrilldownResponse, String> {
    let url = format!("{}/{}/data", API_BASE, id);
    let resp = Request::post(&url)
        .json(ctx)
        .map_err(|e| format!("Failed to serialize: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    if !resp.ok() {
        let status = resp.status();
        // Бэкенд возвращает тело { "error": "..." } — показываем его пользователю.
        let body = resp.text().await.unwrap_or_default();
        let msg = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
            .unwrap_or(body);
        return Err(format!("{} — {}", status, msg));
    }
    resp.json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}
