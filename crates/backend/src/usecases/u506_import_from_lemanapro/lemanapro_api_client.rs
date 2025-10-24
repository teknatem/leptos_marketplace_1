use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;

/// HTTP-клиент для работы с LemanaPro B2B API
pub struct LemanaProApiClient {
    client: reqwest::Client,
    base_url: String,
}

impl LemanaProApiClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: "https://api.lemanapro.ru".to_string(),
        }
    }

    /// Создать клиент для тестовой среды
    pub fn new_test() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: "https://api-test.lemanapro.ru".to_string(),
        }
    }

    /// Записать в лог-файл
    fn log_to_file(&self, message: &str) {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("lemanapro_api_requests.log")
        {
            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let _ = writeln!(file, "[{}] {}", timestamp, message);
        }
    }

    /// Получить список товаров через GET /b2bintegration-products/v1/products
    pub async fn fetch_products(
        &self,
        connection: &ConnectionMP,
        page: i32,
        per_page: i32,
        region_id: Option<i32>,
    ) -> Result<LemanaProProductsResponse> {
        let url = format!("{}/b2bintegration-products/v1/products", self.base_url);

        // Проверка Bearer токена
        if connection.api_key.trim().is_empty() {
            anyhow::bail!("Bearer token (Api-Key) is required for LemanaPro API");
        }

        // Формируем query параметры
        let mut query_params = vec![
            ("page", page.to_string()),
            ("perPage", per_page.to_string()),
        ];

        if let Some(region) = region_id {
            query_params.push(("regionId", region.to_string()));
        }

        self.log_to_file(&format!(
            "=== REQUEST ===\nGET {}\nAuthorization: Bearer ****\nQuery: {:?}",
            url, query_params
        ));

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", connection.api_key))
            .header("Accept", "application/json")
            .query(&query_params)
            .send()
            .await?;

        let status = response.status();
        self.log_to_file(&format!("Response status: {}", status));

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            self.log_to_file(&format!("ERROR Response body:\n{}", body));
            tracing::error!("LemanaPro API request failed: {}", body);
            anyhow::bail!("LemanaPro API request failed with status {}: {}", status, body);
        }

        let body = response.text().await?;
        self.log_to_file(&format!("=== RESPONSE BODY ===\n{}\n", body));

        let preview: String = body.chars().take(500).collect::<String>();
        let preview = if preview.len() < body.len() {
            format!("{}...", preview)
        } else {
            preview
        };
        tracing::debug!("LemanaPro API response preview: {}", preview);

        match serde_json::from_str::<LemanaProProductsResponse>(&body) {
            Ok(data) => {
                self.log_to_file("Successfully parsed JSON");
                Ok(data)
            }
            Err(e) => {
                let error_msg = format!("Failed to parse LemanaPro API JSON: {}", e);
                self.log_to_file(&error_msg);
                tracing::error!("Failed to parse LemanaPro API response. Error: {}", e);
                tracing::error!("Response body: {}", body);
                anyhow::bail!(
                    "Failed to parse LemanaPro API JSON: {}. Response: {}",
                    e,
                    preview
                )
            }
        }
    }
}

impl Default for LemanaProApiClient {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Request/Response structures для LemanaPro B2B API
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LemanaProProductsResponse {
    pub products: Vec<LemanaProProduct>,
    #[serde(default)]
    pub paging: Option<LemanaProPaging>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LemanaProPaging {
    pub page: i32,
    #[serde(rename = "perPage")]
    pub per_page: i32,
    #[serde(rename = "totalCount")]
    pub total_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LemanaProProduct {
    #[serde(rename = "productItem")]
    pub product_item: i64, // Артикул товара

    #[serde(rename = "productAvailible", default)]
    pub product_available: Option<bool>, // Доступность товара

    #[serde(rename = "productName")]
    pub product_name: String, // Наименование товара

    #[serde(rename = "productVat", default)]
    pub product_vat: Option<i32>, // НДС товара

    #[serde(rename = "productUnitSale", default)]
    pub product_unit_sale: Option<LemanaProUnitSale>, // Единица измерения

    #[serde(rename = "productDescription", default)]
    pub product_description: Option<String>, // Описание товара

    #[serde(default)]
    pub categories: Option<LemanaProCategory>, // Категория товара

    #[serde(rename = "productUrl", default)]
    pub product_url: Option<String>, // Ссылка на товар

    #[serde(rename = "productModel", default)]
    pub product_model: Option<String>, // Модель товара

    #[serde(rename = "productBrand", default)]
    pub product_brand: Option<String>, // Бренд товара

    #[serde(rename = "productWeight", default)]
    pub product_weight: Option<f64>, // Вес товара

    #[serde(rename = "productCountryOfOrigin", default)]
    pub product_country_of_origin: Option<String>, // Страна производства

    #[serde(rename = "productDimensions", default)]
    pub product_dimensions: Option<String>, // Размеры (Глубина/Ширина/Высота)

    #[serde(rename = "productPhoto", default)]
    pub product_photo: Option<LemanaProPhoto>, // Фото товара

    #[serde(rename = "productBarcode", default)]
    pub product_barcode: Option<String>, // Штрих-код товара

    #[serde(rename = "productMarkNeed", default)]
    pub product_mark_need: Option<bool>, // Признак маркировки

    #[serde(rename = "productTraceability", default)]
    pub product_traceability: Option<bool>, // Признак прослеживаемости

    #[serde(rename = "productTnvedCode", default)]
    pub product_tnved_code: Option<String>, // Код ТН ВЭД

    #[serde(rename = "productOkpd2", default)]
    pub product_okpd2: Option<String>, // ОКПД2

    #[serde(rename = "productParams", default)]
    pub product_params: Vec<LemanaProParam>, // Характеристики товара
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LemanaProUnitSale {
    #[serde(rename = "unitOkeiCode", default)]
    pub unit_okei_code: Option<String>, // Код ОКЕИ ЕИ товара

    #[serde(rename = "unitName", default)]
    pub unit_name: Option<String>, // Наименование ЕИ товара
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LemanaProCategory {
    #[serde(rename = "categoryId", default)]
    pub category_id: Option<String>, // Идентификатор категории товара

    #[serde(rename = "categoryName", default)]
    pub category_name: Option<String>, // Наименование категории

    #[serde(rename = "parentCategoryId", default)]
    pub parent_category_id: Option<String>, // Идентификатор родительской категории
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LemanaProPhoto {
    #[serde(default)]
    pub value: Vec<String>, // Массив ссылок на фото товара
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LemanaProParam {
    #[serde(rename = "paramId", default)]
    pub param_id: Option<String>, // Идентификатор характеристики товара

    #[serde(rename = "paramName", default)]
    pub param_name: Option<String>, // Название параметра

    #[serde(rename = "paramValue", default)]
    pub param_value: Option<String>, // Значение параметра
}


