use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;

/// HTTP-клиент для работы с OZON Seller API
pub struct OzonApiClient {
    client: reqwest::Client,
}

impl OzonApiClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Записать в лог-файл
    fn log_to_file(&self, message: &str) {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("ozon_api_requests.log")
        {
            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let _ = writeln!(file, "[{}] {}", timestamp, message);
        }
    }

    /// Получить список товаров через POST /v3/product/list
    pub async fn fetch_product_list(
        &self,
        connection: &ConnectionMP,
        limit: i32,
        last_id: Option<String>,
    ) -> Result<OzonProductListResponse> {
        let url = "https://api-seller.ozon.ru/v3/product/list";

        // Проверка обязательных полей для OZON API
        let client_id = connection
            .application_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Client-Id (ID_Приложения) is required for OZON API"))?;

        if connection.api_key.trim().is_empty() {
            anyhow::bail!("Api-Key is required for OZON API");
        }

        let request_body = OzonProductListRequest {
            filter: Some(OzonProductListFilter {
                visibility: Some("ALL".to_string()),
            }),
            last_id: last_id.unwrap_or_default(),
            limit,
        };

        let body = serde_json::to_string(&request_body)?;
        self.log_to_file(&format!(
            "=== REQUEST ===\nPOST {}\nClient-Id: {}\nApi-Key: ****\nBody: {}",
            url, client_id, body
        ));

        let response = self
            .client
            .post(url)
            .header("Client-Id", client_id)
            .header("Api-Key", &connection.api_key)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;

        let status = response.status();
        self.log_to_file(&format!("Response status: {}", status));

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            self.log_to_file(&format!("ERROR Response body:\n{}", body));
            tracing::error!("OZON API request failed: {}", body);
            anyhow::bail!("OZON API request failed with status {}: {}", status, body);
        }

        let body = response.text().await?;
        self.log_to_file(&format!("=== RESPONSE BODY ===\n{}\n", body));

        let preview: String = body.chars().take(500).collect::<String>();
        let preview = if preview.len() < body.len() {
            format!("{}...", preview)
        } else {
            preview
        };
        tracing::debug!("OZON API response preview: {}", preview);

        match serde_json::from_str::<OzonProductListResponse>(&body) {
            Ok(data) => {
                self.log_to_file("Successfully parsed JSON");
                Ok(data)
            }
            Err(e) => {
                let error_msg = format!("Failed to parse OZON API JSON: {}", e);
                self.log_to_file(&error_msg);
                tracing::error!("Failed to parse OZON API response. Error: {}", e);
                tracing::error!("Response body: {}", body);
                anyhow::bail!("Failed to parse OZON API JSON: {}. Response: {}", e, preview)
            }
        }
    }

    /// Получить детальную информацию о товарах через POST /v3/product/info/list
    pub async fn fetch_product_info(
        &self,
        connection: &ConnectionMP,
        product_ids: Vec<i64>,
    ) -> Result<OzonProductInfoResponse> {
        let url = "https://api-seller.ozon.ru/v3/product/info/list";

        // Проверка обязательных полей для OZON API
        let client_id = connection
            .application_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Client-Id (ID_Приложения) is required for OZON API"))?;

        if connection.api_key.trim().is_empty() {
            anyhow::bail!("Api-Key is required for OZON API");
        }

        let request_body = OzonProductInfoRequest {
            product_id: product_ids,
        };

        let body = serde_json::to_string(&request_body)?;
        self.log_to_file(&format!(
            "=== REQUEST ===\nPOST {}\nClient-Id: {}\nApi-Key: ****\nBody: {}",
            url, client_id, body
        ));

        let response = self
            .client
            .post(url)
            .header("Client-Id", client_id)
            .header("Api-Key", &connection.api_key)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;

        let status = response.status();
        self.log_to_file(&format!("Response status: {}", status));

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            self.log_to_file(&format!("ERROR Response body:\n{}", body));
            tracing::error!("OZON API request failed: {}", body);
            anyhow::bail!("OZON API request failed with status {}: {}", status, body);
        }

        let body = response.text().await?;
        self.log_to_file(&format!("=== RESPONSE BODY ===\n{}\n", body));

        match serde_json::from_str::<OzonProductInfoResponse>(&body) {
            Ok(data) => {
                self.log_to_file("Successfully parsed JSON");
                Ok(data)
            }
            Err(e) => {
                let error_msg = format!("Failed to parse OZON API JSON: {}", e);
                self.log_to_file(&error_msg);
                tracing::error!("Failed to parse OZON API response. Error: {}", e);
                anyhow::bail!("Failed to parse OZON API JSON: {}", e)
            }
        }
    }
}

impl Default for OzonApiClient {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Request/Response structures для OZON API v2
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonProductListRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<OzonProductListFilter>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub last_id: String,
    pub limit: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonProductListFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonProductListResponse {
    pub result: OzonProductListResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonProductListResult {
    pub items: Vec<OzonProductListItem>,
    pub total: i32,
    pub last_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonProductListItem {
    pub product_id: i64,
    pub offer_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonProductInfoRequest {
    pub product_id: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonProductInfoResponse {
    // ВАЖНО: /v3/product/info/list возвращает items напрямую, без обертки result!
    pub items: Vec<OzonProductInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonProductInfo {
    pub id: i64,
    pub name: String,
    pub offer_id: String,
    #[serde(default)]
    pub barcodes: Vec<String>,
    #[serde(default)]
    pub description_category_id: Option<i64>,
    #[serde(default)]
    pub images: Vec<String>,
    pub currency_code: Option<String>,
    pub marketing_price: Option<String>,
    pub old_price: Option<String>,
    pub min_price: Option<String>,
    pub price: String,
    pub premium_price: Option<String>,
    #[serde(default)]
    pub errors: Vec<serde_json::Value>, // Сложные объекты с вложенными структурами
    pub vat: Option<String>,
    #[serde(default)]
    pub stocks: Option<OzonProductStocks>,
    // Дополнительные поля, которые могут присутствовать
    #[serde(default)]
    pub price_indexes: Option<serde_json::Value>, // Игнорируем структуру, парсим как JSON
    #[serde(default)]
    pub visibility_details: Option<serde_json::Value>,
    #[serde(default)]
    pub statuses: Option<serde_json::Value>,
    #[serde(default)]
    pub sources: Vec<serde_json::Value>,
    #[serde(default)]
    pub model_info: Option<serde_json::Value>,
    #[serde(default)]
    pub commissions: Vec<serde_json::Value>,
    #[serde(default)]
    pub promotions: Vec<serde_json::Value>,
    #[serde(default)]
    pub availabilities: Vec<serde_json::Value>,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>, // Любые дополнительные поля
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonProductStocks {
    pub has_stock: Option<bool>,
    #[serde(default)]
    pub stocks: Vec<OzonStockItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonStockItem {
    pub present: i32,
    pub reserved: i32,
    pub sku: i64,
    pub source: String,
}
