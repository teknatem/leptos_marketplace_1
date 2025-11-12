use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;

/// HTTP-клиент для работы с Yandex Market API
pub struct YandexApiClient {
    client: reqwest::Client,
}

impl YandexApiClient {
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
            .open("yandex_api_requests.log")
        {
            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let _ = writeln!(file, "[{}] {}", timestamp, message);
        }
    }

    /// Получить список товаров через Yandex Market API
    /// Endpoint: POST /v2/businesses/{businessId}/offer-mappings
    pub async fn fetch_product_list(
        &self,
        connection: &ConnectionMP,
        limit: i32,
        page_token: Option<String>,
    ) -> Result<YandexProductListResponse> {
        // Проверка обязательных полей для Yandex API
        let business_id = connection.business_account_id.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Business ID (БизнесАккаунтID) is required for Yandex Market API")
        })?;

        if connection.api_key.trim().is_empty() {
            anyhow::bail!("Bearer token (API Key) is required for Yandex Market API");
        }

        let url = format!(
            "https://api.partner.market.yandex.ru/v2/businesses/{}/offer-mappings",
            business_id
        );

        // Отправляем пагинацию через query-параметры, так как тело может игнорироваться API для этого эндпоинта
        #[derive(Serialize)]
        struct YandexListQueryParams {
            pub limit: i32,
            #[serde(skip_serializing_if = "Option::is_none", rename = "page_token")]
            pub page_token: Option<String>,
        }

        let request_query = YandexListQueryParams {
            limit,
            page_token: page_token.clone(),
        };

        let token_preview = request_query
            .page_token
            .as_ref()
            .map(|t| &t[..t.len().min(50)])
            .map(|s| s.to_string());

        self.log_to_file(&format!(
            "=== REQUEST ===\nPOST {}\nAuthorization: Bearer ****\nQuery: limit={}, page_token={:?}",
            url, request_query.limit, token_preview
        ));

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", &connection.api_key))
            .header("Content-Type", "application/json")
            .query(&request_query)
            .send()
            .await?;

        let status = response.status();
        self.log_to_file(&format!("Response status: {}", status));

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            self.log_to_file(&format!("ERROR Response body:\n{}", body));
            tracing::error!("Yandex Market API request failed: {}", body);
            anyhow::bail!(
                "Yandex Market API request failed with status {}: {}",
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
        tracing::debug!("Yandex Market API response preview: {}", preview);

        match serde_json::from_str::<YandexProductListResponse>(&body) {
            Ok(data) => {
                self.log_to_file(&format!(
                    "Successfully parsed JSON. Items: {}, nextPageToken: {:?}, total: {:?}",
                    data.result.offer_mapping_entries.len(),
                    data.result.paging.next_page_token,
                    data.result.paging.total
                ));
                tracing::info!(
                    "Yandex API response: {} items, nextPageToken: {:?}, total: {:?}",
                    data.result.offer_mapping_entries.len(),
                    data.result.paging.next_page_token,
                    data.result.paging.total
                );
                Ok(data)
            }
            Err(e) => {
                let error_msg = format!("Failed to parse Yandex Market API JSON: {}", e);
                self.log_to_file(&error_msg);
                tracing::error!("Failed to parse Yandex Market API response. Error: {}", e);
                tracing::error!("Response body: {}", body);
                anyhow::bail!(
                    "Failed to parse Yandex Market API JSON: {}. Response: {}",
                    e,
                    preview
                )
            }
        }
    }

    /// Получить детальную информацию о товарах через Yandex Market API
    /// Endpoint: POST /v2/businesses/{businessId}/offer-cards
    pub async fn fetch_product_info(
        &self,
        connection: &ConnectionMP,
        offer_ids: Vec<String>,
    ) -> Result<YandexProductInfoResponse> {
        let business_id = connection.business_account_id.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Business ID (БизнесАккаунтID) is required for Yandex Market API")
        })?;

        if connection.api_key.trim().is_empty() {
            anyhow::bail!("Bearer token is required for Yandex Market API");
        }

        let url = format!(
            "https://api.partner.market.yandex.ru/v2/businesses/{}/offer-cards",
            business_id
        );

        let request_body = YandexProductInfoRequest { offer_ids };

        let body = serde_json::to_string(&request_body)?;
        self.log_to_file(&format!(
            "=== REQUEST ===\nPOST {}\nAuthorization: Bearer ****\nBody: {}",
            url, body
        ));

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", &connection.api_key))
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;

        let status = response.status();
        self.log_to_file(&format!("Response status: {}", status));

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            self.log_to_file(&format!("ERROR Response body:\n{}", body));
            tracing::error!("Yandex Market API request failed: {}", body);
            anyhow::bail!(
                "Yandex Market API request failed with status {}: {}",
                status,
                body
            );
        }

        let body = response.text().await?;
        self.log_to_file(&format!("=== RESPONSE BODY ===\n{}\n", body));

        match serde_json::from_str::<YandexProductInfoResponse>(&body) {
            Ok(data) => {
                self.log_to_file("Successfully parsed JSON");
                Ok(data)
            }
            Err(e) => {
                let error_msg = format!("Failed to parse Yandex Market API JSON: {}", e);
                self.log_to_file(&error_msg);
                tracing::error!("Failed to parse Yandex Market API response. Error: {}", e);
                anyhow::bail!("Failed to parse Yandex Market API JSON: {}", e)
            }
        }
    }
}

impl Default for YandexApiClient {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Request/Response structures для Yandex Market API
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YandexProductListRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "pageToken")]
    pub page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YandexProductListResponse {
    pub result: YandexProductListResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YandexProductListResult {
    #[serde(rename = "offerMappings")]
    pub offer_mapping_entries: Vec<YandexOfferMappingEntry>,
    pub paging: YandexPaging,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YandexOfferMappingEntry {
    pub offer: YandexOffer,
    #[serde(default)]
    pub mapping: Option<YandexMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YandexOffer {
    #[serde(rename = "offerId")]
    pub offer_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub pictures: Vec<String>,
    #[serde(default)]
    pub barcodes: Vec<String>,
    #[serde(default)]
    pub vendor: Option<String>,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YandexMapping {
    #[serde(rename = "marketSku")]
    pub market_sku: Option<i64>,
    #[serde(rename = "marketSkuName", default)]
    pub market_sku_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YandexPaging {
    #[serde(rename = "nextPageToken", skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
    /// Общее количество элементов (если API возвращает)
    #[serde(default)]
    pub total: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YandexProductInfoRequest {
    #[serde(rename = "offerIds")]
    pub offer_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YandexProductInfoResponse {
    pub result: YandexProductInfoResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YandexProductInfoResult {
    #[serde(rename = "offerMappings")]
    pub offer_mappings: Vec<YandexOfferMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YandexOfferMapping {
    pub offer: YandexOfferCard,
    #[serde(default)]
    pub mapping: Option<YandexOfferCardMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YandexOfferCard {
    #[serde(rename = "offerId")]
    pub offer_id: String,
    #[serde(default, rename = "parameterValues")]
    pub parameter_values: Vec<YandexParameterValue>,
    #[serde(default)]
    pub barcodes: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub pictures: Vec<String>,
    #[serde(default)]
    pub price: Option<YandexPrice>,
    #[serde(default)]
    pub vendor: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>, // Любые дополнительные поля
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YandexOfferCardMapping {
    #[serde(rename = "marketSku")]
    pub market_sku: Option<i64>,
    #[serde(rename = "marketSkuName")]
    pub market_sku_name: Option<String>,
    #[serde(rename = "marketCategoryId")]
    pub market_category_id: Option<i64>,
    #[serde(rename = "marketCategoryName")]
    pub market_category_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YandexParameterValue {
    #[serde(rename = "parameterId")]
    pub parameter_id: i64,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default, rename = "valueId")]
    pub value_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YandexCategory {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YandexPrice {
    pub value: f64,
    pub currency: String,
}

impl YandexApiClient {
    /// Получить список заказов через Yandex Market API с пагинацией
    /// GET /campaigns/{campaignId}/orders
    /// Parameters:
    /// - date_from: начало периода (фильтр по statusUpdateDate)
    /// - date_to: конец периода (фильтр по statusUpdateDate)
    pub async fn fetch_orders(
        &self,
        connection: &ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<Vec<YmOrderItem>> {
        let mut all_orders = Vec::new();
        let mut page = 1;
        let page_size = 50;

        loop {
            let response = self.fetch_orders_page(
                connection,
                date_from,
                date_to,
                page,
                page_size,
            ).await?;

            let orders_count = response.orders.len();
            all_orders.extend(response.orders);

            self.log_to_file(&format!(
                "Fetched page {} with {} orders (total so far: {})",
                page, orders_count, all_orders.len()
            ));

            // Check if there are more pages
            if let Some(pager) = response.pager {
                if let Some(pages_count) = pager.pages_count {
                    if page >= pages_count {
                        break;
                    }
                }
            }

            // Stop if we got less than page_size orders (last page)
            if orders_count < page_size as usize {
                break;
            }

            page += 1;

            // Safety limit to prevent infinite loops
            if page > 100 {
                tracing::warn!("Reached maximum page limit (100), stopping pagination");
                break;
            }
        }

        self.log_to_file(&format!("Total orders fetched: {}", all_orders.len()));
        Ok(all_orders)
    }

    /// Получить одну страницу заказов
    async fn fetch_orders_page(
        &self,
        connection: &ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
        page: i32,
        page_size: i32,
    ) -> Result<YmOrdersResponse> {
        let campaign_id = connection.supplier_id.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Campaign ID (Идентификатор магазина) is required for Yandex Market API")
        })?;

        if connection.api_key.trim().is_empty() {
            anyhow::bail!("Bearer token (API Key) is required for Yandex Market API");
        }

        let url = format!(
            "https://api.partner.market.yandex.ru/campaigns/{}/orders",
            campaign_id
        );

        #[derive(Debug, Serialize)]
        struct QueryParams {
            #[serde(rename = "fromDate")]
            pub from_date: String,
            #[serde(rename = "toDate")]
            pub to_date: String,
            pub page: i32,
            #[serde(rename = "pageSize")]
            pub page_size: i32,
        }

        let query = QueryParams {
            from_date: date_from.format("%d-%m-%Y").to_string(),
            to_date: date_to.format("%d-%m-%Y").to_string(),
            page,
            page_size,
        };

        self.log_to_file(&format!(
            "=== REQUEST PAGE {} ===\nGET {}\nAuthorization: Bearer ****\nQuery: {:?}",
            page, url, query
        ));

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", &connection.api_key))
            .query(&query)
            .send()
            .await?;

        let status = response.status();
        self.log_to_file(&format!("Response status: {}", status));

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            self.log_to_file(&format!("ERROR Response body:\n{}", body));
            tracing::error!("Yandex Market Orders API request failed: {}", body);
            anyhow::bail!("Yandex Market Orders API failed with status {}: {}", status, body);
        }

        let body = response.text().await?;
        self.log_to_file(&format!("=== RESPONSE BODY ===\n{}\n", body));

        match serde_json::from_str::<YmOrdersResponse>(&body) {
            Ok(data) => {
                let orders_count = data.orders.len();
                self.log_to_file(&format!("Successfully parsed {} orders", orders_count));
                Ok(data)
            }
            Err(e) => {
                self.log_to_file(&format!("Failed to parse JSON: {}", e));
                tracing::error!("Failed to parse Yandex Market orders response: {}", e);
                anyhow::bail!("Failed to parse orders response: {}", e)
            }
        }
    }

    /// Получить детали конкретного заказа
    /// GET /campaigns/{campaignId}/orders/{orderId}
    pub async fn fetch_order_details(
        &self,
        connection: &ConnectionMP,
        order_id: i64,
    ) -> Result<YmOrderItem> {
        let campaign_id = connection.supplier_id.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Campaign ID (Идентификатор магазина) is required for Yandex Market API")
        })?;

        if connection.api_key.trim().is_empty() {
            anyhow::bail!("Bearer token (API Key) is required for Yandex Market API");
        }

        let url = format!(
            "https://api.partner.market.yandex.ru/campaigns/{}/orders/{}",
            campaign_id, order_id
        );

        self.log_to_file(&format!(
            "=== REQUEST ===\nGET {}\nAuthorization: Bearer ****",
            url
        ));

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", &connection.api_key))
            .send()
            .await?;

        let status = response.status();
        self.log_to_file(&format!("Response status: {}", status));

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            self.log_to_file(&format!("ERROR Response body:\n{}", body));
            tracing::error!("Yandex Market Order Details API request failed: {}", body);
            anyhow::bail!("Yandex Market Order Details API failed with status {}: {}", status, body);
        }

        let body = response.text().await?;
        self.log_to_file(&format!("=== RESPONSE BODY ===\n{}\n", body));

        match serde_json::from_str::<YmOrderDetailsResponse>(&body) {
            Ok(data) => {
                self.log_to_file("Successfully parsed order details");
                Ok(data.order)
            }
            Err(e) => {
                self.log_to_file(&format!("Failed to parse JSON: {}", e));
                tracing::error!("Failed to parse Yandex Market order details response: {}", e);
                anyhow::bail!("Failed to parse order details response: {}", e)
            }
        }
    }
}

// ============================================================================
// Orders structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmOrdersResponse {
    pub orders: Vec<YmOrderItem>,
    #[serde(default)]
    pub pager: Option<YmOrdersPager>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmOrdersPager {
    pub total: Option<i32>,
    pub from: Option<i32>,
    pub to: Option<i32>,
    #[serde(rename = "currentPage")]
    pub current_page: Option<i32>,
    #[serde(rename = "pagesCount")]
    pub pages_count: Option<i32>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmOrdersPaging {
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmOrderDetailsResponse {
    pub order: YmOrderItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmOrderItem {
    pub id: i64,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(rename = "substatus", default)]
    pub substatus: Option<String>,
    #[serde(rename = "creationDate", default)]
    pub creation_date: Option<String>,
    #[serde(rename = "statusUpdateDate", default)]
    pub status_update_date: Option<String>,
    #[serde(rename = "deliveryDate", default)]
    pub delivery_date: Option<String>,
    #[serde(default)]
    pub items: Vec<YmOrderLineItem>,
    #[serde(default)]
    pub delivery: Option<YmOrderDelivery>,
    #[serde(default)]
    pub total: Option<f64>,
    #[serde(default)]
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmOrderLineItem {
    pub id: i64,
    #[serde(rename = "offerId", default)]
    pub offer_id: Option<String>,
    #[serde(rename = "shopSku", default)]
    pub shop_sku: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub count: i32,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub subsidy: Option<f64>,
    #[serde(default)]
    pub total: Option<f64>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmOrderDelivery {
    #[serde(rename = "type", default)]
    pub delivery_type: Option<String>,
    #[serde(rename = "serviceName", default)]
    pub service_name: Option<String>,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub dates: Option<YmOrderDeliveryDates>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmOrderDeliveryDates {
    #[serde(rename = "realDeliveryDate", default)]
    pub real_delivery_date: Option<String>,
    #[serde(rename = "fromDate", default)]
    pub from_date: Option<String>,
    #[serde(rename = "toDate", default)]
    pub to_date: Option<String>,
}