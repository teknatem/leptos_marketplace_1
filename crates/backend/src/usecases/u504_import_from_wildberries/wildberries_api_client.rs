use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};

use super::progress_tracker::ProgressTracker;
use crate::shared::marketplaces::wildberries::datetime::{
    format_wb_cursor_datetime, parse_wb_datetime, wb_day_end_utc, wb_day_start_utc,
};

const WB_ORDERS_MAX_RATE_LIMIT_SLEEP_SECS: u64 = 300;

#[derive(Debug, Clone, Default)]
struct WbRateLimitHeaders {
    retry_seconds: Option<u64>,
    limit: Option<u64>,
    reset_seconds: Option<u64>,
    remaining: Option<u64>,
}

impl WbRateLimitHeaders {
    fn from_headers(headers: &HeaderMap) -> Self {
        fn parse_header(headers: &HeaderMap, name: &str) -> Option<u64> {
            headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.trim().parse::<u64>().ok())
        }

        Self {
            retry_seconds: parse_header(headers, "X-Ratelimit-Retry"),
            limit: parse_header(headers, "X-Ratelimit-Limit"),
            reset_seconds: parse_header(headers, "X-Ratelimit-Reset"),
            remaining: parse_header(headers, "X-Ratelimit-Remaining"),
        }
    }

    fn is_empty(&self) -> bool {
        self.retry_seconds.is_none()
            && self.limit.is_none()
            && self.reset_seconds.is_none()
            && self.remaining.is_none()
    }

    fn to_log_fields(&self) -> String {
        if self.is_empty() {
            return "not provided".to_string();
        }

        format!(
            "retry={}s, reset={}s, limit={}, remaining={}",
            self.retry_seconds
                .map(|v| v.to_string())
                .unwrap_or_else(|| "n/a".to_string()),
            self.reset_seconds
                .map(|v| v.to_string())
                .unwrap_or_else(|| "n/a".to_string()),
            self.limit
                .map(|v| v.to_string())
                .unwrap_or_else(|| "n/a".to_string()),
            self.remaining
                .map(|v| v.to_string())
                .unwrap_or_else(|| "n/a".to_string()),
        )
    }

    fn to_error_suffix(&self) -> String {
        if self.is_empty() {
            String::new()
        } else {
            format!(" | X-Ratelimit: {}", self.to_log_fields())
        }
    }
}

/// HTTP-РєР»РёРµРЅС‚ РґР»СЏ СЂР°Р±РѕС‚С‹ СЃ Wildberries Supplier API
pub struct WildberriesApiClient {
    client: reqwest::Client,
    /// Привязка к сессии импорта: учёт HTTP для `sys_task_runs` / UI «Активные».
    http_track: Arc<Mutex<Option<(Arc<ProgressTracker>, String)>>>,
}

pub struct HttpTrackingGuard {
    http_track: Arc<Mutex<Option<(Arc<ProgressTracker>, String)>>>,
}

impl Drop for HttpTrackingGuard {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.http_track.lock() {
            *guard = None;
        }
    }
}

impl WildberriesApiClient {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60)) // РЈРІРµР»РёС‡РµРЅ С‚Р°Р№РјР°СѓС‚ РґР»СЏ РјРµРґР»РµРЅРЅС‹С… API
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .default_headers(headers)
                .danger_accept_invalid_certs(true) // Р’СЂРµРјРµРЅРЅРѕ РґР»СЏ РѕС‚Р»Р°РґРєРё
                .no_proxy()
                .redirect(reqwest::redirect::Policy::limited(10)) // РЎР»РµРґРѕРІР°С‚СЊ СЂРµРґРёСЂРµРєС‚Р°Рј
                .build()
                .expect("Failed to create HTTP client"),
            http_track: Arc::new(Mutex::new(None)),
        }
    }

    /// Включает учёт трафика для текущей сессии импорта.
    /// Каждый `ImportExecutor` принадлежит ровно одному менеджеру задачи, поэтому
    /// параллельный вызов невозможен — планировщик не запускает одну задачу дважды.
    pub fn bind_http_tracking(
        &self,
        tracker: Arc<ProgressTracker>,
        session_id: String,
    ) -> HttpTrackingGuard {
        if let Ok(mut g) = self.http_track.lock() {
            if g.is_some() {
                tracing::warn!(
                    "bind_http_tracking: overwriting existing tracking for session {}",
                    session_id
                );
            }
            *g = Some((tracker, session_id));
        }
        HttpTrackingGuard {
            http_track: Arc::clone(&self.http_track),
        }
    }

    pub fn clear_http_tracking(&self) {
        if let Ok(mut g) = self.http_track.lock() {
            *g = None;
        }
    }

    /// Читает тело ответа и при активной привязке увеличивает счётчики HTTP в трекере.
    pub(crate) async fn read_body_tracked(
        &self,
        response: reqwest::Response,
    ) -> Result<String, reqwest::Error> {
        self.read_body_tracked_with_request_bytes(response, 0).await
    }

    pub(crate) async fn read_body_tracked_with_request_bytes(
        &self,
        response: reqwest::Response,
        request_body_len: u64,
    ) -> Result<String, reqwest::Error> {
        let text = response.text().await?;
        let response_bytes = text.len() as u64;
        if let Ok(guard) = self.http_track.lock() {
            if let Some((tracker, sid)) = guard.as_ref() {
                tracker.record_http_exchange(sid, request_body_len, response_bytes);
            }
        }
        Ok(text)
    }

    fn record_http_request_attempt(&self, request_body_len: u64) {
        if let Ok(guard) = self.http_track.lock() {
            if let Some((tracker, sid)) = guard.as_ref() {
                tracker.record_http_request_attempt(sid, request_body_len);
            }
        }
    }

    fn record_http_response_body(&self, response_body_len: u64) {
        if let Ok(guard) = self.http_track.lock() {
            if let Some((tracker, sid)) = guard.as_ref() {
                tracker.record_http_response_body(sid, response_body_len);
            }
        }
    }

    fn set_tracked_current_item(&self, aggregate_index: &str, label: impl Into<String>) {
        if let Ok(guard) = self.http_track.lock() {
            if let Some((tracker, sid)) = guard.as_ref() {
                tracker.set_current_item(sid, aggregate_index, Some(label.into()));
            }
        }
    }

    async fn read_body_for_recorded_request(
        &self,
        response: reqwest::Response,
    ) -> Result<String, reqwest::Error> {
        let text = response.text().await?;
        self.record_http_response_body(text.len() as u64);
        Ok(text)
    }

    /// Р”РёР°РіРЅРѕСЃС‚РёС‡РµСЃРєР°СЏ С„СѓРЅРєС†РёСЏ РґР»СЏ С‚РµСЃС‚РёСЂРѕРІР°РЅРёСЏ СЂР°Р·Р»РёС‡РЅС‹С… РІР°СЂРёР°РЅС‚РѕРІ Р·Р°РїСЂРѕСЃР°
    pub async fn diagnostic_fetch_all_variations(
        &self,
        connection: &ConnectionMP,
    ) -> Result<Vec<DiagnosticResult>> {
        let mut results = Vec::new();

        // Р’Р°СЂРёР°РЅС‚ 1: РўРµРєСѓС‰Р°СЏ СЂРµР°Р»РёР·Р°С†РёСЏ (РїСѓСЃС‚РѕР№ С„РёР»СЊС‚СЂ, limit=100)
        results.push(
            self.test_request_variation(
                connection,
                "Current implementation",
                100,
                WildberriesSettings {
                    cursor: WildberriesCursor::default(),
                    filter: WildberriesFilter::default(),
                },
            )
            .await,
        );

        // Р’Р°СЂРёР°РЅС‚ 2: РЈРІРµР»РёС‡РµРЅРЅС‹Р№ limit РґРѕ 1000
        results.push(
            self.test_request_variation(
                connection,
                "Increased limit to 1000",
                1000,
                WildberriesSettings {
                    cursor: WildberriesCursor::default(),
                    filter: WildberriesFilter::default(),
                },
            )
            .await,
        );

        // Р’Р°СЂРёР°РЅС‚ 3: Р‘РµР· settings РІРѕРѕР±С‰Рµ (РјРёРЅРёРјР°Р»СЊРЅС‹Р№ Р·Р°РїСЂРѕСЃ)
        results.push(
            self.test_minimal_request(connection, "Minimal request (no settings)", 1000)
                .await,
        );

        // Р’Р°СЂРёР°РЅС‚ 4: РЎ СЏРІРЅС‹Рј textSearch РїСѓСЃС‚С‹Рј
        results.push(
            self.test_request_variation(
                connection,
                "Empty textSearch filter",
                1000,
                WildberriesSettings {
                    cursor: WildberriesCursor::default(),
                    filter: WildberriesFilter {
                        find_by_nm_id: None,
                    },
                },
            )
            .await,
        );

        // Р’Р°СЂРёР°РЅС‚ 5: РђР»СЊС‚РµСЂРЅР°С‚РёРІРЅС‹Р№ endpoint - Marketplace API
        results.push(
            self.test_alternative_endpoint(
                connection,
                "Alternative: Marketplace API v3",
                "https://marketplace-api.wildberries.ru",
                "/api/v3/goods/list",
            )
            .await,
        );

        // Р’Р°СЂРёР°РЅС‚ 6: РђР»СЊС‚РµСЂРЅР°С‚РёРІРЅС‹Р№ endpoint - Supplier API (stocks)
        results.push(
            self.test_stocks_endpoint(connection, "Alternative: Supplier stocks API")
                .await,
        );

        // Р’Р°СЂРёР°РЅС‚ 7: РљР РРўРР§Р•РЎРљРР™ РўР•РЎРў - РџРѕРїС‹С‚РєР° РїРѕР»СѓС‡РёС‚СЊ С‚РѕРІР°СЂС‹ Р‘Р•Р— С„РёР»СЊС‚СЂР° categories
        // Р’СЃРµ РїСЂРµРґС‹РґСѓС‰РёРµ Р·Р°РїСЂРѕСЃС‹ РІРѕР·РІСЂР°С‰Р°СЋС‚ С‚РѕР»СЊРєРѕ subjectID=7717
        // РџРѕРїСЂРѕР±СѓРµРј Р·Р°РїСЂРѕСЃРёС‚СЊ СЃ СЏРІРЅС‹Рј СѓРєР°Р·Р°РЅРёРµРј С‡С‚Рѕ С…РѕС‚РёРј РІСЃРµ РєР°С‚РµРіРѕСЂРёРё
        results.push(
            self.test_without_category_filter(
                connection,
                "WITHOUT category filter (attempt to get ALL subjects)",
                1000,
            )
            .await,
        );

        // Р’Р°СЂРёР°РЅС‚ 8: РђР РҐРР’РќР«Р• РўРћР’РђР Р« - /content/v2/get/cards/trash
        // РљР РРўРР§РќРћ: Р’РѕР·РјРѕР¶РЅРѕ Р±РѕР»СЊС€РёРЅСЃС‚РІРѕ С‚РѕРІР°СЂРѕРІ РІ РєРѕСЂР·РёРЅРµ/Р°СЂС…РёРІРµ!
        results.push(
            self.test_trash_endpoint(
                connection,
                "TRASH/Archive endpoint - check deleted/archived products",
                1000,
            )
            .await,
        );

        // Р’Р°СЂРёР°РЅС‚ 9: РџРћР›РЈР§РРўР¬ РЎРџРРЎРћРљ Р’РЎР•РҐ РљРђРўР•Р“РћР РР™ РџР РћР”РђР’Р¦Рђ
        // РџСЂРѕРІРµСЂРёС‚СЊ СЃРєРѕР»СЊРєРѕ РєР°С‚РµРіРѕСЂРёР№ (subjects) РёСЃРїРѕР»СЊР·СѓРµС‚СЃСЏ
        results.push(
            self.test_get_all_subjects(connection, "Get ALL subjects/categories used by seller")
                .await,
        );

        // Р’Р°СЂРёР°РЅС‚ 10: РџР РћР”РћР›Р–РРўР¬ РџРђР“РРќРђР¦РР® - РїРѕР»СѓС‡РёС‚СЊ РЎР›Р•Р”РЈР®Р©РЈР® СЃС‚СЂР°РЅРёС†Сѓ
        // Р’РѕР·РјРѕР¶РЅРѕ API РІРѕР·РІСЂР°С‰Р°РµС‚ С‚РѕРІР°СЂС‹ РїРѕ РєР°С‚РµРіРѕСЂРёСЏРј РїРѕСЃС‚СЂР°РЅРёС‡РЅРѕ
        results.push(
            self.test_pagination_continuation(
                connection,
                "Continue pagination to get NEXT page of products",
            )
            .await,
        );

        Ok(results)
    }

    async fn test_request_variation(
        &self,
        connection: &ConnectionMP,
        test_name: &str,
        limit: i32,
        settings: WildberriesSettings,
    ) -> DiagnosticResult {
        self.log_to_file(&format!(
            "\n========== DIAGNOSTIC TEST: {} ==========",
            test_name
        ));

        let base_url = if let Some(ref supplier_id) = connection.supplier_id {
            if supplier_id.starts_with("http") {
                supplier_id.trim_end_matches('/')
            } else {
                "https://content-api.wildberries.ru"
            }
        } else {
            "https://content-api.wildberries.ru"
        };

        let url = format!("{}/content/v2/get/cards/list", base_url);

        let request_body = WildberriesProductListRequest { settings, limit };

        let body = match serde_json::to_string(&request_body) {
            Ok(b) => b,
            Err(e) => {
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("Failed to serialize request: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: None,
                };
            }
        };

        self.log_to_file(&format!("Request body: {}", body));
        let request_body_len = body.len() as u64;

        let response = match self
            .client
            .post(&url)
            .header("Authorization", &connection.api_key)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                self.log_to_file(&format!("Request failed: {}", e));
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("HTTP request failed: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: None,
                };
            }
        };

        let status = response.status();
        let headers = response.headers().clone();
        self.log_to_file(&format!("Response status: {}", status));
        self.log_to_file(&format!("Response headers: {:?}", headers));

        if !status.is_success() {
            let body = self
                .read_body_tracked_with_request_bytes(response, request_body_len)
                .await
                .unwrap_or_default();
            self.log_to_file(&format!("Error response body: {}", body));
            return DiagnosticResult {
                test_name: test_name.to_string(),
                success: false,
                error: Some(format!("API returned status {}: {}", status, body)),
                total_returned: 0,
                cursor_total: 0,
                response_headers: Some(format!("{:?}", headers)),
            };
        }

        let body = match self
            .read_body_tracked_with_request_bytes(response, request_body_len)
            .await
        {
            Ok(b) => b,
            Err(e) => {
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("Failed to read response body: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: Some(format!("{:?}", headers)),
                };
            }
        };

        self.log_to_file(&format!("Response body: {}", body));

        match serde_json::from_str::<WildberriesProductListResponse>(&body) {
            Ok(data) => {
                self.log_to_file(&format!(
                    "вњ“ Success: {} items, cursor.total={}",
                    data.cards.len(),
                    data.cursor.total
                ));
                DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: true,
                    error: None,
                    total_returned: data.cards.len() as i32,
                    cursor_total: data.cursor.total as i32,
                    response_headers: Some(format!("{:?}", headers)),
                }
            }
            Err(e) => {
                self.log_to_file(&format!("Failed to parse response: {}", e));
                DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("Failed to parse JSON: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: Some(format!("{:?}", headers)),
                }
            }
        }
    }

    async fn test_minimal_request(
        &self,
        connection: &ConnectionMP,
        test_name: &str,
        limit: i32,
    ) -> DiagnosticResult {
        self.log_to_file(&format!(
            "\n========== DIAGNOSTIC TEST: {} ==========",
            test_name
        ));

        let base_url = if let Some(ref supplier_id) = connection.supplier_id {
            if supplier_id.starts_with("http") {
                supplier_id.trim_end_matches('/')
            } else {
                "https://content-api.wildberries.ru"
            }
        } else {
            "https://content-api.wildberries.ru"
        };

        let url = format!("{}/content/v2/get/cards/list", base_url);

        // РњРёРЅРёРјР°Р»СЊРЅС‹Р№ Р·Р°РїСЂРѕСЃ - С‚РѕР»СЊРєРѕ limit
        let body = format!(r#"{{"limit":{}}}"#, limit);
        self.log_to_file(&format!("Minimal request body: {}", body));
        let request_body_len = body.len() as u64;

        let response = match self
            .client
            .post(&url)
            .header("Authorization", &connection.api_key)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                self.log_to_file(&format!("Request failed: {}", e));
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("HTTP request failed: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: None,
                };
            }
        };

        let status = response.status();
        let headers = response.headers().clone();
        self.log_to_file(&format!("Response status: {}", status));

        if !status.is_success() {
            let body = self
                .read_body_tracked_with_request_bytes(response, request_body_len)
                .await
                .unwrap_or_default();
            self.log_to_file(&format!("Error response body: {}", body));
            return DiagnosticResult {
                test_name: test_name.to_string(),
                success: false,
                error: Some(format!("API returned status {}: {}", status, body)),
                total_returned: 0,
                cursor_total: 0,
                response_headers: Some(format!("{:?}", headers)),
            };
        }

        let body = match self
            .read_body_tracked_with_request_bytes(response, request_body_len)
            .await
        {
            Ok(b) => b,
            Err(e) => {
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("Failed to read response body: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: Some(format!("{:?}", headers)),
                };
            }
        };

        self.log_to_file(&format!("Response body: {}", body));

        match serde_json::from_str::<WildberriesProductListResponse>(&body) {
            Ok(data) => {
                self.log_to_file(&format!(
                    "вњ“ Success: {} items, cursor.total={}",
                    data.cards.len(),
                    data.cursor.total
                ));
                DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: true,
                    error: None,
                    total_returned: data.cards.len() as i32,
                    cursor_total: data.cursor.total as i32,
                    response_headers: Some(format!("{:?}", headers)),
                }
            }
            Err(e) => {
                self.log_to_file(&format!("Failed to parse response: {}", e));
                DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("Failed to parse JSON: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: Some(format!("{:?}", headers)),
                }
            }
        }
    }

    async fn test_alternative_endpoint(
        &self,
        connection: &ConnectionMP,
        test_name: &str,
        base_url: &str,
        endpoint_path: &str,
    ) -> DiagnosticResult {
        self.log_to_file(&format!(
            "\n========== DIAGNOSTIC TEST: {} ==========",
            test_name
        ));
        self.log_to_file(&format!("Testing endpoint: {}{}", base_url, endpoint_path));

        let url = format!("{}{}", base_url, endpoint_path);

        // РџСЂРѕР±СѓРµРј РїСЂРѕСЃС‚РѕР№ GET Р·Р°РїСЂРѕСЃ
        let response = match self
            .client
            .get(&url)
            .header("Authorization", &connection.api_key)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                self.log_to_file(&format!("Request failed: {}", e));
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("HTTP request failed: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: None,
                };
            }
        };

        let status = response.status();
        let headers = response.headers().clone();
        self.log_to_file(&format!("Response status: {}", status));
        self.log_to_file(&format!("Response headers: {:?}", headers));

        if !status.is_success() {
            let body = self.read_body_tracked(response).await.unwrap_or_default();
            self.log_to_file(&format!("Error response body: {}", body));

            // 404 РёР»Рё 405 РѕР·РЅР°С‡Р°РµС‚ С‡С‚Рѕ endpoint РЅРµ СЃСѓС‰РµСЃС‚РІСѓРµС‚ РёР»Рё РјРµС‚РѕРґ РЅРµ РїРѕРґРґРµСЂР¶РёРІР°РµС‚СЃСЏ
            if status.as_u16() == 404 || status.as_u16() == 405 {
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("Endpoint not available ({})", status)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: Some(format!("{:?}", headers)),
                };
            }

            return DiagnosticResult {
                test_name: test_name.to_string(),
                success: false,
                error: Some(format!("API returned status {}: {}", status, body)),
                total_returned: 0,
                cursor_total: 0,
                response_headers: Some(format!("{:?}", headers)),
            };
        }

        let body = match self.read_body_tracked(response).await {
            Ok(b) => b,
            Err(e) => {
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("Failed to read response body: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: Some(format!("{:?}", headers)),
                };
            }
        };

        self.log_to_file(&format!(
            "Response body (first 500 chars): {}",
            body.chars().take(500).collect::<String>()
        ));

        // РџСЂРѕР±СѓРµРј СЂР°СЃРїР°СЂСЃРёС‚СЊ РєР°Рє РЅР°С€ СЃС‚Р°РЅРґР°СЂС‚РЅС‹Р№ РѕС‚РІРµС‚
        match serde_json::from_str::<WildberriesProductListResponse>(&body) {
            Ok(data) => {
                self.log_to_file(&format!(
                    "вњ“ Success (parseable as standard response): {} items, cursor.total={}",
                    data.cards.len(),
                    data.cursor.total
                ));
                DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: true,
                    error: None,
                    total_returned: data.cards.len() as i32,
                    cursor_total: data.cursor.total as i32,
                    response_headers: Some(format!("{:?}", headers)),
                }
            }
            Err(_) => {
                // РќРµ РїР°СЂСЃРёС‚СЃСЏ РєР°Рє СЃС‚Р°РЅРґР°СЂС‚РЅС‹Р№ РѕС‚РІРµС‚, РЅРѕ Р·Р°РїСЂРѕСЃ СѓСЃРїРµС€РЅС‹Р№
                self.log_to_file("Response structure is different from standard format");
                DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(
                        "Response has different structure (not standard cards format)".to_string(),
                    ),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: Some(format!("{:?}", headers)),
                }
            }
        }
    }

    async fn test_stocks_endpoint(
        &self,
        connection: &ConnectionMP,
        test_name: &str,
    ) -> DiagnosticResult {
        self.log_to_file(&format!(
            "\n========== DIAGNOSTIC TEST: {} ==========",
            test_name
        ));

        // Supplier stocks API endpoint
        let url = "https://suppliers-api.wildberries.ru/api/v1/supplier/stocks";
        self.log_to_file(&format!("Testing endpoint: {}", url));

        let response = match self
            .client
            .get(url)
            .header("Authorization", &connection.api_key)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                self.log_to_file(&format!("Request failed: {}", e));
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("HTTP request failed: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: None,
                };
            }
        };

        let status = response.status();
        let headers = response.headers().clone();
        self.log_to_file(&format!("Response status: {}", status));
        self.log_to_file(&format!("Response headers: {:?}", headers));

        if !status.is_success() {
            let body = self.read_body_tracked(response).await.unwrap_or_default();
            self.log_to_file(&format!("Error response body: {}", body));

            if status.as_u16() == 404 {
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some("Stocks endpoint not available".to_string()),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: Some(format!("{:?}", headers)),
                };
            }

            return DiagnosticResult {
                test_name: test_name.to_string(),
                success: false,
                error: Some(format!("API returned status {}: {}", status, body)),
                total_returned: 0,
                cursor_total: 0,
                response_headers: Some(format!("{:?}", headers)),
            };
        }

        let body = match self.read_body_tracked(response).await {
            Ok(b) => b,
            Err(e) => {
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("Failed to read response body: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: Some(format!("{:?}", headers)),
                };
            }
        };

        self.log_to_file(&format!(
            "Response body (first 500 chars): {}",
            body.chars().take(500).collect::<String>()
        ));

        // Stocks API РІРѕР·РІСЂР°С‰Р°РµС‚ РјР°СЃСЃРёРІ СЃ РґСЂСѓРіРѕР№ СЃС‚СЂСѓРєС‚СѓСЂРѕР№
        // РџСЂРѕР±СѓРµРј СЂР°СЃРїР°СЂСЃРёС‚СЊ Рё РїРѕСЃС‡РёС‚Р°С‚СЊ РєРѕР»РёС‡РµСЃС‚РІРѕ С‚РѕРІР°СЂРѕРІ
        match serde_json::from_str::<serde_json::Value>(&body) {
            Ok(json) => {
                if let Some(stocks) = json.as_array() {
                    let count = stocks.len();
                    self.log_to_file(&format!("вњ“ Success: Stocks API returned {} items", count));
                    DiagnosticResult {
                        test_name: test_name.to_string(),
                        success: true,
                        error: None,
                        total_returned: count as i32,
                        cursor_total: count as i32, // Stocks API РЅРµ РёРјРµРµС‚ cursor.total
                        response_headers: Some(format!("{:?}", headers)),
                    }
                } else {
                    self.log_to_file("Response is not an array");
                    DiagnosticResult {
                        test_name: test_name.to_string(),
                        success: false,
                        error: Some("Stocks response is not an array".to_string()),
                        total_returned: 0,
                        cursor_total: 0,
                        response_headers: Some(format!("{:?}", headers)),
                    }
                }
            }
            Err(e) => {
                self.log_to_file(&format!("Failed to parse stocks response: {}", e));
                DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("Failed to parse JSON: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: Some(format!("{:?}", headers)),
                }
            }
        }
    }

    async fn test_get_all_subjects(
        &self,
        connection: &ConnectionMP,
        test_name: &str,
    ) -> DiagnosticResult {
        self.log_to_file(&format!(
            "\n========== DIAGNOSTIC TEST: {} ==========",
            test_name
        ));
        self.log_to_file("рџ“Љ Getting list of ALL subjects/categories from seller account");
        self.log_to_file("This will show how many categories are used");

        let base_url = if let Some(ref supplier_id) = connection.supplier_id {
            if supplier_id.starts_with("http") {
                supplier_id.trim_end_matches('/')
            } else {
                "https://content-api.wildberries.ru"
            }
        } else {
            "https://content-api.wildberries.ru"
        };

        // Endpoint РґР»СЏ РїРѕР»СѓС‡РµРЅРёСЏ СЃРїРёСЃРєР° subjects
        let url = format!("{}/content/v2/object/all?limit=1000", base_url);
        self.log_to_file(&format!("GET request to: {}", url));

        let response = match self
            .client
            .get(&url)
            .header("Authorization", &connection.api_key)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                self.log_to_file(&format!("Request failed: {}", e));
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("HTTP request failed: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: None,
                };
            }
        };

        let status = response.status();
        let headers = response.headers().clone();
        self.log_to_file(&format!("Response status: {}", status));

        if !status.is_success() {
            let body = self.read_body_tracked(response).await.unwrap_or_default();
            self.log_to_file(&format!("Error response body: {}", body));
            return DiagnosticResult {
                test_name: test_name.to_string(),
                success: false,
                error: Some(format!("API returned status {}: {}", status, body)),
                total_returned: 0,
                cursor_total: 0,
                response_headers: Some(format!("{:?}", headers)),
            };
        }

        let body = match self.read_body_tracked(response).await {
            Ok(b) => b,
            Err(e) => {
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("Failed to read response body: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: Some(format!("{:?}", headers)),
                };
            }
        };

        self.log_to_file(&format!(
            "Response body preview: {}",
            body.chars().take(1000).collect::<String>()
        ));

        // РџРѕРїСЂРѕР±СѓРµРј СЂР°СЃРїР°СЂСЃРёС‚СЊ РєР°Рє JSON
        match serde_json::from_str::<serde_json::Value>(&body) {
            Ok(json) => {
                if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                    self.log_to_file(&format!(
                        "вњ“ Found {} subjects/categories available to this seller!",
                        data.len()
                    ));

                    // РќР°Р№РґРµРј СѓРЅРёРєР°Р»СЊРЅС‹Рµ subjectID
                    let mut subject_ids = Vec::new();
                    for item in data.iter().take(20) {
                        if let Some(id) = item.get("subjectID").and_then(|i| i.as_i64()) {
                            if let Some(name) = item.get("subjectName").and_then(|n| n.as_str()) {
                                self.log_to_file(&format!("  - SubjectID {}: {}", id, name));
                                subject_ids.push(id);
                            }
                        }
                    }
                    if data.len() > 20 {
                        self.log_to_file(&format!("  ... and {} more", data.len() - 20));
                    }

                    if subject_ids.contains(&7717) {
                        self.log_to_file("вњ“ SubjectID 7717 is in the list!");
                    }

                    if data.len() > 1 {
                        self.log_to_file(&format!(
                            "рџ”Ґ IMPORTANT: Seller has {} categories, but API returns only from ONE (7717)!",
                            data.len()
                        ));
                        self.log_to_file(
                            "This confirms: either need to query each category separately,",
                        );
                        self.log_to_file(
                            "OR continue pagination to get products from other categories.",
                        );
                    }

                    DiagnosticResult {
                        test_name: test_name.to_string(),
                        success: true,
                        error: None,
                        total_returned: data.len() as i32,
                        cursor_total: data.len() as i32,
                        response_headers: Some(format!("{:?}", headers)),
                    }
                } else {
                    self.log_to_file("Failed to find 'data' array in response");
                    DiagnosticResult {
                        test_name: test_name.to_string(),
                        success: false,
                        error: Some("No 'data' array in response".to_string()),
                        total_returned: 0,
                        cursor_total: 0,
                        response_headers: Some(format!("{:?}", headers)),
                    }
                }
            }
            Err(e) => {
                self.log_to_file(&format!("Failed to parse response: {}", e));
                DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("Failed to parse JSON: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: Some(format!("{:?}", headers)),
                }
            }
        }
    }

    async fn test_pagination_continuation(
        &self,
        connection: &ConnectionMP,
        test_name: &str,
    ) -> DiagnosticResult {
        self.log_to_file(&format!(
            "\n========== DIAGNOSTIC TEST: {} ==========",
            test_name
        ));
        self.log_to_file("рџ”„ Testing pagination: Continue from FIRST page cursor");
        self.log_to_file("Hypothesis: API returns products by categories page-by-page");

        let base_url = if let Some(ref supplier_id) = connection.supplier_id {
            if supplier_id.starts_with("http") {
                supplier_id.trim_end_matches('/')
            } else {
                "https://content-api.wildberries.ru"
            }
        } else {
            "https://content-api.wildberries.ru"
        };

        let url = format!("{}/content/v2/get/cards/list", base_url);

        // РЎРЅР°С‡Р°Р»Р° РїРѕР»СѓС‡РёРј РїРµСЂРІСѓСЋ СЃС‚СЂР°РЅРёС†Сѓ РґР»СЏ РёР·РІР»РµС‡РµРЅРёСЏ cursor
        self.log_to_file("Step 1: Get FIRST page to extract cursor...");

        let first_request = WildberriesProductListRequest {
            settings: WildberriesSettings {
                cursor: WildberriesCursor::default(),
                filter: WildberriesFilter::default(),
            },
            limit: 100,
        };

        let body1 = match serde_json::to_string(&first_request) {
            Ok(b) => b,
            Err(e) => {
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("Failed to serialize request: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: None,
                };
            }
        };
        let request_body1_len = body1.len() as u64;

        let response1 = match self
            .client
            .post(&url)
            .header("Authorization", &connection.api_key)
            .header("Content-Type", "application/json")
            .body(body1)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                self.log_to_file(&format!("First request failed: {}", e));
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("HTTP request failed: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: None,
                };
            }
        };

        let body1_text = match self
            .read_body_tracked_with_request_bytes(response1, request_body1_len)
            .await
        {
            Ok(b) => b,
            Err(e) => {
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("Failed to read response body: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: None,
                };
            }
        };

        let first_page: WildberriesProductListResponse = match serde_json::from_str(&body1_text) {
            Ok(data) => data,
            Err(e) => {
                self.log_to_file(&format!("Failed to parse first page: {}", e));
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("Failed to parse first page: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: None,
                };
            }
        };

        self.log_to_file(&format!(
            "First page: {} items, cursor.total={}, cursor.updatedAt={:?}, cursor.nmID={:?}",
            first_page.cards.len(),
            first_page.cursor.total,
            first_page.cursor.updated_at,
            first_page.cursor.nm_id
        ));

        // РўРµРїРµСЂСЊ Р·Р°РїСЂРѕСЃРёРј Р’РўРћР РЈР® СЃС‚СЂР°РЅРёС†Сѓ РёСЃРїРѕР»СЊР·СѓСЏ cursor РёР· РїРµСЂРІРѕР№
        self.log_to_file("Step 2: Get SECOND page using cursor from first page...");

        let second_request = WildberriesProductListRequest {
            settings: WildberriesSettings {
                cursor: first_page.cursor.clone(),
                filter: WildberriesFilter::default(),
            },
            limit: 100,
        };

        let body2 = match serde_json::to_string(&second_request) {
            Ok(b) => b,
            Err(e) => {
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("Failed to serialize second request: {}", e)),
                    total_returned: first_page.cards.len() as i32,
                    cursor_total: first_page.cursor.total as i32,
                    response_headers: None,
                };
            }
        };

        self.log_to_file(&format!("Second request body: {}", body2));
        let request_body2_len = body2.len() as u64;

        let response2 = match self
            .client
            .post(&url)
            .header("Authorization", &connection.api_key)
            .header("Content-Type", "application/json")
            .body(body2)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                self.log_to_file(&format!("Second request failed: {}", e));
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("Second request failed: {}", e)),
                    total_returned: first_page.cards.len() as i32,
                    cursor_total: first_page.cursor.total as i32,
                    response_headers: None,
                };
            }
        };

        let status2 = response2.status();
        let headers2 = response2.headers().clone();
        self.log_to_file(&format!("Second response status: {}", status2));

        if !status2.is_success() {
            let body = self
                .read_body_tracked_with_request_bytes(response2, request_body2_len)
                .await
                .unwrap_or_default();
            self.log_to_file(&format!("Error response body: {}", body));
            return DiagnosticResult {
                test_name: test_name.to_string(),
                success: false,
                error: Some(format!(
                    "Second request returned status {}: {}",
                    status2, body
                )),
                total_returned: first_page.cards.len() as i32,
                cursor_total: first_page.cursor.total as i32,
                response_headers: Some(format!("{:?}", headers2)),
            };
        }

        let body2_text = match self
            .read_body_tracked_with_request_bytes(response2, request_body2_len)
            .await
        {
            Ok(b) => b,
            Err(e) => {
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("Failed to read second response: {}", e)),
                    total_returned: first_page.cards.len() as i32,
                    cursor_total: first_page.cursor.total as i32,
                    response_headers: Some(format!("{:?}", headers2)),
                };
            }
        };

        match serde_json::from_str::<WildberriesProductListResponse>(&body2_text) {
            Ok(second_page) => {
                self.log_to_file(&format!(
                    "вњ“ Second page: {} items, cursor.total={}",
                    second_page.cards.len(),
                    second_page.cursor.total
                ));

                // РџСЂРѕРІРµСЂРёРј subjectID РЅР° РІС‚РѕСЂРѕР№ СЃС‚СЂР°РЅРёС†Рµ
                let mut unique_subjects = std::collections::HashSet::new();
                for card in &second_page.cards {
                    unique_subjects.insert(card.subject_id);
                }

                self.log_to_file(&format!(
                    "Second page has {} unique subjectIDs: {:?}",
                    unique_subjects.len(),
                    unique_subjects
                ));

                if second_page.cards.is_empty() {
                    self.log_to_file(
                        "вљ пёЏ Second page is EMPTY! All products were on first page.",
                    );
                    self.log_to_file("This means cursor.total matches actual product count.");
                } else if unique_subjects.len() > 1 || !unique_subjects.contains(&7717) {
                    self.log_to_file("рџ”Ґ JACKPOT! Second page has DIFFERENT categories!");
                    self.log_to_file("Solution: Need to continue pagination to get ALL products!");
                } else if unique_subjects.contains(&7717) {
                    self.log_to_file("Still subjectID=7717. Need to continue further...");
                }

                DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: true,
                    error: None,
                    total_returned: second_page.cards.len() as i32,
                    cursor_total: second_page.cursor.total as i32,
                    response_headers: Some(format!("{:?}", headers2)),
                }
            }
            Err(e) => {
                self.log_to_file(&format!("Failed to parse second page: {}", e));
                DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("Failed to parse second page: {}", e)),
                    total_returned: first_page.cards.len() as i32,
                    cursor_total: first_page.cursor.total as i32,
                    response_headers: Some(format!("{:?}", headers2)),
                }
            }
        }
    }

    async fn test_trash_endpoint(
        &self,
        connection: &ConnectionMP,
        test_name: &str,
        limit: i32,
    ) -> DiagnosticResult {
        self.log_to_file(&format!(
            "\n========== DIAGNOSTIC TEST: {} ==========",
            test_name
        ));
        self.log_to_file("рџ—‘пёЏ CRITICAL: Checking TRASH/ARCHIVE endpoint");
        self.log_to_file("Maybe most products are ARCHIVED/DELETED?");

        let base_url = if let Some(ref supplier_id) = connection.supplier_id {
            if supplier_id.starts_with("http") {
                supplier_id.trim_end_matches('/')
            } else {
                "https://content-api.wildberries.ru"
            }
        } else {
            "https://content-api.wildberries.ru"
        };

        // TRASH endpoint!
        let url = format!("{}/content/v2/get/cards/trash", base_url);
        self.log_to_file(&format!("Using TRASH endpoint: {}", url));

        let request_body = WildberriesProductListRequest {
            settings: WildberriesSettings {
                cursor: WildberriesCursor::default(),
                filter: WildberriesFilter::default(),
            },
            limit,
        };

        let body = match serde_json::to_string(&request_body) {
            Ok(b) => b,
            Err(e) => {
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("Failed to serialize request: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: None,
                };
            }
        };

        self.log_to_file(&format!("Request body: {}", body));
        let request_body_len = body.len() as u64;

        let response = match self
            .client
            .post(&url)
            .header("Authorization", &connection.api_key)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                self.log_to_file(&format!("Request failed: {}", e));
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("HTTP request failed: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: None,
                };
            }
        };

        let status = response.status();
        let headers = response.headers().clone();
        self.log_to_file(&format!("Response status: {}", status));
        self.log_to_file(&format!("Response headers: {:?}", headers));

        if !status.is_success() {
            let body = self
                .read_body_tracked_with_request_bytes(response, request_body_len)
                .await
                .unwrap_or_default();
            self.log_to_file(&format!("Error response body: {}", body));
            return DiagnosticResult {
                test_name: test_name.to_string(),
                success: false,
                error: Some(format!("API returned status {}: {}", status, body)),
                total_returned: 0,
                cursor_total: 0,
                response_headers: Some(format!("{:?}", headers)),
            };
        }

        let body = match self
            .read_body_tracked_with_request_bytes(response, request_body_len)
            .await
        {
            Ok(b) => b,
            Err(e) => {
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("Failed to read response body: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: Some(format!("{:?}", headers)),
                };
            }
        };

        self.log_to_file(&format!(
            "Response body preview: {}",
            body.chars().take(500).collect::<String>()
        ));

        match serde_json::from_str::<WildberriesProductListResponse>(&body) {
            Ok(data) => {
                self.log_to_file(&format!(
                    "вњ“ Success: {} items in TRASH, cursor.total={}",
                    data.cards.len(),
                    data.cursor.total
                ));

                if data.cursor.total > 100 {
                    self.log_to_file(&format!(
                        "рџ”Ґ JACKPOT! Found {} archived products! This might be the missing products!",
                        data.cursor.total
                    ));
                } else {
                    self.log_to_file("Not many archived products found.");
                }

                // РџСЂРѕРІРµСЂСЏРµРј СѓРЅРёРєР°Р»СЊРЅС‹Рµ subjectID РІ Р°СЂС…РёРІРµ
                let mut unique_subjects = std::collections::HashSet::new();
                for card in &data.cards {
                    unique_subjects.insert(card.subject_id);
                }
                self.log_to_file(&format!(
                    "Archived products have {} unique subjectIDs: {:?}",
                    unique_subjects.len(),
                    unique_subjects
                ));

                DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: true,
                    error: None,
                    total_returned: data.cards.len() as i32,
                    cursor_total: data.cursor.total as i32,
                    response_headers: Some(format!("{:?}", headers)),
                }
            }
            Err(e) => {
                self.log_to_file(&format!("Failed to parse response: {}", e));
                DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("Failed to parse JSON: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: Some(format!("{:?}", headers)),
                }
            }
        }
    }

    async fn test_without_category_filter(
        &self,
        connection: &ConnectionMP,
        test_name: &str,
        limit: i32,
    ) -> DiagnosticResult {
        self.log_to_file(&format!(
            "\n========== DIAGNOSTIC TEST: {} ==========",
            test_name
        ));
        self.log_to_file("CRITICAL: Testing if API filters by subjectID/category");
        self.log_to_file("Previous requests returned ONLY subjectID=7717");
        self.log_to_file("Trying to request ALL categories at once");

        let base_url = if let Some(ref supplier_id) = connection.supplier_id {
            if supplier_id.starts_with("http") {
                supplier_id.trim_end_matches('/')
            } else {
                "https://content-api.wildberries.ru"
            }
        } else {
            "https://content-api.wildberries.ru"
        };

        let url = format!("{}/content/v2/get/cards/list", base_url);

        // РџРѕРїСЂРѕР±СѓРµРј РЎРћР’РЎР•Рњ РјРёРЅРёРјР°Р»СЊРЅС‹Р№ Р·Р°РїСЂРѕСЃ - Р±РµР· cursor РІРѕРѕР±С‰Рµ
        let body = format!(r#"{{"limit":{}}}"#, limit);
        self.log_to_file(&format!("Minimal request (no cursor at all): {}", body));
        let request_body_len = body.len() as u64;

        let response = match self
            .client
            .post(&url)
            .header("Authorization", &connection.api_key)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                self.log_to_file(&format!("Request failed: {}", e));
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("HTTP request failed: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: None,
                };
            }
        };

        let status = response.status();
        let headers = response.headers().clone();
        self.log_to_file(&format!("Response status: {}", status));

        if !status.is_success() {
            let body = self
                .read_body_tracked_with_request_bytes(response, request_body_len)
                .await
                .unwrap_or_default();
            self.log_to_file(&format!("Error response body: {}", body));
            return DiagnosticResult {
                test_name: test_name.to_string(),
                success: false,
                error: Some(format!("API returned status {}: {}", status, body)),
                total_returned: 0,
                cursor_total: 0,
                response_headers: Some(format!("{:?}", headers)),
            };
        }

        let body = match self
            .read_body_tracked_with_request_bytes(response, request_body_len)
            .await
        {
            Ok(b) => b,
            Err(e) => {
                return DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("Failed to read response body: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: Some(format!("{:?}", headers)),
                };
            }
        };

        self.log_to_file(&format!("Response body: {}", body));

        match serde_json::from_str::<WildberriesProductListResponse>(&body) {
            Ok(data) => {
                // РџСЂРѕРІРµСЂСЏРµРј СѓРЅРёРєР°Р»СЊРЅС‹Рµ subjectID
                let mut unique_subjects = std::collections::HashSet::new();
                for card in &data.cards {
                    unique_subjects.insert(card.subject_id);
                }

                self.log_to_file(&format!(
                    "вњ“ Success: {} items, cursor.total={}",
                    data.cards.len(),
                    data.cursor.total
                ));
                self.log_to_file(&format!(
                    "IMPORTANT: Found {} unique subjectIDs: {:?}",
                    unique_subjects.len(),
                    unique_subjects
                ));

                if unique_subjects.len() == 1 {
                    self.log_to_file(
                        "вљ пёЏ WARNING: Still only ONE subjectID! API might be filtering by category.",
                    );
                } else {
                    self.log_to_file(&format!(
                        "вњ“ GOOD: Multiple subjectIDs found! This approach might work."
                    ));
                }

                DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: true,
                    error: None,
                    total_returned: data.cards.len() as i32,
                    cursor_total: data.cursor.total as i32,
                    response_headers: Some(format!("{:?}", headers)),
                }
            }
            Err(e) => {
                self.log_to_file(&format!("Failed to parse response: {}", e));
                DiagnosticResult {
                    test_name: test_name.to_string(),
                    success: false,
                    error: Some(format!("Failed to parse JSON: {}", e)),
                    total_returned: 0,
                    cursor_total: 0,
                    response_headers: Some(format!("{:?}", headers)),
                }
            }
        }
    }

    /// Р—Р°РїРёСЃР°С‚СЊ РІ Р»РѕРі-С„Р°Р№Р»
    fn log_to_file(&self, message: &str) {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("wildberries_api_requests.log")
        {
            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let _ = writeln!(file, "[{}] {}", timestamp, message);
        }
    }

    /// РџРѕР»СѓС‡РёС‚СЊ СЃРїРёСЃРѕРє С‚РѕРІР°СЂРѕРІ С‡РµСЂРµР· POST /content/v2/get/cards/list
    pub async fn fetch_product_list(
        &self,
        connection: &ConnectionMP,
        limit: i32,
        cursor: Option<WildberriesCursor>,
    ) -> Result<WildberriesProductListResponse> {
        // РСЃРїРѕР»СЊР·СѓРµРј URL РёР· РЅР°СЃС‚СЂРѕРµРє РїРѕРґРєР»СЋС‡РµРЅРёСЏ, РµСЃР»Рё Р·Р°РґР°РЅ, РёРЅР°С‡Рµ default
        let base_url = if let Some(ref supplier_id) = connection.supplier_id {
            if supplier_id.starts_with("http") {
                // Р•СЃР»Рё supplier_id СЃРѕРґРµСЂР¶РёС‚ РїРѕР»РЅС‹Р№ URL, РёСЃРїРѕР»СЊР·СѓРµРј РµРіРѕ РєР°Рє base URL
                supplier_id.trim_end_matches('/')
            } else {
                "https://content-api.wildberries.ru"
            }
        } else {
            "https://content-api.wildberries.ru"
        };

        let url = format!("{}/content/v2/get/cards/list", base_url);

        if connection.api_key.trim().is_empty() {
            anyhow::bail!("API Key is required for Wildberries API");
        }

        self.log_to_file(&format!("Using API URL: {}", url));

        // Wildberries API РёСЃРїРѕР»СЊР·СѓРµС‚ РєСѓСЂСЃРѕСЂРЅСѓСЋ РїР°РіРёРЅР°С†РёСЋ
        let request_body = WildberriesProductListRequest {
            settings: WildberriesSettings {
                cursor: cursor.unwrap_or_default(),
                filter: WildberriesFilter {
                    find_by_nm_id: None,
                },
            },
            limit,
        };

        let body = serde_json::to_string(&request_body)?;
        self.log_to_file(&format!(
            "=== REQUEST ===\nPOST {}\nAuthorization: ****\nBody: {}",
            url, body
        ));
        let request_body_len = body.len() as u64;

        let response = match self
            .client
            .post(&url)
            .header("Authorization", &connection.api_key)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error_msg = format!("HTTP request failed: {:?}", e);
                self.log_to_file(&error_msg);
                tracing::error!("Wildberries API connection error: {}", e);

                // РџСЂРѕРІРµСЂСЏРµРј РєРѕРЅРєСЂРµС‚РЅС‹Рµ С‚РёРїС‹ РѕС€РёР±РѕРє
                if e.is_timeout() {
                    anyhow::bail!("Request timeout: API РЅРµ РѕС‚РІРµС‚РёР» РІ С‚РµС‡РµРЅРёРµ 60 СЃРµРєСѓРЅРґ");
                } else if e.is_connect() {
                    anyhow::bail!("Connection error: РЅРµ СѓРґР°Р»РѕСЃСЊ РїРѕРґРєР»СЋС‡РёС‚СЊСЃСЏ Рє СЃРµСЂРІРµСЂСѓ WB. РџСЂРѕРІРµСЂСЊС‚Рµ РёРЅС‚РµСЂРЅРµС‚-СЃРѕРµРґРёРЅРµРЅРёРµ.");
                } else if e.is_request() {
                    anyhow::bail!("Request error: РїСЂРѕР±Р»РµРјР° РїСЂРё РѕС‚РїСЂР°РІРєРµ Р·Р°РїСЂРѕСЃР° - {}", e);
                } else {
                    anyhow::bail!("Unknown error: {}", e);
                }
            }
        };

        let status = response.status();
        self.log_to_file(&format!("Response status: {}", status));

        if !status.is_success() {
            let body = self
                .read_body_tracked_with_request_bytes(response, request_body_len)
                .await
                .unwrap_or_default();
            self.log_to_file(&format!("ERROR Response body:\n{}", body));
            tracing::error!("Wildberries API request failed: {}", body);
            anyhow::bail!(
                "Wildberries API request failed with status {}: {}",
                status,
                body
            );
        }

        let body = self
            .read_body_tracked_with_request_bytes(response, request_body_len)
            .await?;
        self.log_to_file(&format!("=== RESPONSE BODY ===\n{}\n", body));

        let preview: String = body.chars().take(500).collect::<String>();
        let preview = if preview.len() < body.len() {
            format!("{}...", preview)
        } else {
            preview
        };
        tracing::debug!("Wildberries API response preview: {}", preview);

        match serde_json::from_str::<WildberriesProductListResponse>(&body) {
            Ok(data) => {
                let cursor_str = data
                    .cursor
                    .updated_at
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("none");

                self.log_to_file(&format!(
                    "=== PARSED RESPONSE ===\nItems: {}\nCursor.total: {}\nCursor.updatedAt: {}\nCursor.nmID: {:?}",
                    data.cards.len(),
                    data.cursor.total,
                    cursor_str,
                    data.cursor.nm_id
                ));

                if data.cards.is_empty() {
                    self.log_to_file("вљ  WARNING: Empty cards array - no more products!");
                } else {
                    let first_nm_id = data.cards.first().map(|c| c.nm_id);
                    let last_nm_id = data.cards.last().map(|c| c.nm_id);
                    self.log_to_file(&format!(
                        "Product range: first nmID={:?}, last nmID={:?}",
                        first_nm_id, last_nm_id
                    ));
                }

                tracing::info!(
                    "Wildberries API response: {} items, total: {}, cursor: updatedAt={}, nmID={:?}",
                    data.cards.len(),
                    data.cursor.total,
                    cursor_str,
                    data.cursor.nm_id
                );
                Ok(data)
            }
            Err(e) => {
                let error_msg = format!("Failed to parse Wildberries API JSON: {}", e);
                self.log_to_file(&error_msg);
                tracing::error!("Failed to parse Wildberries API response. Error: {}", e);
                tracing::error!("Response body: {}", body);
                anyhow::bail!(
                    "Failed to parse Wildberries API JSON: {}. Response: {}",
                    e,
                    preview
                )
            }
        }
    }

    /// РџРѕР»СѓС‡РёС‚СЊ РґР°РЅРЅС‹Рµ РїРѕ РїСЂРѕРґР°Р¶Р°Рј С‡РµСЂРµР· Statistics API
    /// GET /api/v1/supplier/sales
    /// Р’РђР–РќРћ: Р—Р°РіСЂСѓР¶Р°РµС‚ Р’РЎР• Р·Р°РїРёСЃРё СЃ СѓС‡РµС‚РѕРј РїР°РіРёРЅР°С†РёРё API
    pub async fn fetch_sales(
        &self,
        connection: &ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<Vec<(WbSaleRow, String)>> {
        let url = "https://statistics-api.wildberries.ru/api/v1/supplier/sales";

        if connection.api_key.trim().is_empty() {
            anyhow::bail!("API Key is required for Wildberries API");
        }

        let date_from_str = date_from.format("%Y-%m-%d").to_string();
        let date_to_str = date_to.format("%Y-%m-%d").to_string();

        // API Wildberries Statistics РјРѕР¶РµС‚ РІРѕР·РІСЂР°С‰Р°С‚СЊ РґРѕ 100,000 Р·Р°РїРёСЃРµР№ Р·Р° Р·Р°РїСЂРѕСЃ,
        // РЅРѕ СЂРµРєРѕРјРµРЅРґСѓРµС‚СЃСЏ РґРµР»Р°С‚СЊ Р·Р°РїСЂРѕСЃС‹ СЃ С„Р»Р°РіРѕРј page РґР»СЏ РїР°РіРёРЅР°С†РёРё
        // РЎРѕРіР»Р°СЃРЅРѕ РґРѕРєСѓРјРµРЅС‚Р°С†РёРё: РµСЃР»Рё Р·Р°РїРёСЃРµР№ Р±РѕР»СЊС€Рµ, С‚Рѕ РЅСѓР¶РЅРѕ РґРµР»Р°С‚СЊ РїРѕРІС‚РѕСЂРЅС‹Рµ Р·Р°РїСЂРѕСЃС‹
        // РёСЃРїРѕР»СЊР·СѓСЏ РїР°СЂР°РјРµС‚СЂ flag=1 РґР»СЏ РїРѕР»СѓС‡РµРЅРёСЏ СЃР»РµРґСѓСЋС‰РёС… СЃС‚СЂР°РЅРёС†

        let mut all_sales: Vec<(WbSaleRow, String)> = Vec::new();
        let mut page_flag = 0; // 0 = РїРµСЂРІР°СЏ СЃС‚СЂР°РЅРёС†Р°, 1 = СЃР»РµРґСѓСЋС‰РёРµ СЃС‚СЂР°РЅРёС†С‹

        self.log_to_file(&format!(
            "\nв•”в•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•—"
        ));
        self.log_to_file(&format!("в•‘ WILDBERRIES SALES API - LOADING ALL RECORDS"));
        self.log_to_file(&format!("в•‘ Period: {} to {}", date_from_str, date_to_str));
        self.log_to_file(&format!(
            "в•љв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ќ"
        ));

        loop {
            self.log_to_file(&format!(
                "\nв”Њв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”ђ"
            ));
            self.log_to_file(&format!(
                "в”‚ Request #{} (flag={})",
                (page_flag + 1),
                page_flag
            ));

            self.log_to_file(&format!(
                "=== REQUEST ===\nGET {}?dateFrom={}&dateTo={}&flag={}\nAuthorization: ****",
                url, date_from_str, date_to_str, page_flag
            ));

            self.record_http_request_attempt(0);
            let response = match self
                .client
                .get(url)
                .header("Authorization", &connection.api_key)
                .query(&[
                    ("dateFrom", date_from_str.as_str()),
                    ("dateTo", date_to_str.as_str()),
                    ("flag", &page_flag.to_string()),
                ])
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    let error_msg = format!("HTTP request failed: {:?}", e);
                    self.log_to_file(&error_msg);
                    tracing::error!("Wildberries Sales API connection error: {}", e);

                    // РџСЂРѕРІРµСЂСЏРµРј РєРѕРЅРєСЂРµС‚РЅС‹Рµ С‚РёРїС‹ РѕС€РёР±РѕРє
                    if e.is_timeout() {
                        anyhow::bail!("Request timeout: API РЅРµ РѕС‚РІРµС‚РёР» РІ С‚РµС‡РµРЅРёРµ 60 СЃРµРєСѓРЅРґ");
                    } else if e.is_connect() {
                        anyhow::bail!("Connection error: РЅРµ СѓРґР°Р»РѕСЃСЊ РїРѕРґРєР»СЋС‡РёС‚СЊСЃСЏ Рє СЃРµСЂРІРµСЂСѓ WB. РџСЂРѕРІРµСЂСЊС‚Рµ РёРЅС‚РµСЂРЅРµС‚-СЃРѕРµРґРёРЅРµРЅРёРµ.");
                    } else if e.is_request() {
                        anyhow::bail!("Request error: РїСЂРѕР±Р»РµРјР° РїСЂРё РѕС‚РїСЂР°РІРєРµ Р·Р°РїСЂРѕСЃР° - {}", e);
                    } else {
                        anyhow::bail!("Unknown error: {}", e);
                    }
                }
            };

            let status = response.status();
            self.log_to_file(&format!("Response status: {}", status));

            if !status.is_success() {
                let body = self
                    .read_body_for_recorded_request(response)
                    .await
                    .unwrap_or_default();
                self.log_to_file(&format!("ERROR Response body:\n{}", body));
                tracing::error!("Wildberries Sales API request failed: {}", body);
                anyhow::bail!(
                    "Wildberries Sales API failed with status {}: {}",
                    status,
                    body
                );
            }

            let body = self.read_body_for_recorded_request(response).await?;
            let body_preview = if body.chars().count() > 5000 {
                let preview: String = body.chars().take(5000).collect();
                format!("{}... (total {} chars)", preview, body.len())
            } else {
                body.clone()
            };
            self.log_to_file(&format!(
                "=== RESPONSE BODY PREVIEW ===\n{}\n",
                body_preview
            ));

            match serde_json::from_str::<Vec<WbSaleRow>>(&body) {
                Ok(page_data) => {
                    let page_count = page_data.len();
                    self.log_to_file(&format!("в”‚ Received: {} records", page_count));
                    self.log_to_file(&format!(
                        "в”‚ Total so far: {} records",
                        all_sales.len() + page_count
                    ));

                    if page_data.is_empty() {
                        self.log_to_file(&format!("в”‚ вњ“ Empty response - all records loaded"));
                        self.log_to_file(&format!(
                            "в””в”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”"
                        ));
                        break;
                    }

                    // РџР°СЂСЃРёРј С‚РµР»Рѕ РєР°Рє РјР°СЃСЃРёРІ serde_json::Value РґР»СЏ СЃРѕС…СЂР°РЅРµРЅРёСЏ РѕСЂРёРіРёРЅР°Р»СЊРЅРѕРіРѕ JSON
                    // Р•СЃР»Рё РЅРµ РїРѕР»СѓС‡Р°РµС‚СЃСЏ вЂ” РёСЃРїРѕР»СЊР·СѓРµРј РїСѓСЃС‚РѕР№ РѕР±СЉРµРєС‚ РєР°Рє fallback
                    let raw_values: Vec<serde_json::Value> =
                        serde_json::from_str(&body).unwrap_or_default();

                    let page_pairs: Vec<(WbSaleRow, String)> = page_data
                        .into_iter()
                        .zip(raw_values.into_iter())
                        .map(|(row, raw_val)| {
                            let raw_str = serde_json::to_string(&raw_val)
                                .unwrap_or_else(|_| "{}".to_string());
                            (row, raw_str)
                        })
                        .collect();

                    // Р”РѕР±Р°РІР»СЏРµРј РїРѕР»СѓС‡РµРЅРЅС‹Рµ РґР°РЅРЅС‹Рµ
                    all_sales.extend(page_pairs);

                    // API WB Statistics РІРѕР·РІСЂР°С‰Р°РµС‚ РјР°РєСЃРёРјСѓРј 100,000 Р·Р°РїРёСЃРµР№ Р·Р° Р·Р°РїСЂРѕСЃ
                    // Р•СЃР»Рё РїРѕР»СѓС‡РёР»Рё РјРµРЅСЊС€Рµ, Р·РЅР°С‡РёС‚ СЌС‚Рѕ РїРѕСЃР»РµРґРЅСЏСЏ СЃС‚СЂР°РЅРёС†Р°
                    if page_count < 100000 {
                        self.log_to_file(&format!(
                            "в”‚ вњ“ Received {} records (less than limit) - last page",
                            page_count
                        ));
                        self.log_to_file(&format!(
                            "в””в”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”"
                        ));
                        break;
                    }

                    self.log_to_file(&format!(
                        "в”‚ в†’ More records may be available, requesting next page..."
                    ));
                    self.log_to_file(&format!(
                        "в””в”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”"
                    ));

                    // РџРµСЂРµС…РѕРґРёРј Рє СЃР»РµРґСѓСЋС‰РµР№ СЃС‚СЂР°РЅРёС†Рµ
                    page_flag = 1;
                }
                Err(e) => {
                    self.log_to_file(&format!("Failed to parse JSON: {}", e));
                    tracing::error!("Failed to parse Wildberries sales response: {}", e);
                    anyhow::bail!("Failed to parse sales response: {}", e)
                }
            }

            // РќРµР±РѕР»СЊС€Р°СЏ Р·Р°РґРµСЂР¶РєР° РјРµР¶РґСѓ Р·Р°РїСЂРѕСЃР°РјРё РґР»СЏ СЃРЅРёР¶РµРЅРёСЏ РЅР°РіСЂСѓР·РєРё РЅР° API
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        self.log_to_file(&format!(
            "\nв•”в•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•—"
        ));
        self.log_to_file(&format!(
            "в•‘ COMPLETED: Loaded {} total sale records",
            all_sales.len()
        ));
        // all_sales СЃРѕРґРµСЂР¶РёС‚ РїР°СЂС‹ (WbSaleRow, raw_json_string)
        self.log_to_file(&format!(
            "в•љв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ќ\n"
        ));

        tracing::info!(
            "вњ“ Wildberries Sales API: Successfully loaded {} total records for period {} to {}",
            all_sales.len(),
            date_from_str,
            date_to_str
        );

        Ok(all_sales)
    }

    /// Р—Р°РіСЂСѓР·РёС‚СЊ С„РёРЅР°РЅСЃРѕРІС‹Рµ РѕС‚С‡РµС‚С‹ РёР· Wildberries РїРѕ РїРµСЂРёРѕРґСѓ (reportDetailByPeriod)
    /// Р’РѕР·РІСЂР°С‰Р°РµС‚ С‚РѕР»СЊРєРѕ Р•Р–Р•Р”РќР•Р’РќР«Р• РѕС‚С‡РµС‚С‹ (report_type = 1)
    ///
    /// Р’РђР–РќРћ: API РёРјРµРµС‚ Р»РёРјРёС‚ 1 Р·Р°РїСЂРѕСЃ РІ РјРёРЅСѓС‚Сѓ!
    /// РСЃРїРѕР»СЊР·СѓРµС‚СЃСЏ РїР°РіРёРЅР°С†РёСЏ С‡РµСЂРµР· rrdid РґР»СЏ Р·Р°РіСЂСѓР·РєРё Р±РѕР»СЊС€РёС… РѕР±СЉРµРјРѕРІ РґР°РЅРЅС‹С….
    pub async fn fetch_finance_report_by_period(
        &self,
        connection: &ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<Vec<WbFinanceReportRow>> {
        let url = "https://statistics-api.wildberries.ru/api/v5/supplier/reportDetailByPeriod";

        if connection.api_key.trim().is_empty() {
            anyhow::bail!("API Key is required for Wildberries API");
        }

        let date_from_str = date_from.format("%Y-%m-%d").to_string();
        let date_to_str = date_to.format("%Y-%m-%d").to_string();

        self.log_to_file(&format!(
            "\nв•”в•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•—"
        ));
        self.log_to_file(&format!(
            "в•‘ WILDBERRIES FINANCE REPORT API - reportDetailByPeriod"
        ));
        self.log_to_file(&format!("в•‘ Period: {} to {}", date_from_str, date_to_str));
        self.log_to_file(&format!(
            "в•‘ Rate limit: 1 request per minute (using pagination)"
        ));
        self.log_to_file(&format!(
            "в•љв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ќ"
        ));

        let period = "daily";
        let mut all_daily_reports: Vec<WbFinanceReportRow> = Vec::new();
        let mut rrdid: i64 = 0; // РќР°С‡РёРЅР°РµРј СЃ 0 РґР»СЏ РїРµСЂРІРѕР№ СЃС‚СЂР°РЅРёС†С‹
        let limit = 100000; // РњР°РєСЃРёРјР°Р»СЊРЅС‹Р№ Р»РёРјРёС‚ Р·Р°РїРёСЃРµР№
        let mut page_num = 1;

        loop {
            self.log_to_file(&format!(
                "\nв”Њв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”ђ"
            ));
            self.log_to_file(&format!(
                "в”‚ Page {}: rrdid={}, limit={}",
                page_num, rrdid, limit
            ));
            self.log_to_file(&format!(
                "в””в”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”"
            ));

            self.log_to_file(&format!(
                "=== REQUEST ===\nGET {}?dateFrom={}&dateTo={}&rrdid={}&limit={}&period={}\nAuthorization: ****",
                url, date_from_str, date_to_str, rrdid, limit, period
            ));

            let response = match self
                .client
                .get(url)
                .header("Authorization", &connection.api_key)
                .query(&[
                    ("dateFrom", date_from_str.as_str()),
                    ("dateTo", date_to_str.as_str()),
                    ("rrdid", &rrdid.to_string()),
                    ("limit", &limit.to_string()),
                    ("period", period),
                ])
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    let error_msg = format!("HTTP request failed: {:?}", e);
                    self.log_to_file(&error_msg);
                    tracing::error!("Wildberries Finance Report API connection error: {}", e);

                    // РџСЂРѕРІРµСЂСЏРµРј РєРѕРЅРєСЂРµС‚РЅС‹Рµ С‚РёРїС‹ РѕС€РёР±РѕРє
                    if e.is_timeout() {
                        anyhow::bail!("Request timeout: API РЅРµ РѕС‚РІРµС‚РёР» РІ С‚РµС‡РµРЅРёРµ 60 СЃРµРєСѓРЅРґ");
                    } else if e.is_connect() {
                        anyhow::bail!("Connection error: РЅРµ СѓРґР°Р»РѕСЃСЊ РїРѕРґРєР»СЋС‡РёС‚СЊСЃСЏ Рє СЃРµСЂРІРµСЂСѓ WB. РџСЂРѕРІРµСЂСЊС‚Рµ РёРЅС‚РµСЂРЅРµС‚-СЃРѕРµРґРёРЅРµРЅРёРµ.");
                    } else if e.is_request() {
                        anyhow::bail!("Request error: РїСЂРѕР±Р»РµРјР° РїСЂРё РѕС‚РїСЂР°РІРєРµ Р·Р°РїСЂРѕСЃР° - {}", e);
                    } else {
                        anyhow::bail!("Unknown error: {}", e);
                    }
                }
            };

            let status = response.status();
            self.log_to_file(&format!("Response status: {}", status));

            // РћР±СЂР°Р±РѕС‚РєР° 429 Too Many Requests - Р¶РґРµРј Рё РїРѕРІС‚РѕСЂСЏРµРј
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                self.log_to_file(&format!(
                    "в”‚ вљ пёЏ Rate limit hit (429). Waiting 65 seconds before retry..."
                ));
                tracing::warn!("WB Finance Report API rate limit hit. Waiting 65 seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(65)).await;
                continue;
            }

            // РћР±СЂР°Р±РѕС‚РєР° 204 No Content - РЅРµС‚ РґР°РЅРЅС‹С…
            if status == reqwest::StatusCode::NO_CONTENT {
                self.log_to_file(&format!("в”‚ No more data (204 No Content)"));
                break;
            }

            if !status.is_success() {
                let body = self
                    .read_body_for_recorded_request(response)
                    .await
                    .unwrap_or_default();
                self.log_to_file(&format!("ERROR Response body:\n{}", body));
                tracing::error!("Wildberries Finance Report API request failed: {}", body);
                anyhow::bail!(
                    "Wildberries Finance Report API failed with status {}: {}",
                    status,
                    body
                );
            }

            let body = self.read_body_tracked(response).await?;

            // РџСѓСЃС‚РѕР№ РѕС‚РІРµС‚ - РєРѕРЅРµС† РґР°РЅРЅС‹С…
            if body.trim().is_empty() || body.trim() == "[]" {
                self.log_to_file(&format!("в”‚ Empty response - no more data"));
                break;
            }

            let body_preview = if body.chars().count() > 5000 {
                let preview: String = body.chars().take(5000).collect();
                format!("{}... (total {} chars)", preview, body.len())
            } else {
                body.clone()
            };
            self.log_to_file(&format!(
                "=== RESPONSE BODY PREVIEW ===\n{}\n",
                body_preview
            ));

            // РџР°СЂСЃРёРј Р·Р°РїРёСЃРё
            let page_rows: Vec<WbFinanceReportRow> = match serde_json::from_str(&body) {
                Ok(rows) => rows,
                Err(e) => {
                    self.log_to_file(&format!("Failed to parse JSON: {}", e));
                    tracing::error!("Failed to parse Wildberries finance report response: {}", e);
                    anyhow::bail!("Failed to parse finance report response: {}", e)
                }
            };

            let page_count = page_rows.len();
            self.log_to_file(&format!(
                "в”‚ Received {} records on page {}",
                page_count, page_num
            ));

            if page_count == 0 {
                self.log_to_file(&format!("в”‚ No records on this page - done"));
                break;
            }

            // РќР°С…РѕРґРёРј РјР°РєСЃРёРјР°Р»СЊРЅС‹Р№ rrd_id РґР»СЏ СЃР»РµРґСѓСЋС‰РµР№ СЃС‚СЂР°РЅРёС†С‹
            let max_rrd_id = page_rows.iter().filter_map(|r| r.rrd_id).max().unwrap_or(0);

            // Р¤РёР»СЊС‚СЂСѓРµРј С‚РѕР»СЊРєРѕ Р•Р–Р•Р”РќР•Р’РќР«Р• РѕС‚С‡РµС‚С‹ (report_type = 1)
            let daily_rows: Vec<WbFinanceReportRow> = page_rows
                .into_iter()
                .filter(|row| row.report_type == Some(1))
                .collect();

            self.log_to_file(&format!(
                "в”‚ Filtered {} daily records (report_type=1)",
                daily_rows.len()
            ));

            all_daily_reports.extend(daily_rows);

            // Р•СЃР»Рё РїРѕР»СѓС‡РёР»Рё РјРµРЅСЊС€Рµ Р·Р°РїРёСЃРµР№ С‡РµРј Р»РёРјРёС‚, Р·РЅР°С‡РёС‚ СЌС‚Рѕ РїРѕСЃР»РµРґРЅСЏСЏ СЃС‚СЂР°РЅРёС†Р°
            if page_count < limit as usize {
                self.log_to_file(&format!(
                    "в”‚ Received {} < {} records - this is the last page",
                    page_count, limit
                ));
                break;
            }

            // РџРѕРґРіРѕС‚РѕРІРєР° Рє СЃР»РµРґСѓСЋС‰РµР№ СЃС‚СЂР°РЅРёС†Рµ
            rrdid = max_rrd_id;
            page_num += 1;

            self.log_to_file(&format!(
                "в”‚ в†’ More records may be available. Next rrdid={}",
                rrdid
            ));
            self.log_to_file(&format!(
                "в”‚ вЏі Waiting 65 seconds before next request (rate limit: 1 req/min)..."
            ));

            // Р’РђР–РќРћ: API РёРјРµРµС‚ Р»РёРјРёС‚ 1 Р·Р°РїСЂРѕСЃ РІ РјРёРЅСѓС‚Сѓ!
            // Р–РґРµРј 65 СЃРµРєСѓРЅРґ РґР»СЏ РЅР°РґРµР¶РЅРѕСЃС‚Рё
            tokio::time::sleep(tokio::time::Duration::from_secs(65)).await;
        }

        // Р›РѕРіРёСЂСѓРµРј РїРµСЂРІС‹Рµ 3 Р·Р°РїРёСЃРё РґР»СЏ РїСЂРѕРІРµСЂРєРё Р·Р°РіСЂСѓР·РєРё РїРѕР»РµР№
        for (idx, row) in all_daily_reports.iter().take(3).enumerate() {
            self.log_to_file(&format!(
                "\n=== Sample Record {} ===\nrrd_id: {:?}\ncommission_percent: {:?}\nppvz_sales_commission: {:?}\nretail_price_withdisc_rub: {:?}\nretail_amount: {:?}\n",
                idx + 1,
                row.rrd_id,
                row.commission_percent,
                row.ppvz_sales_commission,
                row.retail_price_withdisc_rub,
                row.retail_amount
            ));
            tracing::info!(
                "WB Finance Report sample {}: rrd_id={:?}, commission_percent={:?}, ppvz_sales_commission={:?}",
                idx + 1,
                row.rrd_id,
                row.commission_percent,
                row.ppvz_sales_commission
            );
        }

        self.log_to_file(&format!(
            "\nв•”в•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•—"
        ));
        self.log_to_file(&format!(
            "в•‘ COMPLETED: Loaded {} daily finance report records ({} pages)",
            all_daily_reports.len(),
            page_num
        ));
        self.log_to_file(&format!(
            "в•љв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ќ\n"
        ));

        tracing::info!(
            "вњ“ Wildberries Finance Report API: Successfully loaded {} daily records for period {} to {}",
            all_daily_reports.len(),
            date_from_str,
            date_to_str
        );

        Ok(all_daily_reports)
    }

    /// РџРѕР»СѓС‡РёС‚СЊ РґР°РЅРЅС‹Рµ РїРѕ Р·Р°РєР°Р·Р°Рј С‡РµСЂРµР· Statistics API (Backfill mode)
    /// GET /api/v1/supplier/orders
    ///
    /// РЎС‚СЂР°С‚РµРіРёСЏ:
    /// - flag=0 (РёРЅРєСЂРµРјРµРЅС‚ РїРѕ lastChangeDate)
    /// - dateFrom = РєСѓСЂСЃРѕСЂ lastChangeDate
    /// - РґР»СЏ СЃР»РµРґСѓСЋС‰РµР№ СЃС‚СЂР°РЅРёС†С‹ РєСѓСЂСЃРѕСЂ СЃРґРІРёРіР°РµРј РЅР° +1РјСЃ РѕС‚ РјР°РєСЃРёРјР°Р»СЊРЅРѕРіРѕ lastChangeDate
    /// - СЃРѕР±Р»СЋРґР°РµРј Р»РёРјРёС‚ API (1 Р·Р°РїСЂРѕСЃ/РјРёРЅ) Рё РѕР±СЂР°Р±Р°С‚С‹РІР°РµРј 429
    ///
    /// date_to РёСЃРїРѕР»СЊР·СѓРµС‚СЃСЏ РєР°Рє soft-stop / С„РёР»СЊС‚СЂ.
    pub async fn fetch_orders(
        &self,
        connection: &ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<Vec<WbOrderRow>> {
        let url = "https://statistics-api.wildberries.ru/api/v1/supplier/orders";

        if connection.api_key.trim().is_empty() {
            anyhow::bail!("API Key is required for Wildberries API");
        }

        let mut all_orders = Vec::new();
        let mut page_num = 1;
        let mut cursor = format!("{}T00:00:00", date_from.format("%Y-%m-%d"));
        let soft_stop =
            wb_day_end_utc(date_to).ok_or_else(|| anyhow::anyhow!("Invalid date_to value"))?;

        self.log_to_file(&format!(
            "\nв•”в•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•—"
        ));
        self.log_to_file(&format!("в•‘ WILDBERRIES ORDERS API - BACKFILL BY CURSOR"));
        self.log_to_file(&format!("в•‘ Period: {} to {}", date_from, date_to));
        self.log_to_file(&format!("в•‘ API URL: {}", url));
        self.log_to_file(&format!(
            "в•‘ Method: flag=0 with lastChangeDate cursor (1 req/min)"
        ));
        self.log_to_file(&format!(
            "в•љв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ќ"
        ));

        loop {
            self.log_to_file(&format!(
                "\nв”Њв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”ђ"
            ));
            self.log_to_file(&format!(
                "в”‚ Page {}: dateFrom={}, flag=0",
                page_num, cursor
            ));

            self.log_to_file(&format!(
                "=== REQUEST ===\nGET {}?dateFrom={}&flag=0\nAuthorization: ****",
                url, cursor
            ));

            self.set_tracked_current_item(
                "a015_wb_orders",
                format!("WB Orders API: запрос страницы {page_num}, dateFrom={cursor}"),
            );
            self.record_http_request_attempt(0);

            let response = match self
                .client
                .get(url)
                .header("Authorization", &connection.api_key)
                .query(&[("dateFrom", cursor.as_str()), ("flag", "0")])
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    let error_debug = format!("{e:?}");
                    let error_msg = format!("HTTP request to Orders API failed: {error_debug}");
                    self.log_to_file(&error_msg);
                    tracing::error!("Wildberries Orders API request error: {}", error_debug);
                    self.set_tracked_current_item(
                        "a015_wb_orders",
                        format!("WB Orders API: ошибка запроса страницы {page_num}"),
                    );

                    if e.is_timeout() {
                        anyhow::bail!(
                            "WB Orders API timeout: сервер не ответил за 60 секунд.\n\
                             URL: {url}?dateFrom={cursor}&flag=0\n\
                             Детали: {error_debug}"
                        );
                    } else if e.is_connect() {
                        anyhow::bail!(
                            "WB Orders API connection failed: не удалось установить соединение с statistics-api.wildberries.ru.\n\
                             Это сетевая ошибка до HTTP-ответа, не ответ 429 и не ошибка формата данных.\n\
                             URL: {url}?dateFrom={cursor}&flag=0\n\
                             Проверьте доступ к хосту, DNS/proxy/firewall/VPN и TLS-соединение.\n\
                             Детали: {error_debug}"
                        );
                    } else if e.is_request() {
                        anyhow::bail!(
                            "WB Orders API request build/send error.\n\
                             URL: {url}?dateFrom={cursor}&flag=0\n\
                             Детали: {error_debug}"
                        );
                    } else {
                        anyhow::bail!(
                            "WB Orders API request failed before receiving a response.\n\
                             URL: {url}?dateFrom={cursor}&flag=0\n\
                             Детали: {error_debug}"
                        );
                    }
                }
            };

            let status = response.status();
            let final_url = response.url().clone();
            self.log_to_file(&format!("Response status: {}", status));
            self.log_to_file(&format!("Final URL: {}", final_url));

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let rate_headers = WbRateLimitHeaders::from_headers(response.headers());
                let wait_secs = rate_headers.retry_seconds.unwrap_or(65).max(1);
                let rate_fields = rate_headers.to_log_fields();
                let body = self
                    .read_body_for_recorded_request(response)
                    .await
                    .unwrap_or_default();
                self.log_to_file(&format!(
                    "в”‚ вљ пёЏ Rate limit hit (429). Waiting {} seconds before retry. X-Ratelimit: {}",
                    wait_secs, rate_fields
                ));
                if !body.trim().is_empty() {
                    self.log_to_file(&format!("Rate limit response body:\n{}", body));
                }

                if wait_secs > WB_ORDERS_MAX_RATE_LIMIT_SLEEP_SECS {
                    let message = format!(
                        "WB Orders API rate limit returned a long retry window: {} seconds. \
                         The task will finish now and can be retried by the next scheduled/manual run. \
                         X-Ratelimit: {}",
                        wait_secs, rate_fields
                    );
                    self.log_to_file(&message);
                    self.set_tracked_current_item(
                        "a015_wb_orders",
                        format!(
                            "WB Orders API: лимит запросов (429), WB просит ждать {} сек.; задача завершена",
                            wait_secs
                        ),
                    );
                    tracing::warn!("{}", message);
                    anyhow::bail!("WB_RATE_LIMIT_DEFERRED: {}", message);
                }

                self.set_tracked_current_item(
                    "a015_wb_orders",
                    format!(
                        "WB Orders API: лимит запросов (429), ожидание {} сек. {}",
                        wait_secs, rate_fields
                    ),
                );
                tracing::warn!(
                    "WB Orders API rate limit hit. Waiting {} seconds. X-Ratelimit: {}",
                    wait_secs,
                    rate_fields
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(wait_secs)).await;
                continue;
            }

            // Р›РѕРіРёСЂСѓРµРј Р·Р°РіРѕР»РѕРІРєРё РѕС‚РІРµС‚Р° РґР»СЏ РґРёР°РіРЅРѕСЃС‚РёРєРё
            self.log_to_file(&format!("Response headers:"));
            for (name, value) in response.headers() {
                if let Ok(val_str) = value.to_str() {
                    self.log_to_file(&format!("  {}: {}", name, val_str));
                }
            }

            if !status.is_success() {
                let body = self.read_body_tracked(response).await.unwrap_or_default();
                self.log_to_file(&format!("ERROR Response body:\n{}", body));
                tracing::error!(
                    "Wildberries Orders API request failed for cursor {}: {}",
                    cursor,
                    body
                );

                // РЎРїРµС†РёР°Р»СЊРЅР°СЏ РѕР±СЂР°Р±РѕС‚РєР° РґР»СЏ 302 СЂРµРґРёСЂРµРєС‚РѕРІ
                if status.as_u16() == 302 || status.as_u16() == 301 {
                    anyhow::bail!(
                        "Wildberries Orders API returned redirect {} for cursor {}. \
                        This may indicate:\n\
                        1. Incorrect API endpoint URL\n\
                        2. Missing or invalid authentication\n\
                        3. API endpoint has moved\n\
                        Response: {}\n\
                        Check Wildberries API documentation for the correct endpoint.",
                        status,
                        cursor,
                        body
                    );
                }

                anyhow::bail!(
                    "Wildberries Orders API failed with status {} for cursor {}: {}",
                    status,
                    cursor,
                    body
                );
            }

            // Р§РёС‚Р°РµРј С‚РµР»Рѕ РѕС‚РІРµС‚Р°
            let body = match self.read_body_for_recorded_request(response).await {
                Ok(b) => b,
                Err(e) => {
                    self.log_to_file(&format!("в”‚ вљ пёЏ Failed to read response body: {}", e));
                    tracing::error!("Failed to read response body for cursor {}: {}", cursor, e);
                    anyhow::bail!("Failed to read response body: {}", e);
                }
            };

            self.log_to_file(&format!("Body length: {} bytes", body.len()));

            // РџСЂРѕРІРµСЂСЏРµРј, РЅРµ РїСѓСЃС‚РѕР№ Р»Рё РѕС‚РІРµС‚
            let body_trimmed = body.trim();
            if body_trimmed.is_empty() || body_trimmed == "[]" {
                self.log_to_file(&format!("в”‚ Empty response, all records loaded"));
                self.log_to_file(&format!("в”‚ Total so far: {} records", all_orders.len()));
                self.log_to_file(&format!(
                    "в””в”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”"
                ));
                break;
            }

            let body_preview = if body.chars().count() > 5000 {
                let preview: String = body.chars().take(5000).collect();
                format!("{}... (total {} chars)", preview, body.len())
            } else {
                body.clone()
            };
            self.log_to_file(&format!(
                "=== RESPONSE BODY PREVIEW ===\n{}\n",
                body_preview
            ));

            match serde_json::from_str::<Vec<WbOrderRow>>(&body) {
                Ok(page_data) => {
                    let page_count = page_data.len();
                    self.log_to_file(&format!(
                        "в”‚ Received: {} rows on page {}",
                        page_count, page_num
                    ));
                    self.log_to_file(&format!(
                        "в”‚ Total so far: {} records",
                        all_orders.len() + page_count
                    ));
                    self.log_to_file(&format!(
                        "в””в”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”Ђв”"
                    ));

                    let mut max_last_change = None::<chrono::DateTime<chrono::Utc>>;
                    let mut kept_rows = 0usize;
                    for row in page_data {
                        let row_last_change =
                            row.last_change_date.as_deref().and_then(parse_wb_datetime);

                        if let Some(parsed) = row_last_change {
                            if max_last_change.map(|v| parsed > v).unwrap_or(true) {
                                max_last_change = Some(parsed);
                            }
                        }

                        // soft-stop РїРѕ date_to: СЃС‚СЂРѕРєРё РїРѕСЃР»Рµ date_to РЅРµ РІРєР»СЋС‡Р°РµРј
                        let include_row = row_last_change.map(|dt| dt <= soft_stop).unwrap_or(true);
                        if include_row {
                            all_orders.push(row);
                            kept_rows += 1;
                        }
                    }

                    self.log_to_file(&format!(
                        "в”‚ Kept {} rows after soft-stop filter",
                        kept_rows
                    ));

                    let Some(max_dt) = max_last_change else {
                        self.log_to_file("в”‚ No lastChangeDate found on page; stopping");
                        break;
                    };

                    if max_dt > soft_stop {
                        self.log_to_file(&format!(
                            "в”‚ Soft-stop reached (max lastChangeDate {} > date_to {})",
                            max_dt, soft_stop
                        ));
                        break;
                    }

                    let next_cursor_dt = max_dt + chrono::Duration::milliseconds(1);
                    cursor = format_wb_cursor_datetime(&next_cursor_dt);
                    page_num += 1;
                }
                Err(e) => {
                    self.log_to_file(&format!("Failed to parse JSON: {}", e));
                    self.log_to_file(&format!("Response body: {}", body_preview));
                    tracing::error!("Failed to parse Wildberries orders response: {}", e);
                    anyhow::bail!("Failed to parse orders response: {}", e)
                }
            }

            // Р›РёРјРёС‚ WB Statistics: 1 Р·Р°РїСЂРѕСЃ РІ РјРёРЅСѓС‚Сѓ
            tokio::time::sleep(tokio::time::Duration::from_secs(65)).await;
        }

        self.log_to_file(&format!(
            "\nв•”в•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•—"
        ));
        self.log_to_file(&format!(
            "в•‘ COMPLETED: Loaded {} total order records",
            all_orders.len()
        ));
        self.log_to_file(&format!(
            "в•љв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ќ\n"
        ));

        tracing::info!(
            "вњ“ Wildberries Orders API: Successfully loaded {} total records for period {} to {}",
            all_orders.len(),
            date_from,
            date_to
        );

        Ok(all_orders)
    }

    pub async fn fetch_documents_list(
        &self,
        connection: &ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<Vec<WbDocumentListItem>> {
        // WB Documents List API: empirical limit ~1 req/10 s (burst 5).
        // We add an inter-page delay of 11 s.
        //
        // ВАЖНО: API не гарантирует фильтрацию по дате через beginTime/endTime.
        // Сортировка desc по дате позволяет сделать early-exit: как только видим документ
        // старше date_from — дальше не идём, т.к. всё остальное ещё старше.
        const PAGE_DELAY_SECS: u64 = 11;
        // Максимум попыток на одну страницу при 429 (защита от вечной петли).
        const MAX_RETRIES_PER_PAGE: u32 = 3;
        const RATE_LIMIT_DEFAULT_WAIT_SECS: u64 = 15;
        // Если API говорит ждать больше этого порога — исчерпана дневная квота;
        // немедленно возвращаем ошибку, чтобы не ждать часами.
        const QUOTA_EXHAUSTED_THRESHOLD_SECS: u64 = 300; // 5 минут

        let url = "https://documents-api.wildberries.ru/api/v1/documents/list";

        if connection.api_key.trim().is_empty() {
            anyhow::bail!("API Key is required for Wildberries API");
        }

        let begin_time = date_from.format("%Y-%m-%d").to_string();
        let end_time = date_to.format("%Y-%m-%d").to_string();
        let limit = 50usize;
        let mut offset = 0usize;
        let mut all_documents = Vec::new();

        'pages: loop {
            // Delay before every page except the very first.
            if offset > 0 {
                tokio::time::sleep(tokio::time::Duration::from_secs(PAGE_DELAY_SECS)).await;
            }

            let mut retries = 0u32;
            let batch = loop {
                let response = self
                    .client
                    .get(url)
                    .header("Authorization", &connection.api_key)
                    .query(&[
                        ("locale", "ru"),
                        ("beginTime", begin_time.as_str()),
                        ("endTime", end_time.as_str()),
                        ("sort", "date"),
                        ("order", "desc"),
                        ("limit", &limit.to_string()),
                        ("offset", &offset.to_string()),
                    ])
                    .send()
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to fetch WB documents list: {}", e))?;

                let status = response.status();

                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    let retry_after = response
                        .headers()
                        .get("X-Ratelimit-Retry")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(RATE_LIMIT_DEFAULT_WAIT_SECS);

                    // Если API просит ждать слишком долго — дневная квота исчерпана.
                    // Немедленно возвращаем ошибку вместо многочасового ожидания.
                    // Префикс QUOTA_EXHAUSTED: позволяет worker-у отложить следующий запуск на 24ч.
                    if retry_after > QUOTA_EXHAUSTED_THRESHOLD_SECS {
                        anyhow::bail!(
                            "QUOTA_EXHAUSTED: WB Documents API: дневная квота исчерпана. \
                             API требует ждать {} с (~{} ч). \
                             Следующий запуск автоматически перенесён на 24 ч.",
                            retry_after,
                            retry_after / 3600
                        );
                    }

                    retries += 1;
                    if retries > MAX_RETRIES_PER_PAGE {
                        anyhow::bail!(
                            "WB Documents List API: превышено {} попыток при rate-limit (offset={}). \
                             Задача остановлена.",
                            MAX_RETRIES_PER_PAGE, offset
                        );
                    }
                    let wait_secs = retry_after.max(RATE_LIMIT_DEFAULT_WAIT_SECS);
                    tracing::warn!(
                        "WB Documents API 429 (попытка {}/{}): ждём {} с (offset={}).",
                        retries,
                        MAX_RETRIES_PER_PAGE,
                        wait_secs,
                        offset
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(wait_secs)).await;
                    continue;
                }

                if !status.is_success() {
                    let body = self.read_body_tracked(response).await.unwrap_or_default();
                    anyhow::bail!(
                        "Wildberries documents list failed with status {}: {}",
                        status,
                        body
                    );
                }

                let body = self.read_body_tracked(response).await?;
                let parsed: WbDocumentsListResponse = serde_json::from_str(&body).map_err(|e| {
                    anyhow::anyhow!("Failed to parse WB documents list response: {}", e)
                })?;
                break parsed.data.documents;
            };

            let batch_len = batch.len();

            // Применяем клиентскую фильтрацию по дате и early-exit.
            // API сортирует desc, поэтому первый документ старше date_from = конец диапазона.
            for doc in batch {
                // creation_time может быть "YYYY-MM-DD" или "YYYY-MM-DDTHH:MM:SSZ"
                let doc_date = doc
                    .creation_time
                    .get(..10)
                    .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

                if let Some(d) = doc_date {
                    if d > date_to {
                        // Документ новее окна — пропускаем (API мог вернуть лишнее)
                        continue;
                    }
                    if d < date_from {
                        // Документ старше окна — дальше всё ещё старше, останавливаемся
                        tracing::debug!(
                            "WB Documents: early-exit на дате {} (date_from={}), offset={}",
                            d,
                            date_from,
                            offset
                        );
                        break 'pages;
                    }
                }
                all_documents.push(doc);
            }

            if batch_len < limit {
                break;
            }

            offset += limit;
        }

        Ok(all_documents)
    }

    pub async fn download_document(
        &self,
        connection: &ConnectionMP,
        service_name: &str,
        extension: &str,
    ) -> Result<WbDocumentDownloadFile> {
        let url = "https://documents-api.wildberries.ru/api/v1/documents/download";

        if connection.api_key.trim().is_empty() {
            anyhow::bail!("API Key is required for Wildberries API");
        }

        let response = self
            .client
            .get(url)
            .header("Authorization", &connection.api_key)
            .query(&[("serviceName", service_name), ("extension", extension)])
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to download WB document: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let body = self.read_body_tracked(response).await.unwrap_or_default();
            anyhow::bail!(
                "Wildberries document download failed with status {}: {}",
                status,
                body
            );
        }

        let body = self.read_body_tracked(response).await?;
        let parsed: WbDocumentDownloadResponse = serde_json::from_str(&body)
            .map_err(|e| anyhow::anyhow!("Failed to parse WB document download response: {}", e))?;

        Ok(parsed.data)
    }

    /// РџРѕР»СѓС‡РёС‚СЊ С‚Р°СЂРёС„С‹ РєРѕРјРёСЃСЃРёР№ РїРѕ РєР°С‚РµРіРѕСЂРёСЏРј
    /// GET https://common-api.wildberries.ru/api/v1/tariffs/commission?locale=ru
    ///
    /// РўСЂРµР±СѓРµС‚ Р°РІС‚РѕСЂРёР·Р°С†РёСЋ С‡РµСЂРµР· API РєР»СЋС‡
    pub async fn fetch_commission_tariffs(
        &self,
        connection: &ConnectionMP,
    ) -> Result<Vec<CommissionTariffRow>> {
        let url = "https://common-api.wildberries.ru/api/v1/tariffs/commission?locale=ru";

        if connection.api_key.trim().is_empty() {
            anyhow::bail!("API Key is required for Wildberries Commission Tariffs API");
        }

        self.log_to_file(&format!(
            "\nв•”в•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•—"
        ));
        self.log_to_file(&format!("в•‘ WILDBERRIES COMMISSION TARIFFS API"));
        self.log_to_file(&format!("в•‘ URL: {}", url));
        self.log_to_file(&format!("в•‘ Method: GET (requires Authorization header)"));
        self.log_to_file(&format!(
            "в•љв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ђв•ќ"
        ));

        self.log_to_file(&format!(
            "=== REQUEST ===\nGET {}\nAuthorization: ****",
            url
        ));

        let response = match self
            .client
            .get(url)
            .header("Authorization", &connection.api_key)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error_msg = format!("HTTP request failed: {:?}", e);
                self.log_to_file(&error_msg);
                tracing::error!("Wildberries Commission Tariffs API connection error: {}", e);

                // РџСЂРѕРІРµСЂСЏРµРј РєРѕРЅРєСЂРµС‚РЅС‹Рµ С‚РёРїС‹ РѕС€РёР±РѕРє
                if e.is_timeout() {
                    anyhow::bail!("Request timeout: API РЅРµ РѕС‚РІРµС‚РёР» РІ С‚РµС‡РµРЅРёРµ 60 СЃРµРєСѓРЅРґ");
                } else if e.is_connect() {
                    anyhow::bail!("Connection error: РЅРµ СѓРґР°Р»РѕСЃСЊ РїРѕРґРєР»СЋС‡РёС‚СЊСЃСЏ Рє СЃРµСЂРІРµСЂСѓ WB. РџСЂРѕРІРµСЂСЊС‚Рµ РёРЅС‚РµСЂРЅРµС‚-СЃРѕРµРґРёРЅРµРЅРёРµ.");
                } else if e.is_request() {
                    anyhow::bail!("Request error: РїСЂРѕР±Р»РµРјР° РїСЂРё РѕС‚РїСЂР°РІРєРµ Р·Р°РїСЂРѕСЃР° - {}", e);
                } else {
                    anyhow::bail!("Unknown error: {}", e);
                }
            }
        };

        let status = response.status();
        self.log_to_file(&format!("Response status: {}", status));

        if !status.is_success() {
            let body = self.read_body_tracked(response).await.unwrap_or_default();
            self.log_to_file(&format!("ERROR Response body:\n{}", body));
            tracing::error!(
                "Wildberries Commission Tariffs API request failed: {}",
                body
            );
            anyhow::bail!(
                "Wildberries Commission Tariffs API failed with status {}: {}",
                status,
                body
            );
        }

        let body = self.read_body_tracked(response).await?;
        self.log_to_file(&format!("=== RESPONSE BODY ===\n{}\n", body));

        // Parse JSON response
        let parsed: CommissionTariffResponse = serde_json::from_str(&body).map_err(|e| {
            self.log_to_file(&format!("ERROR: Failed to parse JSON: {}", e));
            anyhow::anyhow!("Failed to parse commission tariffs response: {}", e)
        })?;

        self.log_to_file(&format!(
            "вњ“ Successfully parsed {} commission tariff records",
            parsed.report.len()
        ));

        tracing::info!(
            "вњ“ Wildberries Commission Tariffs API: Successfully loaded {} tariff records",
            parsed.report.len()
        );

        Ok(parsed.report)
    }

    /// РџРѕР»СѓС‡РёС‚СЊ СЃС‚СЂР°РЅРёС†Сѓ С†РµРЅ С‚РѕРІР°СЂРѕРІ РёР· WB Prices API
    /// GET https://discounts-prices-api.wildberries.ru/api/v2/list/goods/filter?limit=N&offset=N
    pub async fn fetch_goods_prices(
        &self,
        connection: &ConnectionMP,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<WbGoodsPriceRow>> {
        let url = format!(
            "https://discounts-prices-api.wildberries.ru/api/v2/list/goods/filter?limit={}&offset={}",
            limit, offset
        );

        if connection.api_key.trim().is_empty() {
            anyhow::bail!("API Key is required for Wildberries Prices API");
        }

        self.log_to_file(&format!(
            "=== REQUEST ===\nGET {}\nAuthorization: ****",
            url
        ));

        let response = match self
            .client
            .get(&url)
            .header("Authorization", &connection.api_key)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error_msg = format!("HTTP request failed: {:?}", e);
                self.log_to_file(&error_msg);
                tracing::error!("Wildberries Prices API connection error: {}", e);
                if e.is_timeout() {
                    anyhow::bail!("Request timeout: WB Prices API РЅРµ РѕС‚РІРµС‚РёР» РІ С‚РµС‡РµРЅРёРµ 60 СЃРµРєСѓРЅРґ");
                } else if e.is_connect() {
                    anyhow::bail!("Connection error: РЅРµ СѓРґР°Р»РѕСЃСЊ РїРѕРґРєР»СЋС‡РёС‚СЊСЃСЏ Рє discounts-prices-api.wildberries.ru");
                } else {
                    anyhow::bail!("Unknown error: {}", e);
                }
            }
        };

        let status = response.status();
        self.log_to_file(&format!("Response status: {}", status));

        if !status.is_success() {
            let body = self.read_body_tracked(response).await.unwrap_or_default();
            self.log_to_file(&format!("ERROR Response body:\n{}", body));
            tracing::error!("Wildberries Prices API request failed: {}", body);
            anyhow::bail!(
                "Wildberries Prices API failed with status {}: {}",
                status,
                body
            );
        }

        let body = self.read_body_tracked(response).await?;
        self.log_to_file(&format!(
            "=== RESPONSE BODY ===\n{}\n",
            &body[..body.len().min(2000)]
        ));

        let parsed: WbGoodsPriceFilterResponse = serde_json::from_str(&body).map_err(|e| {
            self.log_to_file(&format!("ERROR: Failed to parse JSON: {}", e));
            anyhow::anyhow!("Failed to parse WB Prices response: {}", e)
        })?;

        let rows = parsed.data.map(|d| d.list_goods).unwrap_or_default();
        self.log_to_file(&format!("вњ“ Parsed {} goods price rows", rows.len()));
        tracing::info!(
            "WB Prices API: loaded {} rows (offset={})",
            rows.len(),
            offset
        );

        Ok(rows)
    }

    /// GET /api/v1/calendar/promotions вЂ” СЃРїРёСЃРѕРє Р°РєС†РёР№ РёР· WB Calendar API
    pub async fn fetch_calendar_promotions(
        &self,
        connection: &ConnectionMP,
        start_date_time: &str,
        end_date_time: &str,
        all_promo: bool,
    ) -> Result<Vec<WbCalendarPromotion>> {
        let url = format!(
            "https://dp-calendar-api.wildberries.ru/api/v1/calendar/promotions?startDateTime={}&endDateTime={}&allPromo={}",
            start_date_time, end_date_time, all_promo
        );

        if connection.api_key.trim().is_empty() {
            anyhow::bail!("API Key is required for Wildberries Promotion API");
        }

        self.log_to_file(&format!(
            "=== REQUEST ===\nGET {}\nAuthorization: ****",
            url
        ));

        let response = match self
            .client
            .get(&url)
            .header("Authorization", &connection.api_key)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error_msg = format!("HTTP request failed: {:?}", e);
                self.log_to_file(&error_msg);
                tracing::error!("WB Promotion API connection error: {}", e);
                if e.is_timeout() {
                    anyhow::bail!(
                        "Request timeout: WB Promotion API РЅРµ РѕС‚РІРµС‚РёР» РІ С‚РµС‡РµРЅРёРµ 60 СЃРµРєСѓРЅРґ"
                    );
                } else if e.is_connect() {
                    anyhow::bail!("Connection error: РЅРµ СѓРґР°Р»РѕСЃСЊ РїРѕРґРєР»СЋС‡РёС‚СЊСЃСЏ Рє dp-calendar-api.wildberries.ru");
                } else {
                    anyhow::bail!("Unknown error: {}", e);
                }
            }
        };

        let status = response.status();
        self.log_to_file(&format!("Response status: {}", status));

        if !status.is_success() {
            let body = self.read_body_tracked(response).await.unwrap_or_default();
            self.log_to_file(&format!("ERROR Response body:\n{}", body));
            tracing::error!("WB Promotion API request failed: {}", body);
            anyhow::bail!(
                "WB Promotion Calendar API failed with status {}: {}",
                status,
                body
            );
        }

        let body = self.read_body_tracked(response).await?;
        let body_preview: String = body.chars().take(2000).collect();
        self.log_to_file(&format!("=== RESPONSE BODY ===\n{}\n", body_preview));

        let parsed: WbCalendarPromotionsResponse = serde_json::from_str(&body).map_err(|e| {
            let snippet: String = body.chars().take(400).collect();
            self.log_to_file(&format!(
                "ERROR: Failed to parse JSON: {}\nRaw body: {}",
                e, snippet
            ));
            tracing::error!(
                "WB Calendar Promotions parse error: {} | body: {}",
                e,
                snippet
            );
            anyhow::anyhow!("Failed to parse WB Calendar Promotions response: {}", e)
        })?;

        let promotions = if let Some(data) = parsed.data {
            let mut all = data.promotions;
            all.extend(data.upcoming_promos);
            all
        } else {
            vec![]
        };
        self.log_to_file(&format!("вњ“ Parsed {} promotions", promotions.len()));
        tracing::info!("WB Calendar API: loaded {} promotions", promotions.len());

        Ok(promotions)
    }

    /// GET /api/v1/calendar/promotions/details вЂ” РґРµС‚Р°Р»СЊРЅР°СЏ РёРЅС„РѕСЂРјР°С†РёСЏ РїРѕ СЃРїРёСЃРєСѓ Р°РєС†РёР№ (РґРѕ 100 ID Р·Р° СЂР°Р·)
    pub async fn fetch_promotion_details(
        &self,
        connection: &ConnectionMP,
        promotion_ids: &[i64],
    ) -> Result<Vec<WbCalendarPromotionDetail>> {
        if promotion_ids.is_empty() {
            return Ok(vec![]);
        }
        if connection.api_key.trim().is_empty() {
            anyhow::bail!("API Key is required for Wildberries Promotion Details API");
        }

        // Р¤РѕСЂРјРёСЂСѓРµРј query string: promotionIDs=1&promotionIDs=2&...
        let query: String = promotion_ids
            .iter()
            .map(|id| format!("promotionIDs={}", id))
            .collect::<Vec<_>>()
            .join("&");

        let url = format!(
            "https://dp-calendar-api.wildberries.ru/api/v1/calendar/promotions/details?{}",
            query
        );

        self.log_to_file(&format!(
            "=== REQUEST ===\nGET {}\nAuthorization: ****",
            url
        ));

        let response = match self
            .client
            .get(&url)
            .header("Authorization", &connection.api_key)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("WB Promotion Details API connection error: {}", e);
                anyhow::bail!("Promotion details request error: {}", e);
            }
        };

        let status = response.status();
        if !status.is_success() {
            let err_body = self.read_body_tracked(response).await.unwrap_or_default();
            tracing::warn!("WB Promotion Details API failed: {} - {}", status, err_body);
            return Ok(vec![]);
        }

        let body = self.read_body_tracked(response).await?;
        let body_preview: String = body.chars().take(500).collect();
        self.log_to_file(&format!("=== DETAILS RESPONSE ===\n{}\n", body_preview));

        let parsed: WbCalendarPromotionDetailsResponse = match serde_json::from_str(&body) {
            Ok(p) => p,
            Err(e) => {
                let snippet: String = body.chars().take(400).collect();
                tracing::error!(
                    "WB Promotion Details parse error: {} | body: {}",
                    e,
                    snippet
                );
                return Ok(vec![]);
            }
        };

        let details = parsed.data.map(|d| d.promotions).unwrap_or_default();
        tracing::info!("WB Promotion Details: {} promotions loaded", details.len());

        Ok(details)
    }

    /// GET /api/v1/calendar/promotions/nomenclatures вЂ” СЃРїРёСЃРѕРє nmId С‚РѕРІР°СЂРѕРІ РґР»СЏ Р°РєС†РёРё
    /// РћР±СЏР·Р°С‚РµР»СЊРЅС‹Рµ РїР°СЂР°РјРµС‚СЂС‹: promotionID + inAction
    /// РќРµ СЂР°Р±РѕС‚Р°РµС‚ РґР»СЏ Р°РєС†РёР№ С‚РёРїР° "auto"
    pub async fn fetch_promotion_nomenclatures(
        &self,
        connection: &ConnectionMP,
        promotion_id: i64,
        promotion_type: Option<&str>,
    ) -> Result<Vec<i64>> {
        // РђРІС‚РѕРјР°С‚РёС‡РµСЃРєРёРµ Р°РєС†РёРё РЅРµ РїРѕРґРґРµСЂР¶РёРІР°СЋС‚ СЌС‚РѕС‚ СЌРЅРґРїРѕРёРЅС‚
        if promotion_type.map(|t| t == "auto").unwrap_or(false) {
            tracing::debug!(
                "Skipping nomenclatures for auto promotion {} (not supported)",
                promotion_id
            );
            return Ok(vec![]);
        }

        if connection.api_key.trim().is_empty() {
            anyhow::bail!("API Key is required for Wildberries Promotion Nomenclatures API");
        }

        let mut all_nm_ids: Vec<i64> = Vec::new();
        let page_size: u32 = 1000;

        // Р—Р°РіСЂСѓР¶Р°РµРј РѕР±Р° СЃРѕСЃС‚РѕСЏРЅРёСЏ: СѓС‡Р°СЃС‚РІСѓСЋС‰РёРµ (inAction=true) Рё РїРѕРґС…РѕРґСЏС‰РёРµ (inAction=false)
        for in_action in [true, false] {
            let mut offset: u32 = 0;
            loop {
                let url = format!(
                    "https://dp-calendar-api.wildberries.ru/api/v1/calendar/promotions/nomenclatures?promotionID={}&inAction={}&limit={}&offset={}",
                    promotion_id, in_action, page_size, offset
                );

                self.log_to_file(&format!(
                    "=== REQUEST ===\nGET {}\nAuthorization: ****",
                    url
                ));

                let response = match self
                    .client
                    .get(&url)
                    .header("Authorization", &connection.api_key)
                    .send()
                    .await
                {
                    Ok(resp) => resp,
                    Err(e) => {
                        let error_msg = format!("HTTP request failed: {:?}", e);
                        self.log_to_file(&error_msg);
                        tracing::error!("WB Promotion Nomenclatures API connection error: {}", e);
                        break;
                    }
                };

                let status = response.status();
                self.log_to_file(&format!("Response status: {}", status));

                if !status.is_success() {
                    let err_body = self.read_body_tracked(response).await.unwrap_or_default();
                    self.log_to_file(&format!("ERROR Response body:\n{}", err_body));
                    tracing::warn!(
                        "WB Promotion Nomenclatures API failed for promotionID={} inAction={}: {} - {}",
                        promotion_id, in_action, status, err_body
                    );
                    break;
                }

                let body = self.read_body_tracked(response).await.unwrap_or_default();
                let body_preview: String = body.chars().take(500).collect();
                self.log_to_file(&format!("=== RESPONSE BODY ===\n{}\n", body_preview));

                let parsed: WbPromotionNomenclaturesResponse = match serde_json::from_str(&body) {
                    Ok(p) => p,
                    Err(e) => {
                        let snippet: String = body.chars().take(400).collect();
                        tracing::error!(
                            "WB Promotion Nomenclatures parse error: {} | body: {}",
                            e,
                            snippet
                        );
                        break;
                    }
                };

                let items = parsed.data.map(|d| d.nomenclatures).unwrap_or_default();

                let page_len = items.len() as u32;
                for item in items {
                    if !all_nm_ids.contains(&item.nm_id) {
                        all_nm_ids.push(item.nm_id);
                    }
                }

                if page_len < page_size {
                    break;
                }
                offset += page_size;
            }
        }

        tracing::info!(
            "WB Promotion Nomenclatures: {} unique nmIds for promotionID={}",
            all_nm_ids.len(),
            promotion_id
        );

        Ok(all_nm_ids)
    }

    /// GET /adv/v1/promotion/count вЂ” РїРѕР»СѓС‡РёС‚СЊ РІСЃРµ advertId СЂРµРєР»Р°РјРЅС‹С… РєР°РјРїР°РЅРёР№ (СЃС‚Р°С‚СѓСЃС‹ 7, 9, 11)
    pub async fn fetch_advert_campaign_ids(&self, connection: &ConnectionMP) -> Result<Vec<i64>> {
        let url = "https://advert-api.wildberries.ru/adv/v1/promotion/count";

        if connection.api_key.trim().is_empty() {
            anyhow::bail!("API Key is required for Wildberries Advert API");
        }

        self.log_to_file(&format!(
            "=== REQUEST ===\nGET {}\nAuthorization: ****",
            url
        ));

        let response = match self
            .client
            .get(url)
            .header("Authorization", &connection.api_key)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("WB Advert campaign list connection error: {}", e);
                anyhow::bail!("Connection error for advert campaign list: {}", e);
            }
        };

        let status = response.status();
        let rate_limit = WbRateLimitHeaders::from_headers(response.headers());
        self.log_to_file(&format!("Response status: {}", status));
        self.log_to_file(&format!(
            "Response X-Ratelimit headers: {}",
            rate_limit.to_log_fields()
        ));

        if !status.is_success() {
            let body = self.read_body_tracked(response).await.unwrap_or_default();
            self.log_to_file(&format!("ERROR Response body:\n{}", body));
            tracing::error!(
                "WB Advert campaign list failed: {} - {}{}",
                status,
                body,
                rate_limit.to_error_suffix()
            );
            let body_preview: String = body.chars().take(120).collect();
            let body_preview = body_preview.trim();
            anyhow::bail!(
                "WB Advert API: {} — {}{}",
                status,
                if body_preview.is_empty() {
                    "(пустой ответ)"
                } else {
                    body_preview
                },
                rate_limit.to_error_suffix()
            );
        }

        let body = self.read_body_tracked(response).await?;
        let body_preview: String = body.chars().take(1000).collect();
        self.log_to_file(&format!("=== RESPONSE BODY ===\n{}\n", body_preview));

        let parsed: WbAdvertCampaignListResponse = match serde_json::from_str(&body) {
            Ok(p) => p,
            Err(e) => {
                let snippet: String = body.chars().take(400).collect();
                tracing::error!(
                    "WB Advert campaign list parse error: {} | body: {}",
                    e,
                    snippet
                );
                anyhow::bail!("Failed to parse WB advert campaign list: {}", e);
            }
        };

        let ids: Vec<i64> = parsed
            .adverts
            .clone()
            .unwrap_or_default()
            .into_iter()
            .flat_map(|g| g.advert_list.into_iter().map(|e| e.advert_id))
            .collect();

        tracing::info!("WB Advert: found {} campaign IDs", ids.len());
        self.log_to_file(&format!("вњ“ Found {} advertIds", ids.len()));

        Ok(ids)
    }

    pub async fn fetch_advert_campaign_summaries(
        &self,
        connection: &ConnectionMP,
    ) -> Result<Vec<WbAdvertCampaignSummary>> {
        let url = "https://advert-api.wildberries.ru/adv/v1/promotion/count";

        if connection.api_key.trim().is_empty() {
            anyhow::bail!("API Key is required for Wildberries Advert API");
        }

        let response = self
            .client
            .get(url)
            .header("Authorization", &connection.api_key)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Connection error for advert campaign list: {}", e))?;

        let status = response.status();
        let rate_limit = WbRateLimitHeaders::from_headers(response.headers());
        self.log_to_file(&format!(
            "WB Advert campaign summaries X-Ratelimit headers: {}",
            rate_limit.to_log_fields()
        ));
        if !status.is_success() {
            let body = self.read_body_tracked(response).await.unwrap_or_default();
            let body_preview: String = body.chars().take(120).collect();
            anyhow::bail!(
                "WB Advert API: {} — {}{}",
                status,
                if body_preview.trim().is_empty() {
                    "(пустой ответ)"
                } else {
                    body_preview.trim()
                },
                rate_limit.to_error_suffix()
            );
        }

        let body = self.read_body_tracked(response).await?;
        let parsed: WbAdvertCampaignListResponse = serde_json::from_str(&body).map_err(|e| {
            let snippet: String = body.chars().take(400).collect();
            anyhow::anyhow!(
                "Failed to parse WB advert campaign list: {} | body: {}",
                e,
                snippet
            )
        })?;

        let mut result = Vec::new();
        for group in parsed.adverts.unwrap_or_default() {
            for entry in group.advert_list {
                result.push(WbAdvertCampaignSummary {
                    advert_id: entry.advert_id,
                    campaign_type: group.campaign_type,
                    status: group.status,
                    change_time: entry.change_time,
                });
            }
        }
        Ok(result)
    }

    /// GET /api/advert/v2/adverts — настройки кампаний, включая места размещения.
    pub async fn fetch_advert_campaigns(
        &self,
        connection: &ConnectionMP,
        ids: &[i64],
    ) -> Result<Vec<WbAdvertCampaign>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        let ids_str = ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let url = format!(
            "https://advert-api.wildberries.ru/api/advert/v2/adverts?ids={}",
            ids_str
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", &connection.api_key)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Connection error for advert campaigns: {}", e))?;

        let status = response.status();
        let rate_limit = WbRateLimitHeaders::from_headers(response.headers());
        self.log_to_file(&format!(
            "WB Advert campaigns X-Ratelimit headers: {}",
            rate_limit.to_log_fields()
        ));
        if !status.is_success() {
            let body = self.read_body_tracked(response).await.unwrap_or_default();
            tracing::warn!(
                "WB Advert campaigns failed: {} - {}{}",
                status,
                body,
                rate_limit.to_error_suffix()
            );
            anyhow::bail!(
                "WB Advert campaigns failed with status {}: {}{}",
                status,
                body,
                rate_limit.to_error_suffix()
            );
        }

        let body = self.read_body_tracked(response).await?;
        let parsed: WbAdvertCampaignsResponse = serde_json::from_str(&body).map_err(|e| {
            let snippet: String = body.chars().take(400).collect();
            anyhow::anyhow!(
                "Failed to parse WB advert campaigns: {} | body: {}",
                e,
                snippet
            )
        })?;

        Ok(parsed.adverts)
    }

    pub async fn fetch_advert_campaign_info_values(
        &self,
        connection: &ConnectionMP,
        ids: &[i64],
    ) -> Result<Vec<serde_json::Value>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        let ids_str = ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let url = format!(
            "https://advert-api.wildberries.ru/api/advert/v2/adverts?ids={}",
            ids_str
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", &connection.api_key)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Connection error for advert campaigns: {}", e))?;

        let status = response.status();
        let rate_limit = WbRateLimitHeaders::from_headers(response.headers());
        self.log_to_file(&format!(
            "WB Advert campaigns info X-Ratelimit headers: {}",
            rate_limit.to_log_fields()
        ));
        if !status.is_success() {
            let body = self.read_body_tracked(response).await.unwrap_or_default();
            anyhow::bail!(
                "WB Advert campaigns failed with status {}: {}{}",
                status,
                body,
                rate_limit.to_error_suffix()
            );
        }

        let body = self.read_body_tracked(response).await?;
        let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
            let snippet: String = body.chars().take(400).collect();
            anyhow::anyhow!(
                "Failed to parse WB advert campaigns info: {} | body: {}",
                e,
                snippet
            )
        })?;

        if let Some(adverts) = parsed.get("adverts").and_then(|v| v.as_array()) {
            Ok(adverts.clone())
        } else if let Some(items) = parsed.as_array() {
            Ok(items.clone())
        } else {
            Ok(vec![parsed])
        }
    }

    /// GET /adv/v3/fullstats вЂ” СЃС‚Р°С‚РёСЃС‚РёРєР° СЂРµРєР»Р°РјРЅС‹С… РєР°РјРїР°РЅРёР№ (РјР°РєСЃ 50 ID Р·Р° Р·Р°РїСЂРѕСЃ)
    pub async fn fetch_advert_fullstats(
        &self,
        connection: &ConnectionMP,
        ids: &[i64],
        begin_date: &str,
        end_date: &str,
    ) -> Result<Vec<WbAdvertFullStat>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        let ids_str = ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let url = format!(
            "https://advert-api.wildberries.ru/adv/v3/fullstats?ids={}&beginDate={}&endDate={}",
            ids_str, begin_date, end_date
        );

        self.log_to_file(&format!(
            "=== REQUEST ===\nGET {}\nAuthorization: ****",
            url
        ));

        let response = match self
            .client
            .get(&url)
            .header("Authorization", &connection.api_key)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("WB Advert fullstats connection error: {}", e);
                anyhow::bail!("Connection error for advert fullstats: {}", e);
            }
        };

        let status = response.status();
        let rate_limit = WbRateLimitHeaders::from_headers(response.headers());
        self.log_to_file(&format!("Response status: {}", status));
        self.log_to_file(&format!(
            "Response X-Ratelimit headers: {}",
            rate_limit.to_log_fields()
        ));

        if !status.is_success() {
            let body = self.read_body_tracked(response).await.unwrap_or_default();
            self.log_to_file(&format!("ERROR Response body:\n{}", body));
            tracing::warn!(
                "WB Advert fullstats failed: {} - {}{}",
                status,
                body,
                rate_limit.to_error_suffix()
            );
            let body_preview: String = body.chars().take(120).collect();
            let body_preview = body_preview.trim();
            anyhow::bail!(
                "WB Advert API fullstats: {} — {}{}",
                status,
                if body_preview.is_empty() {
                    "(пустой ответ)"
                } else {
                    body_preview
                },
                rate_limit.to_error_suffix()
            );
        }

        let body = self.read_body_tracked(response).await?;
        let body_preview: String = body.chars().take(2000).collect();
        self.log_to_file(&format!("=== FULLSTATS RESPONSE ===\n{}\n", body_preview));

        if body.trim() == "null" {
            tracing::info!(
                "WB Advert fullstats returned null for ids=[{}]; treating as empty stats",
                ids_str
            );
            return Ok(Vec::new());
        }

        let parsed: Vec<WbAdvertFullStat> = match serde_json::from_str(&body) {
            Ok(p) => p,
            Err(e) => {
                let snippet: String = body.chars().take(400).collect();
                tracing::error!("WB Advert fullstats parse error: {} | body: {}", e, snippet);
                anyhow::bail!(
                    "Failed to parse WB advert fullstats: {} | body: {}",
                    e,
                    snippet
                );
            }
        };

        tracing::info!(
            "WB Advert fullstats: {} campaigns for ids=[{}]",
            parsed.len(),
            ids_str
        );

        Ok(parsed)
    }
}

impl Default for WildberriesApiClient {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Request/Response structures РґР»СЏ Wildberries API
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WildberriesProductListRequest {
    pub settings: WildberriesSettings,
    pub limit: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WildberriesSettings {
    pub cursor: WildberriesCursor,
    pub filter: WildberriesFilter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WildberriesCursor {
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(rename = "nmID", skip_serializing_if = "Option::is_none")]
    pub nm_id: Option<i64>,
    #[serde(default)]
    pub total: i64,
}

impl Default for WildberriesCursor {
    fn default() -> Self {
        Self {
            updated_at: None,
            nm_id: None,
            total: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WildberriesFilter {
    #[serde(rename = "findByNmID", skip_serializing_if = "Option::is_none")]
    pub find_by_nm_id: Option<Vec<i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WildberriesProductListResponse {
    pub cards: Vec<WildberriesCard>,
    pub cursor: WildberriesCursor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WildberriesCard {
    #[serde(rename = "nmID")]
    pub nm_id: i64,
    #[serde(rename = "imtID")]
    pub imt_id: i64,
    #[serde(rename = "subjectID")]
    pub subject_id: i64,
    #[serde(rename = "vendorCode")]
    pub vendor_code: String,
    #[serde(default)]
    pub brand: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub photos: Vec<WildberriesPhoto>,
    #[serde(default)]
    pub video: Option<String>,
    #[serde(default)]
    pub dimensions: Option<WildberriesDimensions>,
    #[serde(default)]
    pub characteristics: Vec<WildberriesCharacteristic>,
    #[serde(default)]
    pub sizes: Vec<WildberriesSize>,
    #[serde(default)]
    pub tags: Vec<WildberriesTag>,
    #[serde(rename = "createdAt", default)]
    pub created_at: String,
    #[serde(rename = "updatedAt", default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WildberriesPhoto {
    #[serde(default)]
    pub big: Option<String>,
    #[serde(default)]
    pub small: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WildberriesDimensions {
    #[serde(default)]
    pub length: Option<i32>,
    #[serde(default)]
    pub width: Option<i32>,
    #[serde(default)]
    pub height: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WildberriesCharacteristic {
    #[serde(
        rename = "РќР°РёРјРµРЅРѕРІР°РЅРёРµ С…Р°СЂР°РєС‚РµСЂРёСЃС‚РёРєРё",
        default
    )]
    pub name: Option<String>,
    #[serde(rename = "Р—РЅР°С‡РµРЅРёРµ С…Р°СЂР°РєС‚РµСЂРёСЃС‚РёРєРё", default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WildberriesSize {
    #[serde(rename = "techSize", default)]
    pub tech_size: Option<String>,
    #[serde(rename = "wbSize", default)]
    pub wb_size: Option<String>,
    #[serde(default)]
    pub price: Option<i32>,
    #[serde(rename = "discountedPrice", default)]
    pub discounted_price: Option<i32>,
    #[serde(default)]
    pub barcode: Option<String>,
    #[serde(default)]
    pub skus: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WildberriesTag {
    #[serde(default)]
    pub id: Option<i32>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
}

// ============================================================================
// Sales structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSaleRow {
    /// РЈРЅРёРєР°Р»СЊРЅС‹Р№ РёРґРµРЅС‚РёС„РёРєР°С‚РѕСЂ СЃС‚СЂРѕРєРё РїСЂРѕРґР°Р¶Рё
    #[serde(default)]
    pub srid: Option<String>,
    /// РќРѕРјРµРЅРєР»Р°С‚СѓСЂРЅС‹Р№ РЅРѕРјРµСЂ С‚РѕРІР°СЂР°
    #[serde(rename = "nmId", default)]
    pub nm_id: Option<i64>,
    /// РђСЂС‚РёРєСѓР» РїСЂРѕРґР°РІС†Р°
    #[serde(rename = "supplierArticle", default)]
    pub supplier_article: Option<String>,
    /// РЁС‚СЂРёС…РєРѕРґ
    #[serde(default)]
    pub barcode: Option<String>,
    /// РќР°Р·РІР°РЅРёРµ С‚РѕРІР°СЂР°
    #[serde(default)]
    pub brand: Option<String>,
    /// РџСЂРµРґРјРµС‚
    #[serde(default)]
    pub subject: Option<String>,
    /// РљР°С‚РµРіРѕСЂРёСЏ
    #[serde(default)]
    pub category: Option<String>,
    /// Р”Р°С‚Р° РїСЂРѕРґР°Р¶Рё
    #[serde(rename = "date", default)]
    pub sale_dt: Option<String>,
    /// Р”Р°С‚Р° РїРѕСЃР»РµРґРЅРµРіРѕ РёР·РјРµРЅРµРЅРёСЏ Р·Р°РїРёСЃРё
    #[serde(rename = "lastChangeDate", default)]
    pub last_change_date: Option<String>,
    /// РЎРєР»Р°Рґ
    #[serde(rename = "warehouseName", default)]
    pub warehouse_name: Option<String>,
    /// РЎС‚СЂР°РЅР°
    #[serde(rename = "countryName", default)]
    pub country_name: Option<String>,
    /// Р РµРіРёРѕРЅ
    #[serde(rename = "oblastOkrugName", default)]
    pub region_name: Option<String>,
    /// Р¦РµРЅР° Р±РµР· СЃРєРёРґРєРё
    #[serde(rename = "priceWithDisc", default)]
    pub price_with_disc: Option<f64>,
    /// РЎРєРёРґРєР° РїСЂРѕРґР°РІС†Р°
    #[serde(rename = "discount", default)]
    pub discount: Option<f64>,
    /// РљРѕР»РёС‡РµСЃС‚РІРѕ
    #[serde(rename = "quantity", default)]
    pub quantity: Option<i32>,
    /// РўРёРї РґРѕРєСѓРјРµРЅС‚Р°: sale РёР»Рё return
    #[serde(rename = "saleID", default)]
    pub sale_id: Option<String>,
    /// РќРѕРјРµСЂ Р·Р°РєР°Р·Р°
    #[serde(rename = "odid", default)]
    pub order_id: Option<i64>,
    /// SPP (РЎРѕРіР»Р°СЃРѕРІР°РЅРЅР°СЏ СЃРєРёРґРєР° РїСЂРѕРґР°РІС†Р°)
    #[serde(rename = "spp", default)]
    pub spp: Option<f64>,
    /// Р’РѕР·РЅР°РіСЂР°Р¶РґРµРЅРёРµ
    #[serde(rename = "forPay", default)]
    pub for_pay: Option<f64>,
    /// РС‚РѕРіРѕРІР°СЏ СЃС‚РѕРёРјРѕСЃС‚СЊ
    #[serde(rename = "finishedPrice", default)]
    pub finished_price: Option<f64>,
    /// Р¤Р»Р°Рі РїРѕСЃС‚Р°РІРєРё
    #[serde(rename = "isSupply", default)]
    pub is_supply: Option<bool>,
    /// Р¤Р»Р°Рі СЂРµР°Р»РёР·Р°С†РёРё
    #[serde(rename = "isRealization", default)]
    pub is_realization: Option<bool>,
    /// РџРѕР»РЅР°СЏ С†РµРЅР°
    #[serde(rename = "totalPrice", default)]
    pub total_price: Option<f64>,
    /// РџСЂРѕС†РµРЅС‚ СЃРєРёРґРєРё
    #[serde(rename = "discountPercent", default)]
    pub discount_percent: Option<f64>,
    /// РЎСѓРјРјР° РїР»Р°С‚РµР¶Р° Р·Р° РїСЂРѕРґР°Р¶Сѓ
    #[serde(rename = "paymentSaleAmount", default)]
    pub payment_sale_amount: Option<f64>,
    /// РўРёРї СЃРєР»Р°РґР°
    #[serde(rename = "warehouseType", default)]
    pub warehouse_type: Option<String>,
}

// ============================================================================
// Finance Report structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbFinanceReportRow {
    /// ID СЃС‚СЂРѕРєРё РѕС‚С‡РµС‚Р°
    #[serde(default)]
    pub rrd_id: Option<i64>,
    /// Р”Р°С‚Р° СЃС‚СЂРѕРєРё С„РёРЅР°РЅСЃРѕРІРѕРіРѕ РѕС‚С‡С‘С‚Р°
    #[serde(default)]
    pub rr_dt: Option<String>,
    /// РќРѕРјРµРЅРєР»Р°С‚СѓСЂРЅС‹Р№ РЅРѕРјРµСЂ С‚РѕРІР°СЂР°
    #[serde(default)]
    pub nm_id: Option<i64>,
    /// РђСЂС‚РёРєСѓР» РїСЂРѕРґР°РІС†Р°
    #[serde(default)]
    pub sa_name: Option<String>,
    /// РљР°С‚РµРіРѕСЂРёСЏ С‚РѕРІР°СЂР°
    #[serde(default)]
    pub subject_name: Option<String>,
    /// РўРёРї РѕРїРµСЂР°С†РёРё РїРѕ Р·Р°РєР°Р·Сѓ
    #[serde(default)]
    pub supplier_oper_name: Option<String>,
    /// РљРѕР»РёС‡РµСЃС‚РІРѕ С‚РѕРІР°СЂРѕРІ
    #[serde(default)]
    pub quantity: Option<i32>,
    /// Р РѕР·РЅРёС‡РЅР°СЏ С†РµРЅР° Р·Р° РµРґРёРЅРёС†Сѓ С‚РѕРІР°СЂР°
    #[serde(default)]
    pub retail_price: Option<f64>,
    /// РћР±С‰Р°СЏ СЃСѓРјРјР° РїСЂРѕРґР°Р¶Рё
    #[serde(default)]
    pub retail_amount: Option<f64>,
    /// Р¦РµРЅР° РїСЂРѕРґР°Р¶Рё СЃ СѓС‡РµС‚РѕРј СЃРєРёРґРѕРє
    #[serde(default)]
    pub retail_price_withdisc_rub: Option<f64>,
    /// РџСЂРѕС†РµРЅС‚ РєРѕРјРёСЃСЃРёРё Wildberries
    #[serde(default)]
    pub commission_percent: Option<f64>,
    /// РљРѕРјРёСЃСЃРёСЏ Р·Р° СЌРєРІР°Р№СЂРёРЅРі
    #[serde(default)]
    pub acquiring_fee: Option<f64>,
    /// РџСЂРѕС†РµРЅС‚ РєРѕРјРёСЃСЃРёРё Р·Р° СЌРєРІР°Р№СЂРёРЅРі
    #[serde(default)]
    pub acquiring_percent: Option<f64>,
    /// РЎСѓРјРјР°, СѓРїР»Р°С‡РµРЅРЅР°СЏ РїРѕРєСѓРїР°С‚РµР»РµРј Р·Р° РґРѕСЃС‚Р°РІРєСѓ
    #[serde(default)]
    pub delivery_amount: Option<f64>,
    /// РЎС‚РѕРёРјРѕСЃС‚СЊ РґРѕСЃС‚Р°РІРєРё РЅР° СЃС‚РѕСЂРѕРЅРµ РїСЂРѕРґР°РІС†Р°
    #[serde(default)]
    pub delivery_rub: Option<f64>,
    /// РЎСѓРјРјР° РІРѕР·РЅР°РіСЂР°Р¶РґРµРЅРёСЏ Р’Р°Р№Р»РґР±РµСЂСЂРёР· Р·Р° С‚РµРєСѓС‰РёР№ РїРµСЂРёРѕРґ (Р’Р’), Р±РµР· РќР”РЎ
    #[serde(default)]
    pub ppvz_vw: Option<f64>,
    /// РќР”РЎ СЃ РІРѕР·РЅР°РіСЂР°Р¶РґРµРЅРёСЏ Р’Р°Р№Р»РґР±РµСЂСЂРёР·
    #[serde(default)]
    pub ppvz_vw_nds: Option<f64>,
    /// РљРѕРјРёСЃСЃРёСЏ WB Р·Р° РїСЂРѕРґР°Р¶Сѓ
    #[serde(default)]
    pub ppvz_sales_commission: Option<f64>,
    /// РЎСѓРјРјР° РІРѕР·РІСЂР°С‚Р° Р·Р° РІРѕР·РІСЂР°С‰С‘РЅРЅС‹Рµ С‚РѕРІР°СЂС‹
    #[serde(default)]
    pub return_amount: Option<f64>,
    /// РЎСѓРјРјР° С€С‚СЂР°С„Р°, СѓРґРµСЂР¶Р°РЅРЅРѕРіРѕ СЃ РїСЂРѕРґР°РІС†Р°
    #[serde(default)]
    pub penalty: Option<f64>,
    /// Р”РѕРїРѕР»РЅРёС‚РµР»СЊРЅС‹Рµ (РєРѕСЂСЂРµРєС‚РёСЂСѓСЋС‰РёРµ) РІС‹РїР»Р°С‚С‹ РїСЂРѕРґР°РІС†Сѓ
    #[serde(default)]
    pub additional_payment: Option<f64>,
    /// РџР»Р°С‚Р° Р·Р° С…СЂР°РЅРµРЅРёРµ С‚РѕРІР°СЂРѕРІ РЅР° СЃРєР»Р°РґРµ
    #[serde(default)]
    pub storage_fee: Option<f64>,
    /// РЎРєРѕСЂСЂРµРєС‚РёСЂРѕРІР°РЅРЅС‹Рµ СЂР°СЃС…РѕРґС‹ РЅР° Р»РѕРіРёСЃС‚РёРєСѓ
    #[serde(default)]
    pub rebill_logistic_cost: Option<f64>,
    /// РўРёРї Р±РѕРЅСѓСЃР° РёР»Рё С€С‚СЂР°С„Р°
    #[serde(default)]
    pub bonus_type_name: Option<String>,
    /// РўРёРї РѕС‚С‡РµС‚Р° (1 = daily, 2 = weekly)
    #[serde(default)]
    pub report_type: Option<i32>,

    // ============ Р”РѕРїРѕР»РЅРёС‚РµР»СЊРЅС‹Рµ РїРѕР»СЏ РёР· API (РґР»СЏ РїРѕР»РЅРѕРіРѕ JSON) ============
    /// ID СЂРµР°Р»РёР·Р°С†РёРѕРЅРЅРѕРіРѕ РѕС‚С‡РµС‚Р°
    #[serde(default)]
    pub realizationreport_id: Option<i64>,
    /// Р”Р°С‚Р° РЅР°С‡Р°Р»Р° РїРµСЂРёРѕРґР° РѕС‚С‡РµС‚Р°
    #[serde(default)]
    pub date_from: Option<String>,
    /// Р”Р°С‚Р° РѕРєРѕРЅС‡Р°РЅРёСЏ РїРµСЂРёРѕРґР° РѕС‚С‡РµС‚Р°
    #[serde(default)]
    pub date_to: Option<String>,
    /// Р”Р°С‚Р° СЃРѕР·РґР°РЅРёСЏ РѕС‚С‡РµС‚Р°
    #[serde(default)]
    pub create_dt: Option<String>,
    /// Р’Р°Р»СЋС‚Р°
    #[serde(default)]
    pub currency_name: Option<String>,
    /// РљРѕРґ РґРѕРіРѕРІРѕСЂР° РїРѕСЃС‚Р°РІС‰РёРєР°
    #[serde(default)]
    pub suppliercontract_code: Option<String>,
    /// ID СЃР±РѕСЂРѕС‡РЅРѕРіРѕ Р·Р°РґР°РЅРёСЏ
    #[serde(default)]
    pub gi_id: Option<i64>,
    /// РџСЂРѕС†РµРЅС‚ РґРѕСЃС‚Р°РІРєРё
    #[serde(default)]
    pub dlv_prc: Option<f64>,
    /// Р”Р°С‚Р° РЅР°С‡Р°Р»Р° РґРµР№СЃС‚РІРёСЏ С„РёРєСЃ. С‚Р°СЂРёС„Р°
    #[serde(default)]
    pub fix_tariff_date_from: Option<String>,
    /// Р”Р°С‚Р° РѕРєРѕРЅС‡Р°РЅРёСЏ РґРµР№СЃС‚РІРёСЏ С„РёРєСЃ. С‚Р°СЂРёС„Р°
    #[serde(default)]
    pub fix_tariff_date_to: Option<String>,
    /// Р‘СЂРµРЅРґ С‚РѕРІР°СЂР°
    #[serde(default)]
    pub brand_name: Option<String>,
    /// Р Р°Р·РјРµСЂ С‚РѕРІР°СЂР°
    #[serde(default)]
    pub ts_name: Option<String>,
    /// РЁС‚СЂРёС…РєРѕРґ С‚РѕРІР°СЂР°
    #[serde(default)]
    pub barcode: Option<String>,
    /// РўРёРї РґРѕРєСѓРјРµРЅС‚Р°
    #[serde(default)]
    pub doc_type_name: Option<String>,
    /// РџСЂРѕС†РµРЅС‚ СЃРєРёРґРєРё
    #[serde(default)]
    pub sale_percent: Option<f64>,
    /// РќР°Р·РІР°РЅРёРµ СЃРєР»Р°РґР°
    #[serde(default)]
    pub office_name: Option<String>,
    /// Р”Р°С‚Р° Р·Р°РєР°Р·Р°
    #[serde(default)]
    pub order_dt: Option<String>,
    /// Р”Р°С‚Р° РїСЂРѕРґР°Р¶Рё
    #[serde(default)]
    pub sale_dt: Option<String>,
    /// ID РїРѕСЃС‚Р°РІРєРё
    #[serde(default)]
    pub shk_id: Option<i64>,
    /// РўРёРї РєРѕСЂРѕР±РѕРІ
    #[serde(default)]
    pub gi_box_type_name: Option<String>,
    /// РЎРєРёРґРєР° РЅР° С‚РѕРІР°СЂ РґР»СЏ РѕС‚С‡РµС‚Р°
    #[serde(default)]
    pub product_discount_for_report: Option<f64>,
    /// РџСЂРѕРјРѕ РїРѕСЃС‚Р°РІС‰РёРєР°
    #[serde(default)]
    pub supplier_promo: Option<f64>,
    /// РЎРѕРіР»Р°СЃРѕРІР°РЅРЅР°СЏ СЃРєРёРґРєР° РїСЂРѕРґР°РІС†Р°
    #[serde(default)]
    pub ppvz_spp_prc: Option<f64>,
    /// Р‘Р°Р·РѕРІС‹Р№ РїСЂРѕС†РµРЅС‚ РєРѕРјРёСЃСЃРёРё
    #[serde(default)]
    pub ppvz_kvw_prc_base: Option<f64>,
    /// РџСЂРѕС†РµРЅС‚ РєРѕРјРёСЃСЃРёРё
    #[serde(default)]
    pub ppvz_kvw_prc: Option<f64>,
    /// РџСЂРѕС†РµРЅС‚ РїРѕРІС‹С€РµРЅРёСЏ СЂРµР№С‚РёРЅРіР° РїРѕСЃС‚Р°РІС‰РёРєР°
    #[serde(default)]
    pub sup_rating_prc_up: Option<f64>,
    /// РЈС‡Р°СЃС‚РёРµ РІ РљР“Р’Рџ v2
    #[serde(default)]
    pub is_kgvp_v2: Option<i32>,
    /// Рљ РїРµСЂРµС‡РёСЃР»РµРЅРёСЋ Р·Р° С‚РѕРІР°СЂ
    #[serde(default)]
    pub ppvz_for_pay: Option<f64>,
    /// Р’РѕР·РЅР°РіСЂР°Р¶РґРµРЅРёРµ
    #[serde(default)]
    pub ppvz_reward: Option<f64>,
    /// РўРёРї РїСЂРѕС†РµСЃСЃРёРЅРіР° РїР»Р°С‚РµР¶Р°
    #[serde(default)]
    pub payment_processing: Option<String>,
    /// Р‘Р°РЅРє-СЌРєРІР°Р№РµСЂ
    #[serde(default)]
    pub acquiring_bank: Option<String>,
    /// РќР°Р·РІР°РЅРёРµ РїСѓРЅРєС‚Р° РІС‹РґР°С‡Рё
    #[serde(default)]
    pub ppvz_office_name: Option<String>,
    /// ID РїСѓРЅРєС‚Р° РІС‹РґР°С‡Рё
    #[serde(default)]
    pub ppvz_office_id: Option<i64>,
    /// ID РїРѕСЃС‚Р°РІС‰РёРєР°
    #[serde(default)]
    pub ppvz_supplier_id: Option<i64>,
    /// РќР°Р·РІР°РЅРёРµ РїРѕСЃС‚Р°РІС‰РёРєР°
    #[serde(default)]
    pub ppvz_supplier_name: Option<String>,
    /// РРќРќ РїРѕСЃС‚Р°РІС‰РёРєР°
    #[serde(default)]
    pub ppvz_inn: Option<String>,
    /// РќРѕРјРµСЂ РґРµРєР»Р°СЂР°С†РёРё
    #[serde(default)]
    pub declaration_number: Option<String>,
    /// ID СЃС‚РёРєРµСЂР°
    #[serde(default)]
    pub sticker_id: Option<String>,
    /// РЎС‚СЂР°РЅР° РїСЂРѕРґР°Р¶Рё
    #[serde(default)]
    pub site_country: Option<String>,
    /// Р”РѕСЃС‚Р°РІРєР° СЃРёР»Р°РјРё РїСЂРѕРґР°РІС†Р°
    #[serde(default)]
    pub srv_dbs: Option<bool>,
    /// РћСЂРіР°РЅРёР·Р°С†РёСЏ, РїСЂРµРґРѕСЃС‚Р°РІРёРІС€Р°СЏ Р»РѕРіРёСЃС‚РёРєСѓ
    #[serde(default)]
    pub rebill_logistic_org: Option<String>,
    /// РЈРґРµСЂР¶Р°РЅРёСЏ
    #[serde(default)]
    pub deduction: Option<f64>,
    /// РџСЂРёРµРјРєР°
    #[serde(default)]
    pub acceptance: Option<f64>,
    /// ID СЃР±РѕСЂРѕС‡РЅРѕРіРѕ Р·Р°РґР°РЅРёСЏ
    #[serde(default)]
    pub assembly_id: Option<i64>,
    /// РљРѕРґ РјР°СЂРєРёСЂРѕРІРєРё
    #[serde(default)]
    pub kiz: Option<String>,
    /// РЈРЅРёРєР°Р»СЊРЅС‹Р№ РёРґРµРЅС‚РёС„РёРєР°С‚РѕСЂ СЃС‚СЂРѕРєРё
    #[serde(default)]
    pub srid: Option<String>,
    /// Р®СЂРёРґРёС‡РµСЃРєРѕРµ Р»РёС†Рѕ
    #[serde(default)]
    pub is_legal_entity: Option<bool>,
    /// ID РІРѕР·РІСЂР°С‚Р°
    #[serde(default)]
    pub trbx_id: Option<String>,
    /// РЎСѓРјРјР° СЃРѕС„РёРЅР°РЅСЃРёСЂРѕРІР°РЅРёСЏ СЂР°СЃСЃСЂРѕС‡РєРё
    #[serde(default)]
    pub installment_cofinancing_amount: Option<f64>,
    /// РџСЂРѕС†РµРЅС‚ СЃРєРёРґРєРё WiBES
    #[serde(default)]
    pub wibes_wb_discount_percent: Option<f64>,
    /// РЎСѓРјРјР° РєСЌС€Р±СЌРєР°
    #[serde(default)]
    pub cashback_amount: Option<f64>,
    /// РЎРєРёРґРєР° РїРѕ РєСЌС€Р±СЌРєСѓ
    #[serde(default)]
    pub cashback_discount: Option<f64>,
    /// РР·РјРµРЅРµРЅРёРµ РєРѕРјРёСЃСЃРёРё РїРѕ РєСЌС€Р±СЌРєСѓ
    #[serde(default)]
    pub cashback_commission_change: Option<f64>,
    /// РЈРЅРёРєР°Р»СЊРЅС‹Р№ ID Р·Р°РєР°Р·Р°
    #[serde(default)]
    pub order_uid: Option<String>,
}

// ============================================================================
// Orders structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbOrderRow {
    /// Р”Р°С‚Р° Р·Р°РєР°Р·Р°
    #[serde(default)]
    pub date: Option<String>,
    /// Р”Р°С‚Р° РїРѕСЃР»РµРґРЅРµРіРѕ РёР·РјРµРЅРµРЅРёСЏ
    #[serde(rename = "lastChangeDate", default)]
    pub last_change_date: Option<String>,
    /// РќР°Р·РІР°РЅРёРµ СЃРєР»Р°РґР°
    #[serde(rename = "warehouseName", default)]
    pub warehouse_name: Option<String>,
    /// РўРёРї СЃРєР»Р°РґР°
    #[serde(rename = "warehouseType", default)]
    pub warehouse_type: Option<String>,
    /// РќР°Р·РІР°РЅРёРµ СЃС‚СЂР°РЅС‹
    #[serde(rename = "countryName", default)]
    pub country_name: Option<String>,
    /// РќР°Р·РІР°РЅРёРµ РѕР±Р»Р°СЃС‚Рё/РѕРєСЂСѓРіР°
    #[serde(rename = "oblastOkrugName", default)]
    pub oblast_okrug_name: Option<String>,
    /// РќР°Р·РІР°РЅРёРµ СЂРµРіРёРѕРЅР°
    #[serde(rename = "regionName", default)]
    pub region_name: Option<String>,
    /// РђСЂС‚РёРєСѓР» РїСЂРѕРґР°РІС†Р°
    #[serde(rename = "supplierArticle", default)]
    pub supplier_article: Option<String>,
    /// nmId (ID РЅРѕРјРµРЅРєР»Р°С‚СѓСЂС‹ WB)
    #[serde(rename = "nmId", default)]
    pub nm_id: Option<i64>,
    /// Р‘Р°СЂРєРѕРґ
    #[serde(default)]
    pub barcode: Option<String>,
    /// РљР°С‚РµРіРѕСЂРёСЏ
    #[serde(default)]
    pub category: Option<String>,
    /// РџСЂРµРґРјРµС‚
    #[serde(default)]
    pub subject: Option<String>,
    /// Р‘СЂРµРЅРґ
    #[serde(default)]
    pub brand: Option<String>,
    /// Р Р°Р·РјРµСЂ
    #[serde(rename = "techSize", default)]
    pub tech_size: Option<String>,
    /// РќРѕРјРµСЂ РїРѕСЃС‚Р°РІРєРё
    #[serde(rename = "incomeID", default)]
    pub income_id: Option<i64>,
    /// Р¤Р»Р°Рі РїРѕСЃС‚Р°РІРєРё
    #[serde(rename = "isSupply", default)]
    pub is_supply: Option<bool>,
    /// Р¤Р»Р°Рі СЂРµР°Р»РёР·Р°С†РёРё
    #[serde(rename = "isRealization", default)]
    pub is_realization: Option<bool>,
    /// Р¦РµРЅР° Р±РµР· СЃРєРёРґРєРё
    #[serde(rename = "totalPrice", default)]
    pub total_price: Option<f64>,
    /// РџСЂРѕС†РµРЅС‚ СЃРєРёРґРєРё
    #[serde(rename = "discountPercent", default)]
    pub discount_percent: Option<f64>,
    /// SPP (РЎРѕРіР»Р°СЃРѕРІР°РЅРЅР°СЏ СЃРєРёРґРєР° РїСЂРѕРґР°РІС†Р°)
    #[serde(default)]
    pub spp: Option<f64>,
    /// РС‚РѕРіРѕРІР°СЏ С†РµРЅР° РґР»СЏ РєР»РёРµРЅС‚Р°
    #[serde(rename = "finishedPrice", default)]
    pub finished_price: Option<f64>,
    /// Р¦РµРЅР° СЃ СѓС‡РµС‚РѕРј СЃРєРёРґРєРё
    #[serde(rename = "priceWithDisc", default)]
    pub price_with_disc: Option<f64>,
    /// Р¤Р»Р°Рі РѕС‚РјРµРЅС‹ Р·Р°РєР°Р·Р°
    #[serde(rename = "isCancel", default)]
    pub is_cancel: Option<bool>,
    /// Р”Р°С‚Р° РѕС‚РјРµРЅС‹
    #[serde(rename = "cancelDate", default)]
    pub cancel_date: Option<String>,
    /// ID СЃС‚РёРєРµСЂР°
    #[serde(default)]
    pub sticker: Option<String>,
    /// G-РЅРѕРјРµСЂ
    #[serde(rename = "gNumber", default)]
    pub g_number: Option<String>,
    /// SRID - СѓРЅРёРєР°Р»СЊРЅС‹Р№ РёРґРµРЅС‚РёС„РёРєР°С‚РѕСЂ Р·Р°РєР°Р·Р°
    #[serde(default)]
    pub srid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WbDocumentListItem {
    pub service_name: String,
    pub name: String,
    pub category: String,
    pub extensions: Vec<String>,
    pub creation_time: String,
    pub viewed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbDocumentsListData {
    pub documents: Vec<WbDocumentListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbDocumentsListResponse {
    pub data: WbDocumentsListData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WbDocumentDownloadFile {
    pub file_name: String,
    pub extension: String,
    pub document: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbDocumentDownloadResponse {
    pub data: WbDocumentDownloadFile,
}

// ============================================================================
// Commission Tariffs structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissionTariffRow {
    #[serde(rename = "kgvpBooking")]
    pub kgvp_booking: f64,
    #[serde(rename = "kgvpMarketplace")]
    pub kgvp_marketplace: f64,
    #[serde(rename = "kgvpPickup")]
    pub kgvp_pickup: f64,
    #[serde(rename = "kgvpSupplier")]
    pub kgvp_supplier: f64,
    #[serde(rename = "kgvpSupplierExpress")]
    pub kgvp_supplier_express: f64,
    #[serde(rename = "paidStorageKgvp")]
    pub paid_storage_kgvp: f64,
    #[serde(rename = "parentID")]
    pub parent_id: i32,
    #[serde(rename = "parentName")]
    pub parent_name: String,
    #[serde(rename = "subjectID")]
    pub subject_id: i32,
    #[serde(rename = "subjectName")]
    pub subject_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissionTariffResponse {
    pub report: Vec<CommissionTariffRow>,
}

// ============================================================================
// Diagnostic structures
// ============================================================================

#[derive(Debug, Clone)]
pub struct DiagnosticResult {
    pub test_name: String,
    pub success: bool,
    pub error: Option<String>,
    pub total_returned: i32,
    pub cursor_total: i32,
    pub response_headers: Option<String>,
}

// ============================================================================
// WB Prices API structures (GET /api/v2/list/goods/filter)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbGoodsPriceFilterResponse {
    #[serde(default)]
    pub data: Option<WbGoodsPriceData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbGoodsPriceData {
    #[serde(rename = "listGoods", default)]
    pub list_goods: Vec<WbGoodsPriceRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbGoodsPriceRow {
    #[serde(rename = "nmID")]
    pub nm_id: i64,
    #[serde(rename = "vendorCode", default)]
    pub vendor_code: Option<String>,
    #[serde(default)]
    pub discount: Option<i32>,
    #[serde(rename = "editableSizePrice", default)]
    pub editable_size_price: bool,
    #[serde(default)]
    pub sizes: Vec<WbGoodsSize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbGoodsSize {
    #[serde(rename = "sizeID", default)]
    pub size_id: Option<i64>,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(rename = "discountedPrice", default)]
    pub discounted_price: Option<f64>,
    #[serde(rename = "techSizeName", default)]
    pub tech_size_name: Option<String>,
}

// ============================================================================
// WB Calendar Promotions API structures
// ============================================================================

/// РћС‚РІРµС‚ GET /api/v1/calendar/promotions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbCalendarPromotionsResponse {
    #[serde(default)]
    pub data: Option<WbCalendarPromotionsData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbCalendarPromotionsData {
    #[serde(default)]
    pub promotions: Vec<WbCalendarPromotion>,
    #[serde(rename = "upcomingPromos", default)]
    pub upcoming_promos: Vec<WbCalendarPromotion>,
}

/// РћРґРЅР° Р°РєС†РёСЏ РёР· WB Calendar API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbCalendarPromotion {
    /// WB РёСЃРїРѕР»СЊР·СѓРµС‚ РїРѕР»Рµ "id" (РЅРµ "promotionID")
    pub id: i64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "startDateTime", default)]
    pub start_date_time: Option<String>,
    #[serde(rename = "endDateTime", default)]
    pub end_date_time: Option<String>,
    /// РўРёРї Р°РєС†РёРё: "auto", "regular", etc.
    #[serde(rename = "type", default)]
    pub promotion_type: Option<String>,
    #[serde(rename = "exceptionProductsCount", default)]
    pub exception_products_count: Option<i32>,
    #[serde(rename = "inPromoActionTotal", default)]
    pub in_promo_action_total: Option<i32>,
}

/// РћС‚РІРµС‚ GET /api/v1/calendar/promotions/details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbCalendarPromotionDetailsResponse {
    #[serde(default)]
    pub data: Option<WbCalendarPromotionDetailsData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbCalendarPromotionDetailsData {
    #[serde(default)]
    pub promotions: Vec<WbCalendarPromotionDetail>,
}

/// Р”РµС‚Р°Р»СЊРЅС‹Рµ РґР°РЅРЅС‹Рµ Р°РєС†РёРё РёР· /details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbCalendarPromotionDetail {
    pub id: i64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub advantages: Vec<String>,
    #[serde(rename = "startDateTime", default)]
    pub start_date_time: Option<String>,
    #[serde(rename = "endDateTime", default)]
    pub end_date_time: Option<String>,
    #[serde(rename = "inPromoActionLeftovers", default)]
    pub in_promo_action_leftovers: Option<i32>,
    #[serde(rename = "inPromoActionTotal", default)]
    pub in_promo_action_total: Option<i32>,
    #[serde(rename = "notInPromoActionLeftovers", default)]
    pub not_in_promo_action_leftovers: Option<i32>,
    #[serde(rename = "notInPromoActionTotal", default)]
    pub not_in_promo_action_total: Option<i32>,
    #[serde(rename = "participationPercentage", default)]
    pub participation_percentage: Option<f64>,
    #[serde(rename = "type", default)]
    pub promotion_type: Option<String>,
    #[serde(rename = "exceptionProductsCount", default)]
    pub exception_products_count: Option<i32>,
    #[serde(default)]
    pub ranging: Vec<WbPromotionRanging>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbPromotionRanging {
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(rename = "participationRate", default)]
    pub participation_rate: Option<f64>,
    #[serde(default)]
    pub boost: Option<f64>,
}

/// РћС‚РІРµС‚ GET /api/v1/calendar/promotions/nomenclatures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbPromotionNomenclaturesResponse {
    #[serde(default)]
    pub data: Option<WbPromotionNomenclaturesData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbPromotionNomenclaturesData {
    #[serde(default)]
    pub nomenclatures: Vec<WbPromotionNmItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbPromotionNmItem {
    /// API РІРѕР·РІСЂР°С‰Р°РµС‚ РїРѕР»Рµ "id" (СЌС‚Рѕ nmId С‚РѕРІР°СЂР°)
    #[serde(rename = "id")]
    pub nm_id: i64,
    #[serde(rename = "inAction", default)]
    pub in_action: bool,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(rename = "planPrice", default)]
    pub plan_price: Option<f64>,
    #[serde(default)]
    pub discount: Option<f64>,
    #[serde(rename = "planDiscount", default)]
    pub plan_discount: Option<f64>,
}

// ============================================================================
// WB Advertising Campaigns API structures (/adv/v3/fullstats)
// ============================================================================

/// РћС‚РІРµС‚ GET /adv/v1/promotion/count вЂ” СЃРїРёСЃРѕРє СЂРµРєР»Р°РјРЅС‹С… РєР°РјРїР°РЅРёР№ РїРѕ С‚РёРїСѓ/СЃС‚Р°С‚СѓСЃСѓ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbAdvertCampaignListResponse {
    #[serde(default)]
    pub adverts: Option<Vec<WbAdvertCampaignGroup>>,
    #[serde(default)]
    pub all: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbAdvertCampaignGroup {
    #[serde(rename = "type", default)]
    pub campaign_type: Option<i32>,
    #[serde(default)]
    pub status: Option<i32>,
    #[serde(default)]
    pub count: Option<i32>,
    #[serde(rename = "advert_list", default)]
    pub advert_list: Vec<WbAdvertCampaignEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbAdvertCampaignEntry {
    #[serde(rename = "advertId")]
    pub advert_id: i64,
    #[serde(rename = "changeTime", default)]
    pub change_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbAdvertCampaignSummary {
    pub advert_id: i64,
    pub campaign_type: Option<i32>,
    pub status: Option<i32>,
    pub change_time: Option<String>,
}

/// РЎС‚Р°С‚РёСЃС‚РёРєР° РЅР° СѓСЂРѕРІРЅРµ РѕРґРЅРѕРіРѕ С‚РѕРІР°СЂР° (nmId) РІРЅСѓС‚СЂРё РґРЅСЏ Рё С‚РёРїР° РїСЂРёР»РѕР¶РµРЅРёСЏ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbAdvertCampaignsResponse {
    #[serde(default)]
    pub adverts: Vec<WbAdvertCampaign>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbAdvertCampaign {
    pub id: i64,
    #[serde(default)]
    pub settings: WbAdvertCampaignSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WbAdvertCampaignSettings {
    #[serde(default)]
    pub placements: WbAdvertCampaignPlacements,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WbAdvertCampaignPlacements {
    #[serde(default)]
    pub search: bool,
    #[serde(default)]
    pub recommendations: bool,
}

/// РЎС‚Р°С‚РёСЃС‚РёРєР° РЅР° СѓСЂРѕРІРЅРµ РѕРґРЅРѕРіРѕ С‚РѕРІР°СЂР° (nmId) РІРЅСѓС‚СЂРё РґРЅСЏ Рё С‚РёРїР° РїСЂРёР»РѕР¶РµРЅРёСЏ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbAdvertFullStatNm {
    #[serde(rename = "nmId")]
    pub nm_id: i64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub views: i64,
    #[serde(default)]
    pub clicks: i64,
    #[serde(default)]
    pub ctr: f64,
    #[serde(default)]
    pub cpc: f64,
    #[serde(default)]
    pub atbs: i64,
    #[serde(default)]
    pub orders: i64,
    #[serde(default)]
    pub shks: i64,
    #[serde(default)]
    pub sum: f64,
    #[serde(rename = "sum_price", default)]
    pub sum_price: f64,
    #[serde(default)]
    pub cr: f64,
    #[serde(default)]
    pub canceled: i64,
}

/// РЎС‚Р°С‚РёСЃС‚РёРєР° РїРѕ С‚РёРїСѓ РїСЂРёР»РѕР¶РµРЅРёСЏ (appType: 1=iOS, 32=Android, 64=Web)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbAdvertFullStatApp {
    #[serde(rename = "appType")]
    pub app_type: i32,
    #[serde(default)]
    pub nms: Vec<WbAdvertFullStatNm>,
    #[serde(default)]
    pub views: i64,
    #[serde(default)]
    pub clicks: i64,
    #[serde(default)]
    pub ctr: f64,
    #[serde(default)]
    pub cpc: f64,
    #[serde(default)]
    pub atbs: i64,
    #[serde(default)]
    pub orders: i64,
    #[serde(default)]
    pub shks: i64,
    #[serde(default)]
    pub sum: f64,
    #[serde(rename = "sum_price", default)]
    pub sum_price: f64,
    #[serde(default)]
    pub cr: f64,
    #[serde(default)]
    pub canceled: i64,
}

/// РЎС‚Р°С‚РёСЃС‚РёРєР° Р·Р° РѕРґРёРЅ РґРµРЅСЊ РїРѕ РєР°РјРїР°РЅРёРё
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbAdvertFullStatDay {
    pub date: String,
    #[serde(default)]
    pub apps: Vec<WbAdvertFullStatApp>,
    #[serde(default)]
    pub views: i64,
    #[serde(default)]
    pub clicks: i64,
    #[serde(default)]
    pub ctr: f64,
    #[serde(default)]
    pub cpc: f64,
    #[serde(default)]
    pub atbs: i64,
    #[serde(default)]
    pub orders: i64,
    #[serde(default)]
    pub shks: i64,
    #[serde(default)]
    pub sum: f64,
    #[serde(rename = "sum_price", default)]
    pub sum_price: f64,
    #[serde(default)]
    pub cr: f64,
    #[serde(default)]
    pub canceled: i64,
}

/// РЎРІРѕРґРЅР°СЏ СЃС‚Р°С‚РёСЃС‚РёРєР° РїРѕ РѕРґРЅРѕР№ СЂРµРєР»Р°РјРЅРѕР№ РєР°РјРїР°РЅРёРё Р·Р° РїРµСЂРёРѕРґ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbAdvertFullStat {
    #[serde(rename = "advertId")]
    pub advert_id: i64,
    #[serde(default)]
    pub days: Vec<WbAdvertFullStatDay>,
    #[serde(default)]
    pub views: i64,
    #[serde(default)]
    pub clicks: i64,
    #[serde(default)]
    pub ctr: f64,
    #[serde(default)]
    pub cpc: f64,
    #[serde(default)]
    pub atbs: i64,
    #[serde(default)]
    pub orders: i64,
    #[serde(default)]
    pub shks: i64,
    #[serde(default)]
    pub sum: f64,
    #[serde(rename = "sum_price", default)]
    pub sum_price: f64,
    #[serde(default)]
    pub cr: f64,
    #[serde(default)]
    pub canceled: i64,
}

// ============================================================================
// WB Supply (FBS) structs and methods
// ============================================================================

/// РџРѕСЃС‚Р°РІРєР° РёР· /api/v3/supplies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSupplyRow {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(rename = "isB2b", default)]
    pub is_b2b: Option<bool>,
    #[serde(default)]
    pub done: Option<bool>,
    #[serde(rename = "createdAt", default)]
    pub created_at: Option<String>,
    #[serde(rename = "closedAt", default)]
    pub closed_at: Option<String>,
    #[serde(rename = "scanDt", default)]
    pub scan_dt: Option<String>,
    #[serde(rename = "cargoType", default)]
    pub cargo_type: Option<i32>,
    #[serde(rename = "crossBorderType", default)]
    pub cross_border_type: Option<i32>,
    #[serde(rename = "destinationOfficeId", default)]
    pub destination_office_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WbSuppliesResponse {
    pub next: i64,
    #[serde(default)]
    pub supplies: Vec<WbSupplyRow>,
}

/// Р—Р°РєР°Р· РІРЅСѓС‚СЂРё РїРѕСЃС‚Р°РІРєРё РёР· /api/v3/supplies/{id}/orders
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WbSupplyOrderIdsResponse {
    #[serde(rename = "orderIds", default)]
    pub order_ids: Vec<i64>,
}

/// РЎС‚РёРєРµСЂ РёР· /api/v3/orders/stickers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbStickerRow {
    #[serde(rename = "orderId", default)]
    pub order_id: i64,
    /// WB returns partA/partB as either integers or quoted strings — handle both.
    #[serde(rename = "partA", default, deserialize_with = "deser_str_or_i64")]
    pub part_a: Option<i64>,
    #[serde(rename = "partB", default, deserialize_with = "deser_str_or_i64")]
    pub part_b: Option<i64>,
    #[serde(default)]
    pub barcode: Option<String>,
    #[serde(default)]
    pub file: Option<String>,
}

/// Deserializes a field that WB sometimes sends as an integer and sometimes as a quoted string.
fn deser_str_or_i64<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Unexpected, Visitor};
    use std::fmt;

    struct StrOrI64;

    impl<'de> Visitor<'de> for StrOrI64 {
        type Value = Option<i64>;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "an integer or a string containing an integer")
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
            Ok(if v == 0 { None } else { Some(v) })
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
            Ok(if v == 0 { None } else { Some(v as i64) })
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            if v.is_empty() {
                return Ok(None);
            }
            v.parse::<i64>()
                .map(|n| if n == 0 { None } else { Some(n) })
                .map_err(|_| de::Error::invalid_value(Unexpected::Str(v), &self))
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
    }

    deserializer.deserialize_any(StrOrI64)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WbStickersResponse {
    #[serde(default)]
    pub stickers: Vec<WbStickerRow>,
}

fn supply_matches_window(
    supply: &WbSupplyRow,
    range_start: chrono::DateTime<chrono::Utc>,
    range_end: chrono::DateTime<chrono::Utc>,
) -> bool {
    let created_at = supply.created_at.as_deref().and_then(parse_wb_datetime);
    let closed_at = supply.closed_at.as_deref().and_then(parse_wb_datetime);
    let scan_dt = supply.scan_dt.as_deref().and_then(parse_wb_datetime);

    let in_range = |value: Option<chrono::DateTime<chrono::Utc>>| {
        value
            .map(|dt| dt >= range_start && dt <= range_end)
            .unwrap_or(false)
    };

    if in_range(created_at) || in_range(closed_at) || in_range(scan_dt) {
        return true;
    }

    !supply.done.unwrap_or(false) && created_at.map(|dt| dt <= range_end).unwrap_or(true)
}

impl WildberriesApiClient {
    pub async fn fetch_supplies(
        &self,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> anyhow::Result<Vec<WbSupplyRow>> {
        let url = "https://marketplace-api.wildberries.ru/api/v3/supplies";
        let mut all_supplies: Vec<WbSupplyRow> = Vec::new();
        let mut next_cursor: i64 = 0;
        let range_start =
            wb_day_start_utc(date_from).ok_or_else(|| anyhow::anyhow!("Invalid date_from"))?;
        let range_end =
            wb_day_end_utc(date_to).ok_or_else(|| anyhow::anyhow!("Invalid date_to"))?;

        loop {
            let next_str = next_cursor.to_string();
            let response = match self
                .client
                .get(url)
                .header("Authorization", &connection.api_key)
                .query(&[("limit", "1000"), ("next", next_str.as_str())])
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    return Err(anyhow::anyhow!("HTTP error fetching supplies: {}", e));
                }
            };

            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                tracing::warn!("WB supplies API rate limit, sleeping 65s");
                tokio::time::sleep(std::time::Duration::from_secs(65)).await;
                continue;
            }

            if !response.status().is_success() {
                let status = response.status();
                let body = self.read_body_tracked(response).await.unwrap_or_default();
                return Err(anyhow::anyhow!(
                    "WB supplies API error {}: {}",
                    status,
                    body
                ));
            }

            let parsed: WbSuppliesResponse = response
                .json()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to parse WB supplies response: {}", e))?;

            let page_supplies = parsed.supplies;
            let new_next = parsed.next;

            for supply in page_supplies {
                if supply_matches_window(&supply, range_start, range_end) {
                    all_supplies.push(supply);
                }
            }

            if new_next == 0 {
                break;
            }
            next_cursor = new_next;

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        tracing::info!(
            "Fetched {} supplies in date range {}-{}",
            all_supplies.len(),
            date_from,
            date_to
        );
        Ok(all_supplies)
    }

    pub async fn fetch_supply_orders(
        &self,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        supply_id: &str,
    ) -> anyhow::Result<Vec<i64>> {
        let url = format!(
            "https://marketplace-api.wildberries.ru/api/marketplace/v3/supplies/{}/order-ids",
            supply_id
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", &connection.api_key)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch supply order ids: {}", e))?;

        let status = response.status();

        // 404 means WB has no orders for this supply (expected for old/closed supplies)
        if status == reqwest::StatusCode::NOT_FOUND {
            let body = self.read_body_tracked(response).await.unwrap_or_default();
            tracing::info!(
                "WB supply orders 404 for supply {} — body: {}",
                supply_id,
                body
            );
            return Ok(vec![]);
        }

        if !status.is_success() {
            let body = self.read_body_tracked(response).await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "WB supply order ids API error {}: {}",
                status,
                body
            ));
        }

        let body = response
            .text()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read supply order ids response: {}", e))?;
        tracing::info!(
            "WB supply order ids raw response for {}: {}",
            supply_id,
            &body[..body.len().min(500)]
        );

        let parsed: WbSupplyOrderIdsResponse = serde_json::from_str(&body).map_err(|e| {
            anyhow::anyhow!(
                "Failed to parse supply order ids response: {}\nBody: {}",
                e,
                &body[..body.len().min(300)]
            )
        })?;

        Ok(parsed.order_ids)
    }

    pub async fn fetch_supply_order_ids(
        &self,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        supply_id: &str,
    ) -> anyhow::Result<Vec<i64>> {
        let url = format!(
            "https://marketplace-api.wildberries.ru/api/marketplace/v3/supplies/{}/order-ids",
            supply_id
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", &connection.api_key)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch supply order ids: {}", e))?;

        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            let body = self.read_body_tracked(response).await.unwrap_or_default();
            tracing::info!(
                "WB supply order ids 404 for supply {} — body: {}",
                supply_id,
                body
            );
            return Ok(vec![]);
        }

        if !status.is_success() {
            let body = self.read_body_tracked(response).await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "WB supply order ids API error {}: {}",
                status,
                body
            ));
        }

        let body = response
            .text()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read supply order ids response: {}", e))?;
        tracing::info!(
            "WB supply order ids raw response for {}: {}",
            supply_id,
            &body[..body.len().min(500)]
        );

        let parsed: WbSupplyOrderIdsResponse = serde_json::from_str(&body).map_err(|e| {
            anyhow::anyhow!(
                "Failed to parse supply order ids response: {}\nBody: {}",
                e,
                &body[..body.len().min(300)]
            )
        })?;

        Ok(parsed
            .order_ids
            .into_iter()
            .filter(|&order_id| order_id > 0)
            .collect())
    }

    pub async fn fetch_order_stickers(
        &self,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        order_ids: &[i64],
        sticker_type: &str,
        width: i32,
        height: i32,
    ) -> anyhow::Result<Vec<WbStickerRow>> {
        if order_ids.is_empty() {
            return Ok(vec![]);
        }

        // WB API limit: max 100 order IDs per request
        const BATCH_SIZE: usize = 100;
        let url = "https://marketplace-api.wildberries.ru/api/v3/orders/stickers";
        let mut all_stickers: Vec<WbStickerRow> = Vec::new();

        for chunk in order_ids.chunks(BATCH_SIZE) {
            let body = serde_json::json!({ "orders": chunk });
            let request_body = body.to_string();
            let request_body_len = request_body.len() as u64;

            let response = self
                .client
                .post(url)
                .header("Authorization", &connection.api_key)
                .header("Content-Type", "application/json")
                .query(&[
                    ("type", sticker_type),
                    ("width", &width.to_string()),
                    ("height", &height.to_string()),
                ])
                .body(request_body)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to fetch stickers: {}", e))?;

            if !response.status().is_success() {
                let status = response.status();
                let body_text = self
                    .read_body_tracked_with_request_bytes(response, request_body_len)
                    .await
                    .unwrap_or_default();
                return Err(anyhow::anyhow!(
                    "WB stickers API error {}: {}",
                    status,
                    body_text
                ));
            }

            let body_text = self
                .read_body_tracked_with_request_bytes(response, request_body_len)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to read stickers response body: {}", e))?;

            tracing::debug!(
                "WB stickers raw response (batch {} ids): {}",
                chunk.len(),
                &body_text[..body_text.len().min(500)]
            );

            let parsed: WbStickersResponse = serde_json::from_str(&body_text).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to parse stickers JSON: {}\nRaw: {}",
                    e,
                    &body_text[..body_text.len().min(500)]
                )
            })?;

            all_stickers.extend(parsed.stickers);
        }

        Ok(all_stickers)
    }

    /// Fetches brand-new FBS orders from /api/v3/orders/new (no cursor pagination).
    /// These are orders in "waiting" status — just placed, not yet in any supply.
    /// Call this for real-time order visibility without the statistics API delay.
    pub async fn fetch_new_marketplace_orders(
        &self,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
    ) -> anyhow::Result<Vec<WbMarketplaceOrderRow>> {
        let url = "https://marketplace-api.wildberries.ru/api/v3/orders/new";
        self.record_http_request_attempt(0);

        let response = self
            .client
            .get(url)
            .header("Authorization", &connection.api_key)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch new marketplace orders: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let body = self
                .read_body_for_recorded_request(response)
                .await
                .unwrap_or_default();
            return Err(anyhow::anyhow!(
                "WB /api/v3/orders/new error {}: {}",
                status,
                body
            ));
        }

        let body = self
            .read_body_for_recorded_request(response)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read new orders response: {}", e))?;

        // /api/v3/orders/new returns {"orders": [...]} without pagination
        let parsed: WbMarketplaceOrdersResponse = serde_json::from_str(&body).map_err(|e| {
            anyhow::anyhow!(
                "Failed to parse new orders response: {}\nBody: {}",
                e,
                &body[..body.len().min(500)]
            )
        })?;

        tracing::info!(
            "Fetched {} new marketplace orders from /api/v3/orders/new",
            parsed.orders.len()
        );
        Ok(parsed.orders)
    }

    /// Fetches all FBS orders from /api/v3/orders with cursor pagination.
    /// Returns orders with supplyId field — the real-time link between orders and supplies.
    pub async fn fetch_marketplace_orders(
        &self,
        connection: &contracts::domain::a006_connection_mp::aggregate::ConnectionMP,
        date_from: i64,
        date_to: i64,
    ) -> anyhow::Result<Vec<WbMarketplaceOrderRow>> {
        let mut all_orders: Vec<WbMarketplaceOrderRow> = Vec::new();
        let mut next_cursor: i64 = 0;
        let limit = 1000i64;

        loop {
            let url = "https://marketplace-api.wildberries.ru/api/v3/orders";
            self.record_http_request_attempt(0);
            let response = self
                .client
                .get(url)
                .header("Authorization", &connection.api_key)
                .query(&[
                    ("limit", limit.to_string()),
                    ("next", next_cursor.to_string()),
                    ("dateFrom", date_from.to_string()),
                    ("dateTo", date_to.to_string()),
                ])
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to fetch marketplace orders: {}", e))?;

            let status = response.status();
            if !status.is_success() {
                let body = self
                    .read_body_for_recorded_request(response)
                    .await
                    .unwrap_or_default();
                return Err(anyhow::anyhow!(
                    "WB marketplace orders API error {}: {}",
                    status,
                    body
                ));
            }

            let body = self
                .read_body_for_recorded_request(response)
                .await
                .map_err(|e| {
                    anyhow::anyhow!("Failed to read marketplace orders response: {}", e)
                })?;

            let parsed: WbMarketplaceOrdersResponse = serde_json::from_str(&body).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to parse marketplace orders response: {}\nBody: {}",
                    e,
                    &body[..body.len().min(500)]
                )
            })?;

            let page_count = parsed.orders.len();
            tracing::info!(
                "Marketplace orders page: {} records, next={}",
                page_count,
                parsed.next
            );

            all_orders.extend(parsed.orders);

            if parsed.next == 0 || page_count == 0 {
                break;
            }
            next_cursor = parsed.next;
        }

        tracing::info!(
            "Fetched {} marketplace orders total (dateFrom={}, dateTo={})",
            all_orders.len(),
            date_from,
            date_to
        );
        Ok(all_orders)
    }

    /// GET https://returns-api.wildberries.ru/api/v1/claims
    ///
    /// Загружает заявки покупателей на возврат товара.
    /// Requires: WB token with "Buyers Returns" category.
    /// Returns last 14 days only. Fetches both is_archive=false and is_archive=true.
    pub async fn fetch_claims(
        &self,
        connection: &ConnectionMP,
    ) -> Result<Vec<WbClaimRow>> {
        const BASE_URL: &str = "https://returns-api.wildberries.ru/api/v1/claims";
        const PAGE_LIMIT: u32 = 200;

        if connection.api_key.trim().is_empty() {
            anyhow::bail!("API Key is required for WB Buyers Returns API");
        }

        let mut all_claims: Vec<WbClaimRow> = Vec::new();

        for is_archive in [false, true] {
            let archive_label = if is_archive { "archive" } else { "active" };
            let mut offset: u32 = 0;
            let mut page = 0u32;

            loop {
                page += 1;
                self.log_to_file(&format!(
                    "=== WB Claims ({archive_label}) page {page} offset={offset} ==="
                ));
                self.record_http_request_attempt(0);

                let resp = match self
                    .client
                    .get(BASE_URL)
                    .header("Authorization", connection.api_key.trim())
                    .query(&[
                        ("is_archive", is_archive.to_string()),
                        ("limit", PAGE_LIMIT.to_string()),
                        ("offset", offset.to_string()),
                    ])
                    .send()
                    .await
                {
                    Ok(r) => r,
                    Err(e) => {
                        anyhow::bail!("WB Claims API request failed: {}", e);
                    }
                };

                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                self.record_http_response_body(body.len() as u64);

                if status == 429 {
                    tracing::warn!("WB Claims API rate limit hit, sleeping 60s");
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                    continue;
                }

                if status == 404 {
                    // 404 from feedbacks-api auth gateway means the API key
                    // lacks the "Buyers Returns" token category.
                    tracing::warn!(
                        "WB Claims API: 404 Not Found — API key does not have \
                         'Buyers Returns' (Возвраты покупателей) permission. \
                         Skipping claims import. Response: {}",
                        &body[..body.len().min(300)]
                    );
                    return Ok(all_claims);
                }

                if !status.is_success() {
                    anyhow::bail!(
                        "WB Claims API returned status {}: {}",
                        status,
                        &body[..body.len().min(500)]
                    );
                }

                let parsed: WbClaimsResponse = match serde_json::from_str(&body) {
                    Ok(v) => v,
                    Err(e) => {
                        anyhow::bail!("Failed to parse WB Claims response: {}: {}", e, &body[..body.len().min(500)]);
                    }
                };

                let page_len = parsed.claims.len();
                self.log_to_file(&format!(
                    "WB Claims ({archive_label}) page {page}: {page_len} items"
                ));

                all_claims.extend(parsed.claims);

                if page_len < PAGE_LIMIT as usize {
                    break;
                }
                offset += PAGE_LIMIT;
            }
        }

        tracing::info!("WB Claims: fetched {} total", all_claims.len());
        Ok(all_claims)
    }
}

/// Order from /api/v3/orders — marketplace FBS orders with real-time supplyId.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbMarketplaceOrderRow {
    pub id: i64,
    #[serde(rename = "orderUid", default)]
    pub order_uid: Option<String>,
    #[serde(default)]
    pub article: Option<String>,
    #[serde(rename = "nmId", default)]
    pub nm_id: Option<i64>,
    #[serde(rename = "chrtId", default)]
    pub chrt_id: Option<i64>,
    #[serde(default)]
    pub rid: Option<String>,
    #[serde(rename = "createdAt", default)]
    pub created_at: Option<String>,
    #[serde(rename = "warehouseId", default)]
    pub warehouse_id: Option<i64>,
    #[serde(rename = "salePrice", default)]
    pub sale_price: Option<i64>,
    #[serde(rename = "scanPrice", default)]
    pub scan_price: Option<i64>,
    #[serde(default)]
    pub price: Option<i64>,
    #[serde(rename = "finalPrice", default)]
    pub final_price: Option<i64>,
    #[serde(rename = "convertedPrice", default)]
    pub converted_price: Option<i64>,
    #[serde(rename = "convertedFinalPrice", default)]
    pub converted_final_price: Option<i64>,
    #[serde(rename = "currencyCode", default)]
    pub currency_code: Option<i32>,
    #[serde(rename = "convertedCurrencyCode", default)]
    pub converted_currency_code: Option<i32>,
    #[serde(rename = "cargoType", default)]
    pub cargo_type: Option<i32>,
    /// Supply ID in format "WB-GI-XXXXXXXX" — the key for linking orders to supplies.
    #[serde(rename = "supplyId", default)]
    pub supply_id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(rename = "isZeroOrder", default)]
    pub is_zero_order: Option<bool>,
    #[serde(default)]
    pub skus: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WbMarketplaceOrdersResponse {
    #[serde(default)]
    pub next: i64,
    #[serde(default)]
    pub orders: Vec<WbMarketplaceOrderRow>,
}

/// Заявка покупателя на возврат из GET /api/v1/claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbClaimRow {
    pub id: String,
    #[serde(rename = "claim_type", default)]
    pub claim_type: Option<i32>,
    #[serde(default)]
    pub status: Option<i32>,
    #[serde(rename = "status_ex", default)]
    pub status_ex: Option<i32>,
    #[serde(rename = "nm_id", default)]
    pub nm_id: Option<i64>,
    #[serde(rename = "imt_name", default)]
    pub imt_name: Option<String>,
    #[serde(rename = "user_comment", default)]
    pub user_comment: Option<String>,
    #[serde(rename = "wb_comment", default)]
    pub wb_comment: Option<String>,
    #[serde(default)]
    pub dt: Option<String>,
    #[serde(rename = "order_dt", default)]
    pub order_dt: Option<String>,
    #[serde(rename = "dt_update", default)]
    pub dt_update: Option<String>,
    #[serde(rename = "delivery_dt", default)]
    pub delivery_dt: Option<String>,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(rename = "currency_code", default)]
    pub currency_code: Option<String>,
    #[serde(default)]
    pub srid: Option<String>,
    #[serde(rename = "origin_id_info", default)]
    pub origin_id_info: Option<String>,
    #[serde(default)]
    pub actions: Option<Vec<String>>,
    #[serde(rename = "is_archive", default)]
    pub is_archive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WbClaimsResponse {
    #[serde(default)]
    pub claims: Vec<WbClaimRow>,
    #[serde(default)]
    pub total: Option<i64>,
}
