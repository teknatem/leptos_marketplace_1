use anyhow::Result;
use chrono::Datelike;
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
                anyhow::bail!(
                    "Failed to parse OZON API JSON: {}. Response: {}",
                    e,
                    preview
                )
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

    /// Получить список финансовых транзакций через POST /v3/finance/transaction/list
    pub async fn fetch_finance_transactions(
        &self,
        connection: &ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
        page: i32,
        page_size: i32,
    ) -> Result<OzonFinanceTransactionResponse> {
        let url = "https://api-seller.ozon.ru/v3/finance/transaction/list";

        let client_id = connection
            .application_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Client-Id (ID_Приложения) is required for OZON API"))?;
        if connection.api_key.trim().is_empty() {
            anyhow::bail!("Api-Key is required for OZON API");
        }

        let from_ts = format!("{}T00:00:00Z", date_from.format("%Y-%m-%d"));
        let to_ts = format!("{}T23:59:59Z", date_to.format("%Y-%m-%d"));
        let request_body = OzonFinanceTransactionRequest {
            filter: OzonFinanceTransactionFilter {
                date: OzonFinanceDateFilter {
                    from: from_ts,
                    to: to_ts,
                },
            },
            page,
            page_size,
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

        match serde_json::from_str::<OzonFinanceTransactionResponse>(&body) {
            Ok(data) => Ok(data),
            Err(e) => {
                let preview: String = body.chars().take(500).collect();
                anyhow::bail!(
                    "Failed to parse finance transactions JSON: {}. Body: {}",
                    e,
                    preview
                )
            }
        }
    }

    /// Получить список возвратов через POST /v1/returns/list
    /// Использует курсорную пагинацию через last_id
    pub async fn fetch_returns_list(
        &self,
        connection: &ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
        last_id: i64,
        limit: i32,
    ) -> Result<OzonReturnsListResponse> {
        let url = "https://api-seller.ozon.ru/v1/returns/list";

        let client_id = connection
            .application_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Client-Id (ID_Приложения) is required for OZON API"))?;
        if connection.api_key.trim().is_empty() {
            anyhow::bail!("Api-Key is required for OZON API");
        }

        // Преобразуем даты в ISO datetime формат
        let time_from = format!("{}T00:00:00Z", date_from.format("%Y-%m-%d"));
        let time_to = format!("{}T23:59:59Z", date_to.format("%Y-%m-%d"));

        let request_body = OzonReturnsListRequest {
            filter: OzonReturnsFilter {
                logistic_return_date: Some(OzonReturnsDateFilter { time_from, time_to }),
            },
            limit,
            last_id,
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

        match serde_json::from_str::<OzonReturnsListResponse>(&body) {
            Ok(data) => {
                self.log_to_file("Successfully parsed returns JSON");
                Ok(data)
            }
            Err(e) => {
                let preview: String = body.chars().take(500).collect();
                anyhow::bail!("Failed to parse returns JSON: {}. Body: {}", e, preview)
            }
        }
    }

    /// Получить список FBS отправлений (продаж) через POST /v3/posting/fbs/list
    pub async fn fetch_fbs_postings(
        &self,
        connection: &ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
        limit: i32,
        offset: i32,
    ) -> Result<OzonPostingListResponse> {
        let url = "https://api-seller.ozon.ru/v3/posting/fbs/list";

        let client_id = connection
            .application_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Client-Id required for OZON API"))?;
        if connection.api_key.trim().is_empty() {
            anyhow::bail!("Api-Key required for OZON API");
        }

        let time_from = format!("{}T00:00:00Z", date_from.format("%Y-%m-%d"));
        let time_to = format!("{}T23:59:59Z", date_to.format("%Y-%m-%d"));

        let request_body = OzonPostingListRequest {
            filter: OzonPostingFilter {
                since: Some(time_from),
                to: Some(time_to),
                status: None, // Import all statuses (delivered, cancelled, etc.)
            },
            limit: Some(limit),
            offset: Some(offset),
        };

        let body = serde_json::to_string(&request_body)?;
        self.log_to_file(&format!(
            "=== FBS POSTINGS REQUEST ===\nPOST {}\nBody: {}",
            url, body
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
            anyhow::bail!("OZON FBS postings request failed: {}", status);
        }

        let body = response.text().await?;
        self.log_to_file(&format!("=== FBS POSTINGS RESPONSE ===\n{}\n", body));

        match serde_json::from_str::<OzonPostingListResponse>(&body) {
            Ok(data) => Ok(data),
            Err(e) => anyhow::bail!("Failed to parse FBS postings JSON: {}", e),
        }
    }

    /// Получить список FBO отправлений (продаж) через POST /v2/posting/fbo/list
    pub async fn fetch_fbo_postings(
        &self,
        connection: &ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
        limit: i32,
        offset: i32,
    ) -> Result<OzonPostingListResponse> {
        let url = "https://api-seller.ozon.ru/v2/posting/fbo/list";

        let client_id = connection
            .application_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Client-Id required for OZON API"))?;
        if connection.api_key.trim().is_empty() {
            anyhow::bail!("Api-Key required for OZON API");
        }

        let time_from = format!("{}T00:00:00Z", date_from.format("%Y-%m-%d"));
        let time_to = format!("{}T23:59:59Z", date_to.format("%Y-%m-%d"));

        let request_body = OzonPostingListRequest {
            filter: OzonPostingFilter {
                since: Some(time_from),
                to: Some(time_to),
                status: None, // Import all statuses (delivered, cancelled, etc.)
            },
            limit: Some(limit),
            offset: Some(offset),
        };

        let body = serde_json::to_string(&request_body)?;
        self.log_to_file(&format!(
            "=== FBO POSTINGS REQUEST ===\nPOST {}\nBody: {}",
            url, body
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
            anyhow::bail!("OZON FBO postings request failed: {}", status);
        }

        let body = response.text().await?;
        self.log_to_file(&format!("=== FBO POSTINGS RESPONSE ===\n{}\n", body));

        match serde_json::from_str::<OzonPostingListResponse>(&body) {
            Ok(data) => Ok(data),
            Err(e) => anyhow::bail!("Failed to parse FBO postings JSON: {}", e),
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

// ============================================================================
// Finance transaction list v3 structures (minimal subset)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFinanceTransactionRequest {
    pub filter: OzonFinanceTransactionFilter,
    pub page: i32,
    #[serde(rename = "page_size")]
    pub page_size: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFinanceTransactionFilter {
    pub date: OzonFinanceDateFilter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFinanceDateFilter {
    pub from: String, // YYYY-MM-DD
    pub to: String,   // YYYY-MM-DD
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFinanceTransactionResponse {
    pub result: OzonFinanceTransactionResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFinanceTransactionResult {
    pub operations: Vec<OzonFinanceOperation>,
    #[serde(default)]
    pub page: Option<i32>,
    #[serde(default, rename = "page_size")]
    pub page_size: Option<i32>,
    #[serde(default)]
    pub total: Option<i32>,
    #[serde(default)]
    pub has_next: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFinanceOperation {
    #[serde(default)]
    pub operation_id: serde_json::Value, // can be number or string; we don't use it
    pub operation_type: String,
    pub operation_date: String, // ISO date-time
    pub amount: f64,
    #[serde(default)]
    pub posting: Option<OzonPosting>,
    #[serde(default)]
    pub items: Vec<OzonFinanceItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFinanceItem {
    #[serde(default)]
    pub sku: Option<i64>,
    #[serde(default)]
    pub offer_id: Option<String>,
    #[serde(default)]
    pub quantity: Option<i32>,
}

// ============================================================================
// Returns list v1 structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonReturnsListRequest {
    pub filter: OzonReturnsFilter,
    pub limit: i32,
    pub last_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonReturnsFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logistic_return_date: Option<OzonReturnsDateFilter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonReturnsDateFilter {
    pub time_from: String, // ISO datetime: "2019-08-24T14:15:22Z"
    pub time_to: String,   // ISO datetime: "2019-08-24T14:15:22Z"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonReturnsListResponse {
    pub returns: Vec<OzonReturnItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonReturnItem {
    pub id: i64,
    #[serde(default)]
    pub company_id: Option<i64>,
    #[serde(default)]
    pub return_reason_name: Option<String>,
    #[serde(rename = "type", default)]
    pub return_type: Option<String>,
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub order_id: Option<i64>,
    #[serde(default)]
    pub order_number: Option<String>,
    #[serde(default)]
    pub posting_number: Option<String>,
    #[serde(default)]
    pub product: Option<OzonReturnProduct>, // ВАЖНО: единственное число, объект а не массив!
    #[serde(default)]
    pub logistic: Option<OzonReturnLogistic>,
    #[serde(default)]
    pub clearing_id: Option<i64>,
    #[serde(default)]
    pub return_clearing_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonReturnProduct {
    #[serde(default)]
    pub sku: Option<i64>,
    #[serde(default)]
    pub offer_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub price: Option<OzonReturnPrice>,
    #[serde(default)]
    pub quantity: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonReturnPrice {
    #[serde(default)]
    pub currency_code: Option<String>,
    #[serde(default)]
    pub price: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonReturnLogistic {
    #[serde(default)]
    pub return_date: Option<String>, // ISO datetime
    #[serde(default)]
    pub final_moment: Option<String>,
}

// ============================================================================
// Structures for FBS/FBO Postings (Sales Documents)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonPostingListRequest {
    pub filter: OzonPostingFilter,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonPostingFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<String>, // ISO datetime
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>, // ISO datetime
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonPostingListResponse {
    pub result: OzonPostingListResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonPostingListResult {
    #[serde(default)]
    pub postings: Vec<OzonPosting>,
    #[serde(default)]
    pub has_next: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonPosting {
    pub posting_number: String,
    pub status: String,
    #[serde(default)]
    pub substatus: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub in_process_at: Option<String>,
    #[serde(default)]
    pub delivering_date: Option<String>,
    #[serde(default)]
    pub delivered_at: Option<String>,
    pub products: Vec<OzonPostingProduct>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonPostingProduct {
    #[serde(default)]
    pub product_id: Option<i64>,
    pub offer_id: String,
    pub name: String,
    #[serde(default)]
    pub quantity: i32,
    #[serde(default, deserialize_with = "deserialize_price_option")]
    pub price: Option<f64>,
    #[serde(default)]
    pub currency_code: Option<String>,
}

/// Десериализует цену из строки или числа в Option<f64>
fn deserialize_price_option<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Deserialize};

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrFloat {
        String(String),
        Float(f64),
    }

    match Option::<StringOrFloat>::deserialize(deserializer)? {
        Some(StringOrFloat::String(s)) => {
            s.parse::<f64>()
                .map(Some)
                .map_err(de::Error::custom)
        }
        Some(StringOrFloat::Float(f)) => Ok(Some(f)),
        None => Ok(None),
    }
}

// ========== Finance Realization API ==========

impl OzonApiClient {
    /// Получить финансовые данные реализации через POST /v1/finance/realization/posting
    /// ВАЖНО: API принимает только year и month (не диапазон дат!)
    pub async fn fetch_finance_realization(
        &self,
        connection: &ConnectionMP,
        date_from: chrono::NaiveDate,
        _date_to: chrono::NaiveDate, // Не используется, API работает по месяцам
        limit: i32,
        offset: i32,
    ) -> Result<OzonFinanceRealizationResponse> {
        let url = "https://api-seller.ozon.ru/v1/finance/realization/posting";

        let client_id = connection
            .application_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Client-Id required for OZON API"))?;
        if connection.api_key.trim().is_empty() {
            anyhow::bail!("Api-Key required for OZON API");
        }

        // API OZON Finance Realization работает по месяцам, берем год и месяц из date_from
        let year = date_from.year();
        let month = date_from.month() as i32;

        tracing::info!(
            "Finance Realization Request: date_from={}, year={}, month={}",
            date_from,
            year,
            month
        );

        let request_body = OzonFinanceRealizationRequest {
            year,
            month,
            page: offset / limit + 1,  // Convert offset to page number
            page_size: limit,
        };

        let body = serde_json::to_string(&request_body)?;
        tracing::info!("Finance Realization Request Body: {}", body);
        self.log_to_file(&format!(
            "=== FINANCE REALIZATION REQUEST ===\nPOST {}\nBody: {}",
            url, body
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
            tracing::error!("OZON Finance Realization API request failed: {}", body);
            anyhow::bail!(
                "OZON Finance Realization API request failed with status {}: {}",
                status,
                body
            );
        }

        let body = response.text().await?;
        self.log_to_file(&format!("=== RESPONSE BODY ===\n{}\n", body));

        match serde_json::from_str::<OzonFinanceRealizationResponse>(&body) {
            Ok(data) => {
                self.log_to_file("Successfully parsed finance realization JSON");
                Ok(data)
            }
            Err(e) => {
                let preview: String = body.chars().take(500).collect();
                anyhow::bail!(
                    "Failed to parse finance realization JSON: {}. Body: {}",
                    e,
                    preview
                )
            }
        }
    }
}

/// Запрос на получение финансовых данных реализации
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFinanceRealizationRequest {
    pub year: i32,  // Год (2024)
    pub month: i32, // Месяц (1-12)
    pub page: i32,
    pub page_size: i32,
}

/// Ответ API финансовых данных реализации (БЕЗ обертки result!)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFinanceRealizationResponse {
    pub header: OzonFinanceRealizationHeader,
    pub rows: Vec<OzonFinanceRealizationRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFinanceRealizationHeader {
    pub number: String,           // Номер отчета
    pub doc_date: String,         // Дата документа
    pub start_date: String,       // Начало периода
    pub stop_date: String,        // Конец периода
    pub contract_date: String,    // Дата договора
    pub contract_number: String,  // Номер договора
    pub payer_name: String,       // Плательщик
    pub payer_inn: String,
    pub payer_kpp: String,
    pub receiver_name: String,    // Получатель
    pub receiver_inn: String,
    pub receiver_kpp: String,
    pub doc_amount: f64,          // Сумма документа
    pub vat_amount: f64,          // Сумма НДС
    pub currency_sys_name: String,// Валюта
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFinanceRealizationRow {
    pub row_number: i32,
    pub item: OzonFinanceRealizationItem,
    #[serde(default)]
    pub commission_ratio: Option<f64>,
    #[serde(default)]
    pub seller_price_per_instance: Option<f64>,
    #[serde(default)]
    pub delivery_commission: Option<OzonFinanceCommission>,
    #[serde(default)]
    pub return_commission: Option<OzonFinanceCommission>,
    #[serde(default)]
    pub order: Option<OzonFinanceOrder>,
    #[serde(default)]
    pub legal_entity_document: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFinanceOrder {
    pub posting_number: String,
    #[serde(default)]
    pub created_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFinanceCommission {
    #[serde(default)]
    pub amount: f64,
    #[serde(default)]
    pub bonus: f64,
    #[serde(default)]
    pub commission: f64,
    #[serde(default)]
    pub compensation: f64,
    #[serde(default)]
    pub price_per_instance: f64,
    #[serde(default)]
    pub quantity: f64,
    #[serde(default)]
    pub standard_fee: f64,
    #[serde(default)]
    pub bank_coinvestment: f64,
    #[serde(default)]
    pub stars: f64,
    #[serde(default)]
    pub pick_up_point_coinvestment: f64,
    #[serde(default)]
    pub total: f64,
}

/// Helper для десериализации SKU (может быть строкой или числом)
fn deserialize_sku<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct SkuVisitor;

    impl<'de> Visitor<'de> for SkuVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or integer")
        }

        fn visit_str<E>(self, value: &str) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_i64<E>(self, value: i64) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_u64<E>(self, value: u64) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }
    }

    deserializer.deserialize_any(SkuVisitor)
}

/// Данные товара в строке финансового отчета
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFinanceRealizationItem {
    pub name: String,             // Название товара
    #[serde(deserialize_with = "deserialize_sku")]
    pub sku: String,              // SKU товара (может быть строкой или числом)
    #[serde(default)]
    pub offer_id: String,         // Артикул продавца
    #[serde(default)]
    pub barcode: String,          // Штрихкод
}
