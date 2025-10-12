use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;

/// HTTP-клиент для работы с Wildberries Supplier API
pub struct WildberriesApiClient {
    client: reqwest::Client,
}

impl WildberriesApiClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .danger_accept_invalid_certs(true) // Временно для отладки
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Записать в лог-файл
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

    /// Получить список товаров через POST /content/v2/get/cards/list
    pub async fn fetch_product_list(
        &self,
        connection: &ConnectionMP,
        limit: i32,
        cursor: Option<WildberriesCursor>,
    ) -> Result<WildberriesProductListResponse> {
        // Используем URL из настроек подключения, если задан, иначе default
        let base_url = if let Some(ref supplier_id) = connection.supplier_id {
            if supplier_id.starts_with("http") {
                // Если supplier_id содержит полный URL, используем его как base URL
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

        // Wildberries API использует курсорную пагинацию
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

                // Проверяем конкретные типы ошибок
                if e.is_timeout() {
                    anyhow::bail!("Request timeout: API не ответил в течение 30 секунд");
                } else if e.is_connect() {
                    anyhow::bail!("Connection error: не удалось подключиться к серверу WB. Проверьте интернет-соединение.");
                } else if e.is_request() {
                    anyhow::bail!("Request error: проблема при отправке запроса - {}", e);
                } else {
                    anyhow::bail!("Unknown error: {}", e);
                }
            }
        };

        let status = response.status();
        self.log_to_file(&format!("Response status: {}", status));

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            self.log_to_file(&format!("ERROR Response body:\n{}", body));
            tracing::error!("Wildberries API request failed: {}", body);
            anyhow::bail!(
                "Wildberries API request failed with status {}: {}",
                status,
                body
            );
        }

        let body = response.text().await?;
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
                let cursor_str = data.cursor.updated_at.as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("none");

                self.log_to_file(&format!(
                    "Successfully parsed JSON. Items: {}, cursor: updatedAt={}, nmID={:?}, total={}",
                    data.cards.len(),
                    cursor_str,
                    data.cursor.nm_id,
                    data.cursor.total
                ));
                tracing::info!(
                    "Wildberries API response: {} items, total: {}, cursor: updatedAt={}",
                    data.cards.len(),
                    data.cursor.total,
                    cursor_str
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
}

impl Default for WildberriesApiClient {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Request/Response structures для Wildberries API
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
    #[serde(rename = "Наименование характеристики", default)]
    pub name: Option<String>,
    #[serde(rename = "Значение характеристики", default)]
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
