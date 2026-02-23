use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;

/// HTTP-клиент для работы с Yandex Market API
pub struct YandexApiClient {
    client: reqwest::Client,
}

impl YandexApiClient {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60)) // Увеличен таймаут для медленных API
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .default_headers(headers)
                .danger_accept_invalid_certs(true) // Временно для отладки
                .no_proxy()
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

        let response = match self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", &connection.api_key))
            .header("Content-Type", "application/json")
            .query(&request_query)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error_msg = format!("HTTP request failed: {:?}", e);
                self.log_to_file(&error_msg);
                tracing::error!("Yandex Market API connection error: {}", e);

                // Проверяем конкретные типы ошибок
                if e.is_timeout() {
                    anyhow::bail!("Request timeout: API не ответил в течение 60 секунд");
                } else if e.is_connect() {
                    anyhow::bail!("Connection error: не удалось подключиться к серверу Yandex Market. Проверьте интернет-соединение.");
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

        let response = match self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", &connection.api_key))
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error_msg = format!("HTTP request failed: {:?}", e);
                self.log_to_file(&error_msg);
                tracing::error!("Yandex Market API connection error: {}", e);

                // Проверяем конкретные типы ошибок
                if e.is_timeout() {
                    anyhow::bail!("Request timeout: API не ответил в течение 60 секунд");
                } else if e.is_connect() {
                    anyhow::bail!("Connection error: не удалось подключиться к серверу Yandex Market. Проверьте интернет-соединение.");
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
            let response = self
                .fetch_orders_page(connection, date_from, date_to, page, page_size)
                .await?;

            let orders_count = response.orders.len();
            all_orders.extend(response.orders);

            self.log_to_file(&format!(
                "Fetched page {} with {} orders (total so far: {})",
                page,
                orders_count,
                all_orders.len()
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
            anyhow::anyhow!(
                "Campaign ID (Идентификатор магазина) is required for Yandex Market API"
            )
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

        let response = match self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", &connection.api_key))
            .query(&query)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error_msg = format!("HTTP request failed: {:?}", e);
                self.log_to_file(&error_msg);
                tracing::error!("Yandex Market Orders API connection error: {}", e);

                // Проверяем конкретные типы ошибок
                if e.is_timeout() {
                    anyhow::bail!("Request timeout: API не ответил в течение 60 секунд");
                } else if e.is_connect() {
                    anyhow::bail!("Connection error: не удалось подключиться к серверу Yandex Market. Проверьте интернет-соединение.");
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
            tracing::error!("Yandex Market Orders API request failed: {}", body);
            anyhow::bail!(
                "Yandex Market Orders API failed with status {}: {}",
                status,
                body
            );
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
            anyhow::anyhow!(
                "Campaign ID (Идентификатор магазина) is required for Yandex Market API"
            )
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

        let response = match self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", &connection.api_key))
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error_msg = format!("HTTP request failed: {:?}", e);
                self.log_to_file(&error_msg);
                tracing::error!("Yandex Market Order Details API connection error: {}", e);

                // Проверяем конкретные типы ошибок
                if e.is_timeout() {
                    anyhow::bail!("Request timeout: API не ответил в течение 60 секунд");
                } else if e.is_connect() {
                    anyhow::bail!("Connection error: не удалось подключиться к серверу Yandex Market. Проверьте интернет-соединение.");
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
            tracing::error!("Yandex Market Order Details API request failed: {}", body);
            anyhow::bail!(
                "Yandex Market Order Details API failed with status {}: {}",
                status,
                body
            );
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
                tracing::error!(
                    "Failed to parse Yandex Market order details response: {}",
                    e
                );
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
    /// Платеж покупателя (общая стоимость товаров включая НДС, без доставки)
    #[serde(rename = "itemsTotal", default)]
    pub items_total: Option<f64>,
    /// Стоимость доставки
    #[serde(rename = "deliveryTotal", default)]
    pub delivery_total: Option<f64>,
    /// Субсидии от Маркета (вознаграждение продавцу за скидки)
    #[serde(default)]
    pub subsidies: Option<Vec<YmOrderSubsidy>>,
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
    /// Цена товара после всех скидок (buyerPrice)
    #[serde(rename = "buyerPrice", default)]
    pub buyer_price: Option<f64>,
    /// Субсидии на уровне товара
    #[serde(default)]
    pub subsidies: Option<Vec<YmOrderItemSubsidy>>,
}

/// Субсидия на уровне заказа
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmOrderSubsidy {
    /// Сумма субсидии
    #[serde(default)]
    pub amount: Option<f64>,
    /// Тип субсидии: YANDEX_CASHBACK, SUBSIDY, DELIVERY
    #[serde(rename = "type", default)]
    pub subsidy_type: Option<String>,
}

/// Субсидия на уровне товара
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmOrderItemSubsidy {
    /// Сумма субсидии
    #[serde(default)]
    pub amount: Option<f64>,
    /// Тип субсидии
    #[serde(rename = "type", default)]
    pub subsidy_type: Option<String>,
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

// ============================================================================
// Returns structures (GET /v2/campaigns/{campaignId}/returns)
// ============================================================================

/// Wrapper for the API response (top level with status and result)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmReturnsApiResponse {
    pub status: String,
    pub result: YmReturnsResult,
}

/// Inner result structure containing returns and paging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmReturnsResult {
    #[serde(default)]
    pub returns: Vec<YmReturnItem>,
    #[serde(default)]
    pub paging: Option<YmReturnsPaging>,
}

/// Token-based pagination structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmReturnsPaging {
    #[serde(rename = "nextPageToken", default)]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmReturnItem {
    /// ID возврата
    pub id: i64,
    /// ID заказа
    #[serde(rename = "orderId")]
    pub order_id: i64,
    /// Тип возврата: RETURN или UNREDEEMED
    #[serde(rename = "returnType", default)]
    pub return_type: Option<String>,
    /// Статус возврата денег: REFUNDED, REFUND_IN_PROGRESS, NOT_REFUNDED и т.д.
    #[serde(rename = "refundStatus", default)]
    pub refund_status: Option<String>,
    /// Дата создания возврата
    #[serde(rename = "creationDate", default)]
    pub created_at: Option<String>,
    /// Дата обновления возврата
    #[serde(rename = "updateDate", default)]
    pub updated_at: Option<String>,
    /// Общая сумма возврата
    #[serde(default)]
    pub amount: Option<YmReturnAmount>,
    /// Товары в возврате
    #[serde(default)]
    pub items: Vec<YmReturnItemLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmReturnAmount {
    /// Сумма
    #[serde(default)]
    pub value: Option<f64>,
    /// Валюта
    #[serde(rename = "currencyId", default)]
    pub currency_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmReturnItemLine {
    /// Market SKU (идентификатор товара в Маркете)
    #[serde(rename = "marketSku", default)]
    pub market_sku: Option<i64>,
    /// offerId (идентификатор товара продавца)
    #[serde(rename = "offerId", default)]
    pub offer_id: Option<String>,
    /// shopSku (артикул продавца)
    #[serde(rename = "shopSku", default)]
    pub shop_sku: Option<String>,
    /// Название товара
    #[serde(rename = "offerName", default)]
    pub offer_name: Option<String>,
    /// Количество
    #[serde(default)]
    pub count: i32,
    /// Цена товара
    #[serde(default)]
    pub price: Option<f64>,
    /// Причина возврата
    #[serde(rename = "returnReason", default)]
    pub return_reason: Option<String>,
    /// Комментарий покупателя
    #[serde(rename = "returnReasonComment", default)]
    pub return_reason_comment: Option<String>,
    /// Решения по возврату
    #[serde(default)]
    pub decisions: Vec<YmReturnDecision>,
    /// Фотографии дефектов
    #[serde(default)]
    pub photos: Vec<YmReturnPhoto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmReturnDecision {
    /// Тип решения: REFUND_MONEY, DECLINE_REFUND, REFUND_MONEY_INCLUDING_SHIPMENT и т.д.
    #[serde(rename = "decisionType", default)]
    pub decision_type: Option<String>,
    /// Сумма возврата
    #[serde(default)]
    pub amount: Option<YmReturnAmount>,
    /// Компенсация за обратную доставку
    #[serde(rename = "partnerCompensationAmount", default)]
    pub partner_compensation_amount: Option<YmReturnAmount>,
    /// Комментарий к решению
    #[serde(default)]
    pub comment: Option<String>,
    /// Дата решения
    #[serde(rename = "decisionDate", default)]
    pub decision_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmReturnPhoto {
    /// URL фото
    #[serde(default)]
    pub url: Option<String>,
}

impl YandexApiClient {
    /// Получить список возвратов через Yandex Market API с пагинацией (token-based)
    /// GET /v2/campaigns/{campaignId}/returns
    /// Parameters:
    /// - date_from: начало периода (фильтр по дате обновления)
    /// - date_to: конец периода (фильтр по дате обновления)
    pub async fn fetch_returns(
        &self,
        connection: &ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<Vec<YmReturnItem>> {
        let mut all_returns = Vec::new();
        let mut page_token: Option<String> = None;
        let page_size = 50;
        let mut page_count = 0;

        loop {
            page_count += 1;
            let response = self
                .fetch_returns_page(
                    connection,
                    date_from,
                    date_to,
                    page_token.clone(),
                    page_size,
                )
                .await?;

            let returns_count = response.returns.len();
            all_returns.extend(response.returns);

            self.log_to_file(&format!(
                "Fetched page {} with {} returns (total so far: {})",
                page_count,
                returns_count,
                all_returns.len()
            ));

            // Check if there are more pages (token-based pagination)
            let next_token = response.paging.and_then(|p| p.next_page_token);

            if next_token.is_none() {
                // No more pages
                break;
            }

            // Stop if we got no returns (last page)
            if returns_count == 0 {
                break;
            }

            page_token = next_token;

            // Safety limit to prevent infinite loops
            if page_count > 100 {
                tracing::warn!("Reached maximum page limit (100), stopping returns pagination");
                break;
            }
        }

        self.log_to_file(&format!("Total returns fetched: {}", all_returns.len()));
        Ok(all_returns)
    }

    /// Получить одну страницу возвратов (token-based pagination)
    async fn fetch_returns_page(
        &self,
        connection: &ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
        page_token: Option<String>,
        page_size: i32,
    ) -> Result<YmReturnsResult> {
        let campaign_id = connection.supplier_id.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "Campaign ID (Идентификатор магазина) is required for Yandex Market API"
            )
        })?;

        if connection.api_key.trim().is_empty() {
            anyhow::bail!("Bearer token (API Key) is required for Yandex Market API");
        }

        let url = format!(
            "https://api.partner.market.yandex.ru/v2/campaigns/{}/returns",
            campaign_id
        );

        // Build query parameters
        let mut query_params: Vec<(&str, String)> = vec![
            ("fromDate", date_from.format("%d-%m-%Y").to_string()),
            ("toDate", date_to.format("%d-%m-%Y").to_string()),
            ("pageSize", page_size.to_string()),
        ];

        if let Some(ref token) = page_token {
            query_params.push(("page_token", token.clone()));
        }

        self.log_to_file(&format!(
            "=== REQUEST RETURNS PAGE ===\nGET {}\nAuthorization: Bearer ****\nQuery: {:?}",
            url, query_params
        ));

        let response = match self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", &connection.api_key))
            .query(&query_params)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error_msg = format!("HTTP request failed: {:?}", e);
                self.log_to_file(&error_msg);
                tracing::error!("Yandex Market Returns API connection error: {}", e);

                // Проверяем конкретные типы ошибок
                if e.is_timeout() {
                    anyhow::bail!("Request timeout: API не ответил в течение 60 секунд");
                } else if e.is_connect() {
                    anyhow::bail!("Connection error: не удалось подключиться к серверу Yandex Market. Проверьте интернет-соединение.");
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
            tracing::error!("Yandex Market Returns API request failed: {}", body);
            anyhow::bail!(
                "Yandex Market Returns API failed with status {}: {}",
                status,
                body
            );
        }

        let body = response.text().await?;
        self.log_to_file(&format!("=== RETURNS RESPONSE BODY ===\n{}\n", body));

        match serde_json::from_str::<YmReturnsApiResponse>(&body) {
            Ok(api_response) => {
                let returns_count = api_response.result.returns.len();
                self.log_to_file(&format!("Successfully parsed {} returns", returns_count));
                Ok(api_response.result)
            }
            Err(e) => {
                self.log_to_file(&format!("Failed to parse returns JSON: {}", e));
                tracing::error!("Failed to parse Yandex Market returns response: {}", e);
                anyhow::bail!("Failed to parse returns response: {}", e)
            }
        }
    }

    // =========================================================================
    // Payment Report (united-netting) — two-phase async report generation
    // =========================================================================

    /// Phase 1: Request payment report generation.
    /// Endpoint: POST https://api.partner.market.yandex.ru/v2/reports/united-netting/generate
    /// Returns reportId that can be polled with `poll_report_status`.
    pub async fn generate_payment_report(
        &self,
        connection: &ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<String> {
        // businessId goes in the JSON body; format is a query parameter
        let business_id: i64 = connection
            .business_account_id
            .as_ref()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "businessId (БизнесАккаунтID) is required for YM payment report generation"
                )
            })?
            .parse()
            .map_err(|e| anyhow::anyhow!("businessId must be an integer: {}", e))?;

        let url =
            "https://api.partner.market.yandex.ru/v2/reports/united-netting/generate?format=CSV";

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct GeneratePaymentReportRequest {
            business_id: i64,
            date_from: String,
            date_to: String,
        }

        let body = GeneratePaymentReportRequest {
            business_id,
            date_from: date_from.format("%Y-%m-%d").to_string(),
            date_to: date_to.format("%Y-%m-%d").to_string(),
        };

        self.log_to_file(&format!(
            "=== GENERATE PAYMENT REPORT ===\nPOST {}\nbusinessId={}, date_from={}, date_to={}",
            url,
            business_id,
            date_from,
            date_to
        ));

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", &connection.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to request payment report generation: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let err_body = response.text().await.unwrap_or_default();
            self.log_to_file(&format!(
                "Generate payment report failed ({}): {}",
                status, err_body
            ));
            anyhow::bail!(
                "generate_payment_report failed with status {}: {}",
                status,
                err_body
            );
        }

        let resp_body = response.text().await?;
        self.log_to_file(&format!(
            "Generate payment report response: {}",
            resp_body
        ));

        let parsed: serde_json::Value = serde_json::from_str(&resp_body)
            .map_err(|e| anyhow::anyhow!("Failed to parse generate response: {}", e))?;

        let report_id = parsed
            .pointer("/result/reportId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("reportId not found in response: {}", resp_body))?
            .to_string();

        self.log_to_file(&format!("Payment report generated, reportId={}", report_id));
        Ok(report_id)
    }

    /// Phase 2: Poll report generation status.
    /// Endpoint: GET https://api.partner.market.yandex.ru/v2/reports/info/{reportId}
    /// Returns (status_str, Option<download_url>).
    /// status_str values: "PENDING" | "PROCESSING" | "DONE" | "FAILED"
    pub async fn poll_report_status(
        &self,
        connection: &ConnectionMP,
        report_id: &str,
    ) -> Result<(String, Option<String>)> {
        let url = format!(
            "https://api.partner.market.yandex.ru/v2/reports/info/{}",
            report_id
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", &connection.api_key))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to poll report status: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let err_body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "poll_report_status failed with status {}: {}",
                status,
                err_body
            );
        }

        let resp_body = response.text().await?;
        self.log_to_file(&format!(
            "Poll report status ({}): {}",
            report_id, resp_body
        ));

        let parsed: serde_json::Value = serde_json::from_str(&resp_body)
            .map_err(|e| anyhow::anyhow!("Failed to parse status response: {}", e))?;

        let report_status = parsed
            .pointer("/result/status")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN")
            .to_string();

        let file_url = parsed
            .pointer("/result/file")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok((report_status, file_url))
    }

    /// Phase 3: Download the generated CSV report from the signed URL.
    /// Returns the raw CSV text content.
    /// Downloads a ZIP archive from the given URL, saves it to `downloads/p907_ym_payment_report/`,
    /// extracts the first CSV file found inside, saves that too, and returns (csv_text, zip_path, csv_path).
    pub async fn download_report_zip(&self, url: &str) -> Result<(String, String, String)> {
        self.log_to_file(&format!("Downloading payment report ZIP from: {}", url));

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to download report file: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let err_body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "download_report_zip failed with status {}: {}",
                status,
                err_body
            );
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read report file body: {}", e))?;

        self.log_to_file(&format!(
            "Downloaded payment report ZIP, size={} bytes",
            bytes.len()
        ));

        // Save ZIP and extract CSV in a blocking thread (zip operations are synchronous)
        let log_closure = {
            let log_path = "yandex_api_requests.log".to_string();
            move |msg: String| {
                use std::io::Write;
                if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&log_path) {
                    let ts = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                    let _ = writeln!(f, "[{}] {}", ts, msg);
                }
            }
        };

        let bytes_vec = bytes.to_vec();
        let (csv_text, zip_path, csv_path) =
            tokio::task::spawn_blocking(move || -> Result<(String, String, String)> {
                // Ensure downloads directory exists
                let dir = std::path::Path::new("downloads/p907_ym_payment_report");
                std::fs::create_dir_all(dir)
                    .map_err(|e| anyhow::anyhow!("Failed to create downloads dir: {}", e))?;

                // Save ZIP with timestamp
                let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                let zip_path = dir.join(format!("report_{}.zip", ts));
                std::fs::write(&zip_path, &bytes_vec)
                    .map_err(|e| anyhow::anyhow!("Failed to save ZIP to disk: {}", e))?;
                let zip_path_str = zip_path.to_string_lossy().to_string();
                log_closure(format!("Saved ZIP to: {}", zip_path_str));

                // Extract CSV from ZIP
                let cursor = std::io::Cursor::new(&bytes_vec[..]);
                let mut archive = zip::ZipArchive::new(cursor)
                    .map_err(|e| anyhow::anyhow!("Failed to open ZIP archive: {}", e))?;

                log_closure(format!("ZIP contains {} file(s):", archive.len()));
                for i in 0..archive.len() {
                    if let Ok(f) = archive.by_index(i) {
                        log_closure(format!(
                            "  ZIP entry[{}]: {} ({} bytes uncompressed)",
                            i,
                            f.name(),
                            f.size()
                        ));
                    }
                }

                for i in 0..archive.len() {
                    let mut file = archive
                        .by_index(i)
                        .map_err(|e| anyhow::anyhow!("Failed to read ZIP entry {}: {}", i, e))?;

                    let name = file.name().to_string();
                    let lower = name.to_lowercase();

                    if lower.ends_with(".csv") {
                        use std::io::Read;
                        let mut raw_bytes: Vec<u8> = Vec::new();
                        file.read_to_end(&mut raw_bytes)
                            .map_err(|e| anyhow::anyhow!("Failed to read {} from ZIP: {}", name, e))?;

                        // Save raw CSV bytes to disk before decoding
                        let csv_path = dir.join(format!("report_{}.csv", ts));
                        std::fs::write(&csv_path, &raw_bytes)
                            .map_err(|e| anyhow::anyhow!("Failed to save CSV to disk: {}", e))?;
                        let csv_path_str = csv_path.to_string_lossy().to_string();
                        log_closure(format!("Saved CSV to: {}", csv_path_str));

                        // Decode: strip UTF-8 BOM if present
                        let content = if raw_bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
                            String::from_utf8_lossy(&raw_bytes[3..]).into_owned()
                        } else {
                            String::from_utf8_lossy(&raw_bytes).into_owned()
                        };

                        let preview: String = content.chars().take(500).collect();
                        log_closure(format!(
                            "CSV '{}': {} chars, first 500 chars: {:?}",
                            name,
                            content.len(),
                            preview
                        ));

                        return Ok((content, zip_path_str, csv_path_str));
                    }
                }

                anyhow::bail!("No .csv file found inside the payment report ZIP archive")
            })
            .await
            .map_err(|e| anyhow::anyhow!("spawn_blocking panicked: {}", e))??;

        Ok((csv_text, zip_path, csv_path))
    }
}
