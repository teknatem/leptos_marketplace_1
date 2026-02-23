use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;

/// HTTP-клиент для работы с Wildberries Supplier API
pub struct WildberriesApiClient {
    client: reqwest::Client,
}

impl WildberriesApiClient {
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
                .redirect(reqwest::redirect::Policy::limited(10)) // Следовать редиректам
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Диагностическая функция для тестирования различных вариантов запроса
    pub async fn diagnostic_fetch_all_variations(
        &self,
        connection: &ConnectionMP,
    ) -> Result<Vec<DiagnosticResult>> {
        let mut results = Vec::new();

        // Вариант 1: Текущая реализация (пустой фильтр, limit=100)
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

        // Вариант 2: Увеличенный limit до 1000
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

        // Вариант 3: Без settings вообще (минимальный запрос)
        results.push(
            self.test_minimal_request(connection, "Minimal request (no settings)", 1000)
                .await,
        );

        // Вариант 4: С явным textSearch пустым
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

        // Вариант 5: Альтернативный endpoint - Marketplace API
        results.push(
            self.test_alternative_endpoint(
                connection,
                "Alternative: Marketplace API v3",
                "https://marketplace-api.wildberries.ru",
                "/api/v3/goods/list",
            )
            .await,
        );

        // Вариант 6: Альтернативный endpoint - Supplier API (stocks)
        results.push(
            self.test_stocks_endpoint(connection, "Alternative: Supplier stocks API")
                .await,
        );

        // Вариант 7: КРИТИЧЕСКИЙ ТЕСТ - Попытка получить товары БЕЗ фильтра categories
        // Все предыдущие запросы возвращают только subjectID=7717
        // Попробуем запросить с явным указанием что хотим все категории
        results.push(
            self.test_without_category_filter(
                connection,
                "WITHOUT category filter (attempt to get ALL subjects)",
                1000,
            )
            .await,
        );

        // Вариант 8: АРХИВНЫЕ ТОВАРЫ - /content/v2/get/cards/trash
        // КРИТИЧНО: Возможно большинство товаров в корзине/архиве!
        results.push(
            self.test_trash_endpoint(
                connection,
                "TRASH/Archive endpoint - check deleted/archived products",
                1000,
            )
            .await,
        );

        // Вариант 9: ПОЛУЧИТЬ СПИСОК ВСЕХ КАТЕГОРИЙ ПРОДАВЦА
        // Проверить сколько категорий (subjects) используется
        results.push(
            self.test_get_all_subjects(connection, "Get ALL subjects/categories used by seller")
                .await,
        );

        // Вариант 10: ПРОДОЛЖИТЬ ПАГИНАЦИЮ - получить СЛЕДУЮЩУЮ страницу
        // Возможно API возвращает товары по категориям постранично
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
            let body = response.text().await.unwrap_or_default();
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

        let body = match response.text().await {
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
                    "✓ Success: {} items, cursor.total={}",
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

        // Минимальный запрос - только limit
        let body = format!(r#"{{"limit":{}}}"#, limit);
        self.log_to_file(&format!("Minimal request body: {}", body));

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
            let body = response.text().await.unwrap_or_default();
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

        let body = match response.text().await {
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
                    "✓ Success: {} items, cursor.total={}",
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

        // Пробуем простой GET запрос
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
            let body = response.text().await.unwrap_or_default();
            self.log_to_file(&format!("Error response body: {}", body));

            // 404 или 405 означает что endpoint не существует или метод не поддерживается
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

        let body = match response.text().await {
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

        // Пробуем распарсить как наш стандартный ответ
        match serde_json::from_str::<WildberriesProductListResponse>(&body) {
            Ok(data) => {
                self.log_to_file(&format!(
                    "✓ Success (parseable as standard response): {} items, cursor.total={}",
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
                // Не парсится как стандартный ответ, но запрос успешный
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
            let body = response.text().await.unwrap_or_default();
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

        let body = match response.text().await {
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

        // Stocks API возвращает массив с другой структурой
        // Пробуем распарсить и посчитать количество товаров
        match serde_json::from_str::<serde_json::Value>(&body) {
            Ok(json) => {
                if let Some(stocks) = json.as_array() {
                    let count = stocks.len();
                    self.log_to_file(&format!("✓ Success: Stocks API returned {} items", count));
                    DiagnosticResult {
                        test_name: test_name.to_string(),
                        success: true,
                        error: None,
                        total_returned: count as i32,
                        cursor_total: count as i32, // Stocks API не имеет cursor.total
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
        self.log_to_file("📊 Getting list of ALL subjects/categories from seller account");
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

        // Endpoint для получения списка subjects
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
            let body = response.text().await.unwrap_or_default();
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

        let body = match response.text().await {
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

        // Попробуем распарсить как JSON
        match serde_json::from_str::<serde_json::Value>(&body) {
            Ok(json) => {
                if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                    self.log_to_file(&format!(
                        "✓ Found {} subjects/categories available to this seller!",
                        data.len()
                    ));

                    // Найдем уникальные subjectID
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
                        self.log_to_file("✓ SubjectID 7717 is in the list!");
                    }

                    if data.len() > 1 {
                        self.log_to_file(&format!(
                            "🔥 IMPORTANT: Seller has {} categories, but API returns only from ONE (7717)!",
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
        self.log_to_file("🔄 Testing pagination: Continue from FIRST page cursor");
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

        // Сначала получим первую страницу для извлечения cursor
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

        let body1_text = match response1.text().await {
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

        // Теперь запросим ВТОРУЮ страницу используя cursor из первой
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
            let body = response2.text().await.unwrap_or_default();
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

        let body2_text = match response2.text().await {
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
                    "✓ Second page: {} items, cursor.total={}",
                    second_page.cards.len(),
                    second_page.cursor.total
                ));

                // Проверим subjectID на второй странице
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
                    self.log_to_file("⚠️ Second page is EMPTY! All products were on first page.");
                    self.log_to_file("This means cursor.total matches actual product count.");
                } else if unique_subjects.len() > 1 || !unique_subjects.contains(&7717) {
                    self.log_to_file("🔥 JACKPOT! Second page has DIFFERENT categories!");
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
        self.log_to_file("🗑️ CRITICAL: Checking TRASH/ARCHIVE endpoint");
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
            let body = response.text().await.unwrap_or_default();
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

        let body = match response.text().await {
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
                    "✓ Success: {} items in TRASH, cursor.total={}",
                    data.cards.len(),
                    data.cursor.total
                ));

                if data.cursor.total > 100 {
                    self.log_to_file(&format!(
                        "🔥 JACKPOT! Found {} archived products! This might be the missing products!",
                        data.cursor.total
                    ));
                } else {
                    self.log_to_file("Not many archived products found.");
                }

                // Проверяем уникальные subjectID в архиве
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

        // Попробуем СОВСЕМ минимальный запрос - без cursor вообще
        let body = format!(r#"{{"limit":{}}}"#, limit);
        self.log_to_file(&format!("Minimal request (no cursor at all): {}", body));

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
            let body = response.text().await.unwrap_or_default();
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

        let body = match response.text().await {
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
                // Проверяем уникальные subjectID
                let mut unique_subjects = std::collections::HashSet::new();
                for card in &data.cards {
                    unique_subjects.insert(card.subject_id);
                }

                self.log_to_file(&format!(
                    "✓ Success: {} items, cursor.total={}",
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
                        "⚠️ WARNING: Still only ONE subjectID! API might be filtering by category.",
                    );
                } else {
                    self.log_to_file(&format!(
                        "✓ GOOD: Multiple subjectIDs found! This approach might work."
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
                    anyhow::bail!("Request timeout: API не ответил в течение 60 секунд");
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
                    self.log_to_file("⚠ WARNING: Empty cards array - no more products!");
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

    /// Получить данные по продажам через Statistics API
    /// GET /api/v1/supplier/sales
    /// ВАЖНО: Загружает ВСЕ записи с учетом пагинации API
    pub async fn fetch_sales(
        &self,
        connection: &ConnectionMP,
        date_from: chrono::NaiveDate,
        date_to: chrono::NaiveDate,
    ) -> Result<Vec<WbSaleRow>> {
        let url = "https://statistics-api.wildberries.ru/api/v1/supplier/sales";

        if connection.api_key.trim().is_empty() {
            anyhow::bail!("API Key is required for Wildberries API");
        }

        let date_from_str = date_from.format("%Y-%m-%d").to_string();
        let date_to_str = date_to.format("%Y-%m-%d").to_string();

        // API Wildberries Statistics может возвращать до 100,000 записей за запрос,
        // но рекомендуется делать запросы с флагом page для пагинации
        // Согласно документации: если записей больше, то нужно делать повторные запросы
        // используя параметр flag=1 для получения следующих страниц

        let mut all_sales = Vec::new();
        let mut page_flag = 0; // 0 = первая страница, 1 = следующие страницы

        self.log_to_file(&format!(
            "\n╔════════════════════════════════════════════════════════════════╗"
        ));
        self.log_to_file(&format!("║ WILDBERRIES SALES API - LOADING ALL RECORDS"));
        self.log_to_file(&format!("║ Period: {} to {}", date_from_str, date_to_str));
        self.log_to_file(&format!(
            "╚════════════════════════════════════════════════════════════════╝"
        ));

        loop {
            self.log_to_file(&format!(
                "\n┌────────────────────────────────────────────────────────────┐"
            ));
            self.log_to_file(&format!(
                "│ Request #{} (flag={})",
                (page_flag + 1),
                page_flag
            ));

            self.log_to_file(&format!(
                "=== REQUEST ===\nGET {}?dateFrom={}&dateTo={}&flag={}\nAuthorization: ****",
                url, date_from_str, date_to_str, page_flag
            ));

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

                    // Проверяем конкретные типы ошибок
                    if e.is_timeout() {
                        anyhow::bail!("Request timeout: API не ответил в течение 60 секунд");
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
                tracing::error!("Wildberries Sales API request failed: {}", body);
                anyhow::bail!(
                    "Wildberries Sales API failed with status {}: {}",
                    status,
                    body
                );
            }

            let body = response.text().await?;
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
                    self.log_to_file(&format!("│ Received: {} records", page_count));
                    self.log_to_file(&format!(
                        "│ Total so far: {} records",
                        all_sales.len() + page_count
                    ));

                    if page_data.is_empty() {
                        self.log_to_file(&format!("│ ✓ Empty response - all records loaded"));
                        self.log_to_file(&format!(
                            "└────────────────────────────────────────────────────────────┘"
                        ));
                        break;
                    }

                    // Добавляем полученные данные
                    all_sales.extend(page_data);

                    // API WB Statistics возвращает максимум 100,000 записей за запрос
                    // Если получили меньше, значит это последняя страница
                    if page_count < 100000 {
                        self.log_to_file(&format!(
                            "│ ✓ Received {} records (less than limit) - last page",
                            page_count
                        ));
                        self.log_to_file(&format!(
                            "└────────────────────────────────────────────────────────────┘"
                        ));
                        break;
                    }

                    self.log_to_file(&format!(
                        "│ → More records may be available, requesting next page..."
                    ));
                    self.log_to_file(&format!(
                        "└────────────────────────────────────────────────────────────┘"
                    ));

                    // Переходим к следующей странице
                    page_flag = 1;
                }
                Err(e) => {
                    self.log_to_file(&format!("Failed to parse JSON: {}", e));
                    tracing::error!("Failed to parse Wildberries sales response: {}", e);
                    anyhow::bail!("Failed to parse sales response: {}", e)
                }
            }

            // Небольшая задержка между запросами для снижения нагрузки на API
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        self.log_to_file(&format!(
            "\n╔════════════════════════════════════════════════════════════════╗"
        ));
        self.log_to_file(&format!(
            "║ COMPLETED: Loaded {} total sale records",
            all_sales.len()
        ));
        self.log_to_file(&format!(
            "╚════════════════════════════════════════════════════════════════╝\n"
        ));

        tracing::info!(
            "✓ Wildberries Sales API: Successfully loaded {} total records for period {} to {}",
            all_sales.len(),
            date_from_str,
            date_to_str
        );

        Ok(all_sales)
    }

    /// Загрузить финансовые отчеты из Wildberries по периоду (reportDetailByPeriod)
    /// Возвращает только ЕЖЕДНЕВНЫЕ отчеты (report_type = 1)
    /// 
    /// ВАЖНО: API имеет лимит 1 запрос в минуту!
    /// Используется пагинация через rrdid для загрузки больших объемов данных.
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
            "\n╔════════════════════════════════════════════════════════════════╗"
        ));
        self.log_to_file(&format!(
            "║ WILDBERRIES FINANCE REPORT API - reportDetailByPeriod"
        ));
        self.log_to_file(&format!("║ Period: {} to {}", date_from_str, date_to_str));
        self.log_to_file(&format!("║ Rate limit: 1 request per minute (using pagination)"));
        self.log_to_file(&format!(
            "╚════════════════════════════════════════════════════════════════╝"
        ));

        let mut all_daily_reports: Vec<WbFinanceReportRow> = Vec::new();
        let mut rrdid: i64 = 0;  // Начинаем с 0 для первой страницы
        let limit = 100000;  // Максимальный лимит записей
        let mut page_num = 1;

        loop {
            self.log_to_file(&format!(
                "\n┌────────────────────────────────────────────────────────────┐"
            ));
            self.log_to_file(&format!(
                "│ Page {}: rrdid={}, limit={}",
                page_num, rrdid, limit
            ));
            self.log_to_file(&format!(
                "└────────────────────────────────────────────────────────────┘"
            ));

            self.log_to_file(&format!(
                "=== REQUEST ===\nGET {}?dateFrom={}&dateTo={}&rrdid={}&limit={}\nAuthorization: ****",
                url, date_from_str, date_to_str, rrdid, limit
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
                ])
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    let error_msg = format!("HTTP request failed: {:?}", e);
                    self.log_to_file(&error_msg);
                    tracing::error!("Wildberries Finance Report API connection error: {}", e);

                    // Проверяем конкретные типы ошибок
                    if e.is_timeout() {
                        anyhow::bail!("Request timeout: API не ответил в течение 60 секунд");
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

            // Обработка 429 Too Many Requests - ждем и повторяем
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                self.log_to_file(&format!(
                    "│ ⚠️ Rate limit hit (429). Waiting 65 seconds before retry..."
                ));
                tracing::warn!("WB Finance Report API rate limit hit. Waiting 65 seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(65)).await;
                continue;
            }

            // Обработка 204 No Content - нет данных
            if status == reqwest::StatusCode::NO_CONTENT {
                self.log_to_file(&format!("│ No more data (204 No Content)"));
                break;
            }

            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                self.log_to_file(&format!("ERROR Response body:\n{}", body));
                tracing::error!("Wildberries Finance Report API request failed: {}", body);
                anyhow::bail!(
                    "Wildberries Finance Report API failed with status {}: {}",
                    status,
                    body
                );
            }

            let body = response.text().await?;
            
            // Пустой ответ - конец данных
            if body.trim().is_empty() || body.trim() == "[]" {
                self.log_to_file(&format!("│ Empty response - no more data"));
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

            // Парсим записи
            let page_rows: Vec<WbFinanceReportRow> = match serde_json::from_str(&body) {
                Ok(rows) => rows,
                Err(e) => {
                    self.log_to_file(&format!("Failed to parse JSON: {}", e));
                    tracing::error!("Failed to parse Wildberries finance report response: {}", e);
                    anyhow::bail!("Failed to parse finance report response: {}", e)
                }
            };

            let page_count = page_rows.len();
            self.log_to_file(&format!("│ Received {} records on page {}", page_count, page_num));

            if page_count == 0 {
                self.log_to_file(&format!("│ No records on this page - done"));
                break;
            }

            // Находим максимальный rrd_id для следующей страницы
            let max_rrd_id = page_rows
                .iter()
                .filter_map(|r| r.rrd_id)
                .max()
                .unwrap_or(0);

            // Фильтруем только ЕЖЕДНЕВНЫЕ отчеты (report_type = 1)
            let daily_rows: Vec<WbFinanceReportRow> = page_rows
                .into_iter()
                .filter(|row| row.report_type == Some(1))
                .collect();

            self.log_to_file(&format!(
                "│ Filtered {} daily records (report_type=1)",
                daily_rows.len()
            ));

            all_daily_reports.extend(daily_rows);

            // Если получили меньше записей чем лимит, значит это последняя страница
            if page_count < limit as usize {
                self.log_to_file(&format!(
                    "│ Received {} < {} records - this is the last page",
                    page_count, limit
                ));
                break;
            }

            // Подготовка к следующей странице
            rrdid = max_rrd_id;
            page_num += 1;

            self.log_to_file(&format!(
                "│ → More records may be available. Next rrdid={}",
                rrdid
            ));
            self.log_to_file(&format!(
                "│ ⏳ Waiting 65 seconds before next request (rate limit: 1 req/min)..."
            ));
            
            // ВАЖНО: API имеет лимит 1 запрос в минуту!
            // Ждем 65 секунд для надежности
            tokio::time::sleep(tokio::time::Duration::from_secs(65)).await;
        }

        // Логируем первые 3 записи для проверки загрузки полей
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
            "\n╔════════════════════════════════════════════════════════════════╗"
        ));
        self.log_to_file(&format!(
            "║ COMPLETED: Loaded {} daily finance report records ({} pages)",
            all_daily_reports.len(), page_num
        ));
        self.log_to_file(&format!(
            "╚════════════════════════════════════════════════════════════════╝\n"
        ));

        tracing::info!(
            "✓ Wildberries Finance Report API: Successfully loaded {} daily records for period {} to {}",
            all_daily_reports.len(),
            date_from_str,
            date_to_str
        );

        Ok(all_daily_reports)
    }

    /// Получить данные по заказам через Statistics API (Backfill mode)
    /// GET /api/v1/supplier/orders
    ///
    /// Стратегия:
    /// - flag=0 (инкремент по lastChangeDate)
    /// - dateFrom = курсор lastChangeDate
    /// - для следующей страницы курсор сдвигаем на +1мс от максимального lastChangeDate
    /// - соблюдаем лимит API (1 запрос/мин) и обрабатываем 429
    ///
    /// date_to используется как soft-stop / фильтр.
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
        let soft_stop = date_to
            .and_hms_milli_opt(23, 59, 59, 999)
            .ok_or_else(|| anyhow::anyhow!("Invalid date_to value"))?;

        fn parse_wb_dt(value: &str) -> Option<chrono::NaiveDateTime> {
            chrono::DateTime::parse_from_rfc3339(value)
                .ok()
                .map(|dt| dt.naive_utc())
                .or_else(|| chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f").ok())
                .or_else(|| chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S").ok())
        }

        fn format_cursor(dt: chrono::NaiveDateTime) -> String {
            dt.format("%Y-%m-%dT%H:%M:%S%.3f").to_string()
        }

        self.log_to_file(&format!(
            "\n╔════════════════════════════════════════════════════════════════╗"
        ));
        self.log_to_file(&format!("║ WILDBERRIES ORDERS API - BACKFILL BY CURSOR"));
        self.log_to_file(&format!("║ Period: {} to {}", date_from, date_to));
        self.log_to_file(&format!("║ API URL: {}", url));
        self.log_to_file(&format!(
            "║ Method: flag=0 with lastChangeDate cursor (1 req/min)"
        ));
        self.log_to_file(&format!(
            "╚════════════════════════════════════════════════════════════════╝"
        ));

        loop {
            self.log_to_file(&format!(
                "\n┌────────────────────────────────────────────────────────────┐"
            ));
            self.log_to_file(&format!(
                "│ Page {}: dateFrom={}, flag=0",
                page_num, cursor
            ));

            self.log_to_file(&format!(
                "=== REQUEST ===\nGET {}?dateFrom={}&flag=0\nAuthorization: ****",
                url, cursor
            ));

            let response = match self
                .client
                .get(url)
                .header("Authorization", &connection.api_key)
                .query(&[
                    ("dateFrom", cursor.as_str()),
                    ("flag", "0"),
                ])
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    let error_msg = format!("HTTP request to Orders API failed: {:?}", e);
                    self.log_to_file(&error_msg);
                    tracing::error!("❌ Wildberries Orders API connection error: {}", e);

                    // Проверяем конкретные типы ошибок
                    if e.is_timeout() {
                        anyhow::bail!(
                            "⏱️ Request timeout: Orders API не ответил в течение 60 секунд.\n\n\
                            ⚠️ ВЕРОЯТНАЯ ПРИЧИНА: API endpoint /api/v1/supplier/orders может не существовать в Wildberries API.\n\
                            💡 РЕКОМЕНДАЦИЯ: Попробуйте отключить импорт заказов (a015_wb_orders) и использовать только:\n\
                               - a007_marketplace_product (товары)\n\
                               - a012_wb_sales (продажи)\n\
                               - p903_wb_finance_report (финансы)\n\n\
                            📚 Проверьте актуальную документацию: https://openapi.wb.ru/statistics/api/ru/\n\
                            🔗 URL: {}", 
                            url
                        );
                    } else if e.is_connect() {
                        anyhow::bail!(
                            "🔌 Connection error: не удалось подключиться к WB Orders API.\n\n\
                            ⚠️ ВЕРОЯТНАЯ ПРИЧИНА: API endpoint не существует или был изменён.\n\
                            Возможные решения:\n\
                            1. 📚 Проверьте документацию Wildberries API\n\
                            2. 🌐 Убедитесь в наличии интернет-соединения\n\
                            3. 🔑 Проверьте права API ключа\n\
                            4. ⚙️ Отключите импорт заказов и используйте Sales API (a012)\n\n\
                            🔗 URL: {}\n\
                            Error: {}",
                            url,
                            e
                        );
                    } else if e.is_request() {
                        anyhow::bail!("📤 Request error при загрузке orders: {}", e);
                    } else {
                        anyhow::bail!(
                            "❓ Unknown error при запросе orders: {}.\n\n\
                            ⚠️ ВОЗМОЖНО: API endpoint не существует или не доступен.\n\
                            📝 Проверьте документацию Wildberries API для корректного endpoint заказов.\n\
                            🔗 URL: {}", 
                            e, url
                        );
                    }
                }
            };

            let status = response.status();
            let final_url = response.url().clone();
            self.log_to_file(&format!("Response status: {}", status));
            self.log_to_file(&format!("Final URL: {}", final_url));

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                self.log_to_file(&format!(
                    "│ ⚠️ Rate limit hit (429). Waiting 65 seconds before retry..."
                ));
                tracing::warn!("WB Orders API rate limit hit. Waiting 65 seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(65)).await;
                continue;
            }

            // Логируем заголовки ответа для диагностики
            self.log_to_file(&format!("Response headers:"));
            for (name, value) in response.headers() {
                if let Ok(val_str) = value.to_str() {
                    self.log_to_file(&format!("  {}: {}", name, val_str));
                }
            }

            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                self.log_to_file(&format!("ERROR Response body:\n{}", body));
                tracing::error!(
                    "Wildberries Orders API request failed for cursor {}: {}",
                    cursor,
                    body
                );

                // Специальная обработка для 302 редиректов
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

            // Читаем тело ответа
            let body = match response.text().await {
                Ok(b) => b,
                Err(e) => {
                    self.log_to_file(&format!("│ ⚠️ Failed to read response body: {}", e));
                    tracing::error!(
                        "Failed to read response body for cursor {}: {}",
                        cursor,
                        e
                    );
                    anyhow::bail!("Failed to read response body: {}", e);
                }
            };

            self.log_to_file(&format!("Body length: {} bytes", body.len()));

            // Проверяем, не пустой ли ответ
            let body_trimmed = body.trim();
            if body_trimmed.is_empty() || body_trimmed == "[]" {
                self.log_to_file(&format!("│ Empty response, all records loaded"));
                self.log_to_file(&format!("│ Total so far: {} records", all_orders.len()));
                self.log_to_file(&format!(
                    "└────────────────────────────────────────────────────────────┘"
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
                        "│ Received: {} rows on page {}",
                        page_count, page_num
                    ));
                    self.log_to_file(&format!(
                        "│ Total so far: {} records",
                        all_orders.len() + page_count
                    ));
                    self.log_to_file(&format!(
                        "└────────────────────────────────────────────────────────────┘"
                    ));

                    let mut max_last_change = None::<chrono::NaiveDateTime>;
                    let mut kept_rows = 0usize;
                    for row in page_data {
                        let row_last_change = row
                            .last_change_date
                            .as_deref()
                            .and_then(parse_wb_dt);

                        if let Some(parsed) = row_last_change {
                            if max_last_change.map(|v| parsed > v).unwrap_or(true) {
                                max_last_change = Some(parsed);
                            }
                        }

                        // soft-stop по date_to: строки после date_to не включаем
                        let include_row = row_last_change.map(|dt| dt <= soft_stop).unwrap_or(true);
                        if include_row {
                            all_orders.push(row);
                            kept_rows += 1;
                        }
                    }

                    self.log_to_file(&format!(
                        "│ Kept {} rows after soft-stop filter",
                        kept_rows
                    ));

                    let Some(max_dt) = max_last_change else {
                        self.log_to_file("│ No lastChangeDate found on page; stopping");
                        break;
                    };

                    if max_dt > soft_stop {
                        self.log_to_file(&format!(
                            "│ Soft-stop reached (max lastChangeDate {} > date_to {})",
                            max_dt, soft_stop
                        ));
                        break;
                    }

                    let next_cursor_dt = max_dt + chrono::Duration::milliseconds(1);
                    cursor = format_cursor(next_cursor_dt);
                    page_num += 1;
                }
                Err(e) => {
                    self.log_to_file(&format!("Failed to parse JSON: {}", e));
                    self.log_to_file(&format!("Response body: {}", body_preview));
                    tracing::error!(
                        "Failed to parse Wildberries orders response: {}",
                        e
                    );
                    anyhow::bail!("Failed to parse orders response: {}", e)
                }
            }

            // Лимит WB Statistics: 1 запрос в минуту
            tokio::time::sleep(tokio::time::Duration::from_secs(65)).await;
        }

        self.log_to_file(&format!(
            "\n╔════════════════════════════════════════════════════════════════╗"
        ));
        self.log_to_file(&format!(
            "║ COMPLETED: Loaded {} total order records",
            all_orders.len()
        ));
        self.log_to_file(&format!(
            "╚════════════════════════════════════════════════════════════════╝\n"
        ));

        tracing::info!(
            "✓ Wildberries Orders API: Successfully loaded {} total records for period {} to {}",
            all_orders.len(),
            date_from,
            date_to
        );

        Ok(all_orders)
    }

    /// Получить тарифы комиссий по категориям
    /// GET https://common-api.wildberries.ru/api/v1/tariffs/commission?locale=ru
    ///
    /// Требует авторизацию через API ключ
    pub async fn fetch_commission_tariffs(
        &self,
        connection: &ConnectionMP,
    ) -> Result<Vec<CommissionTariffRow>> {
        let url = "https://common-api.wildberries.ru/api/v1/tariffs/commission?locale=ru";

        if connection.api_key.trim().is_empty() {
            anyhow::bail!("API Key is required for Wildberries Commission Tariffs API");
        }

        self.log_to_file(&format!(
            "\n╔════════════════════════════════════════════════════════════════╗"
        ));
        self.log_to_file(&format!("║ WILDBERRIES COMMISSION TARIFFS API"));
        self.log_to_file(&format!("║ URL: {}", url));
        self.log_to_file(&format!("║ Method: GET (requires Authorization header)"));
        self.log_to_file(&format!(
            "╚════════════════════════════════════════════════════════════════╝"
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

                // Проверяем конкретные типы ошибок
                if e.is_timeout() {
                    anyhow::bail!("Request timeout: API не ответил в течение 60 секунд");
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

        let body = response.text().await?;
        self.log_to_file(&format!("=== RESPONSE BODY ===\n{}\n", body));

        // Parse JSON response
        let parsed: CommissionTariffResponse = serde_json::from_str(&body).map_err(|e| {
            self.log_to_file(&format!("ERROR: Failed to parse JSON: {}", e));
            anyhow::anyhow!("Failed to parse commission tariffs response: {}", e)
        })?;

        self.log_to_file(&format!(
            "✓ Successfully parsed {} commission tariff records",
            parsed.report.len()
        ));

        tracing::info!(
            "✓ Wildberries Commission Tariffs API: Successfully loaded {} tariff records",
            parsed.report.len()
        );

        Ok(parsed.report)
    }

    /// Получить страницу цен товаров из WB Prices API
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

        self.log_to_file(&format!("=== REQUEST ===\nGET {}\nAuthorization: ****", url));

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
                    anyhow::bail!("Request timeout: WB Prices API не ответил в течение 60 секунд");
                } else if e.is_connect() {
                    anyhow::bail!("Connection error: не удалось подключиться к discounts-prices-api.wildberries.ru");
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
            tracing::error!("Wildberries Prices API request failed: {}", body);
            anyhow::bail!(
                "Wildberries Prices API failed with status {}: {}",
                status,
                body
            );
        }

        let body = response.text().await?;
        self.log_to_file(&format!("=== RESPONSE BODY ===\n{}\n", &body[..body.len().min(2000)]));

        let parsed: WbGoodsPriceFilterResponse = serde_json::from_str(&body).map_err(|e| {
            self.log_to_file(&format!("ERROR: Failed to parse JSON: {}", e));
            anyhow::anyhow!("Failed to parse WB Prices response: {}", e)
        })?;

        let rows = parsed.data.map(|d| d.list_goods).unwrap_or_default();
        self.log_to_file(&format!("✓ Parsed {} goods price rows", rows.len()));
        tracing::info!("WB Prices API: loaded {} rows (offset={})", rows.len(), offset);

        Ok(rows)
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

// ============================================================================
// Sales structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSaleRow {
    /// Уникальный идентификатор строки продажи
    #[serde(default)]
    pub srid: Option<String>,
    /// Номенклатурный номер товара
    #[serde(rename = "nmId", default)]
    pub nm_id: Option<i64>,
    /// Артикул продавца
    #[serde(rename = "supplierArticle", default)]
    pub supplier_article: Option<String>,
    /// Штрихкод
    #[serde(default)]
    pub barcode: Option<String>,
    /// Название товара
    #[serde(default)]
    pub brand: Option<String>,
    /// Предмет
    #[serde(default)]
    pub subject: Option<String>,
    /// Категория
    #[serde(default)]
    pub category: Option<String>,
    /// Дата продажи
    #[serde(rename = "date", default)]
    pub sale_dt: Option<String>,
    /// Дата последнего изменения записи
    #[serde(rename = "lastChangeDate", default)]
    pub last_change_date: Option<String>,
    /// Склад
    #[serde(rename = "warehouseName", default)]
    pub warehouse_name: Option<String>,
    /// Страна
    #[serde(rename = "countryName", default)]
    pub country_name: Option<String>,
    /// Регион
    #[serde(rename = "oblastOkrugName", default)]
    pub region_name: Option<String>,
    /// Цена без скидки
    #[serde(rename = "priceWithDisc", default)]
    pub price_with_disc: Option<f64>,
    /// Скидка продавца
    #[serde(rename = "discount", default)]
    pub discount: Option<f64>,
    /// Количество
    #[serde(rename = "quantity", default)]
    pub quantity: Option<i32>,
    /// Тип документа: sale или return
    #[serde(rename = "saleID", default)]
    pub sale_id: Option<String>,
    /// Номер заказа
    #[serde(rename = "odid", default)]
    pub order_id: Option<i64>,
    /// SPP (Согласованная скидка продавца)
    #[serde(rename = "spp", default)]
    pub spp: Option<f64>,
    /// Вознаграждение
    #[serde(rename = "forPay", default)]
    pub for_pay: Option<f64>,
    /// Итоговая стоимость
    #[serde(rename = "finishedPrice", default)]
    pub finished_price: Option<f64>,
    /// Флаг поставки
    #[serde(rename = "isSupply", default)]
    pub is_supply: Option<bool>,
    /// Флаг реализации
    #[serde(rename = "isRealization", default)]
    pub is_realization: Option<bool>,
    /// Полная цена
    #[serde(rename = "totalPrice", default)]
    pub total_price: Option<f64>,
    /// Процент скидки
    #[serde(rename = "discountPercent", default)]
    pub discount_percent: Option<f64>,
    /// Сумма платежа за продажу
    #[serde(rename = "paymentSaleAmount", default)]
    pub payment_sale_amount: Option<f64>,
    /// Тип склада
    #[serde(rename = "warehouseType", default)]
    pub warehouse_type: Option<String>,
}

// ============================================================================
// Finance Report structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbFinanceReportRow {
    /// ID строки отчета
    #[serde(default)]
    pub rrd_id: Option<i64>,
    /// Дата строки финансового отчёта
    #[serde(default)]
    pub rr_dt: Option<String>,
    /// Номенклатурный номер товара
    #[serde(default)]
    pub nm_id: Option<i64>,
    /// Артикул продавца
    #[serde(default)]
    pub sa_name: Option<String>,
    /// Категория товара
    #[serde(default)]
    pub subject_name: Option<String>,
    /// Тип операции по заказу
    #[serde(default)]
    pub supplier_oper_name: Option<String>,
    /// Количество товаров
    #[serde(default)]
    pub quantity: Option<i32>,
    /// Розничная цена за единицу товара
    #[serde(default)]
    pub retail_price: Option<f64>,
    /// Общая сумма продажи
    #[serde(default)]
    pub retail_amount: Option<f64>,
    /// Цена продажи с учетом скидок
    #[serde(default)]
    pub retail_price_withdisc_rub: Option<f64>,
    /// Процент комиссии Wildberries
    #[serde(default)]
    pub commission_percent: Option<f64>,
    /// Комиссия за эквайринг
    #[serde(default)]
    pub acquiring_fee: Option<f64>,
    /// Процент комиссии за эквайринг
    #[serde(default)]
    pub acquiring_percent: Option<f64>,
    /// Сумма, уплаченная покупателем за доставку
    #[serde(default)]
    pub delivery_amount: Option<f64>,
    /// Стоимость доставки на стороне продавца
    #[serde(default)]
    pub delivery_rub: Option<f64>,
    /// Сумма услуги по возврату денежных средств
    #[serde(default)]
    pub ppvz_vw: Option<f64>,
    /// НДС по услуге возврата денежных средств
    #[serde(default)]
    pub ppvz_vw_nds: Option<f64>,
    /// Комиссия WB за продажу
    #[serde(default)]
    pub ppvz_sales_commission: Option<f64>,
    /// Сумма возврата за возвращённые товары
    #[serde(default)]
    pub return_amount: Option<f64>,
    /// Сумма штрафа, удержанного с продавца
    #[serde(default)]
    pub penalty: Option<f64>,
    /// Дополнительные (корректирующие) выплаты продавцу
    #[serde(default)]
    pub additional_payment: Option<f64>,
    /// Плата за хранение товаров на складе
    #[serde(default)]
    pub storage_fee: Option<f64>,
    /// Скорректированные расходы на логистику
    #[serde(default)]
    pub rebill_logistic_cost: Option<f64>,
    /// Тип бонуса или штрафа
    #[serde(default)]
    pub bonus_type_name: Option<String>,
    /// Тип отчета (1 = daily, 2 = weekly)
    #[serde(default)]
    pub report_type: Option<i32>,

    // ============ Дополнительные поля из API (для полного JSON) ============
    /// ID реализационного отчета
    #[serde(default)]
    pub realizationreport_id: Option<i64>,
    /// Дата начала периода отчета
    #[serde(default)]
    pub date_from: Option<String>,
    /// Дата окончания периода отчета
    #[serde(default)]
    pub date_to: Option<String>,
    /// Дата создания отчета
    #[serde(default)]
    pub create_dt: Option<String>,
    /// Валюта
    #[serde(default)]
    pub currency_name: Option<String>,
    /// Код договора поставщика
    #[serde(default)]
    pub suppliercontract_code: Option<String>,
    /// ID сборочного задания
    #[serde(default)]
    pub gi_id: Option<i64>,
    /// Процент доставки
    #[serde(default)]
    pub dlv_prc: Option<f64>,
    /// Дата начала действия фикс. тарифа
    #[serde(default)]
    pub fix_tariff_date_from: Option<String>,
    /// Дата окончания действия фикс. тарифа
    #[serde(default)]
    pub fix_tariff_date_to: Option<String>,
    /// Бренд товара
    #[serde(default)]
    pub brand_name: Option<String>,
    /// Размер товара
    #[serde(default)]
    pub ts_name: Option<String>,
    /// Штрихкод товара
    #[serde(default)]
    pub barcode: Option<String>,
    /// Тип документа
    #[serde(default)]
    pub doc_type_name: Option<String>,
    /// Процент скидки
    #[serde(default)]
    pub sale_percent: Option<f64>,
    /// Название склада
    #[serde(default)]
    pub office_name: Option<String>,
    /// Дата заказа
    #[serde(default)]
    pub order_dt: Option<String>,
    /// Дата продажи
    #[serde(default)]
    pub sale_dt: Option<String>,
    /// ID поставки
    #[serde(default)]
    pub shk_id: Option<i64>,
    /// Тип коробов
    #[serde(default)]
    pub gi_box_type_name: Option<String>,
    /// Скидка на товар для отчета
    #[serde(default)]
    pub product_discount_for_report: Option<f64>,
    /// Промо поставщика
    #[serde(default)]
    pub supplier_promo: Option<f64>,
    /// Согласованная скидка продавца
    #[serde(default)]
    pub ppvz_spp_prc: Option<f64>,
    /// Базовый процент комиссии
    #[serde(default)]
    pub ppvz_kvw_prc_base: Option<f64>,
    /// Процент комиссии
    #[serde(default)]
    pub ppvz_kvw_prc: Option<f64>,
    /// Процент повышения рейтинга поставщика
    #[serde(default)]
    pub sup_rating_prc_up: Option<f64>,
    /// Участие в КГВП v2
    #[serde(default)]
    pub is_kgvp_v2: Option<i32>,
    /// К перечислению за товар
    #[serde(default)]
    pub ppvz_for_pay: Option<f64>,
    /// Вознаграждение
    #[serde(default)]
    pub ppvz_reward: Option<f64>,
    /// Тип процессинга платежа
    #[serde(default)]
    pub payment_processing: Option<String>,
    /// Банк-эквайер
    #[serde(default)]
    pub acquiring_bank: Option<String>,
    /// Название пункта выдачи
    #[serde(default)]
    pub ppvz_office_name: Option<String>,
    /// ID пункта выдачи
    #[serde(default)]
    pub ppvz_office_id: Option<i64>,
    /// ID поставщика
    #[serde(default)]
    pub ppvz_supplier_id: Option<i64>,
    /// Название поставщика
    #[serde(default)]
    pub ppvz_supplier_name: Option<String>,
    /// ИНН поставщика
    #[serde(default)]
    pub ppvz_inn: Option<String>,
    /// Номер декларации
    #[serde(default)]
    pub declaration_number: Option<String>,
    /// ID стикера
    #[serde(default)]
    pub sticker_id: Option<String>,
    /// Страна продажи
    #[serde(default)]
    pub site_country: Option<String>,
    /// Доставка силами продавца
    #[serde(default)]
    pub srv_dbs: Option<bool>,
    /// Организация, предоставившая логистику
    #[serde(default)]
    pub rebill_logistic_org: Option<String>,
    /// Удержания
    #[serde(default)]
    pub deduction: Option<f64>,
    /// Приемка
    #[serde(default)]
    pub acceptance: Option<f64>,
    /// ID сборочного задания
    #[serde(default)]
    pub assembly_id: Option<i64>,
    /// Код маркировки
    #[serde(default)]
    pub kiz: Option<String>,
    /// Уникальный идентификатор строки
    #[serde(default)]
    pub srid: Option<String>,
    /// Юридическое лицо
    #[serde(default)]
    pub is_legal_entity: Option<bool>,
    /// ID возврата
    #[serde(default)]
    pub trbx_id: Option<String>,
    /// Сумма софинансирования рассрочки
    #[serde(default)]
    pub installment_cofinancing_amount: Option<f64>,
    /// Процент скидки WiBES
    #[serde(default)]
    pub wibes_wb_discount_percent: Option<f64>,
    /// Сумма кэшбэка
    #[serde(default)]
    pub cashback_amount: Option<f64>,
    /// Скидка по кэшбэку
    #[serde(default)]
    pub cashback_discount: Option<f64>,
    /// Изменение комиссии по кэшбэку
    #[serde(default)]
    pub cashback_commission_change: Option<f64>,
    /// Уникальный ID заказа
    #[serde(default)]
    pub order_uid: Option<String>,
}

// ============================================================================
// Orders structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbOrderRow {
    /// Дата заказа
    #[serde(default)]
    pub date: Option<String>,
    /// Дата последнего изменения
    #[serde(rename = "lastChangeDate", default)]
    pub last_change_date: Option<String>,
    /// Название склада
    #[serde(rename = "warehouseName", default)]
    pub warehouse_name: Option<String>,
    /// Тип склада
    #[serde(rename = "warehouseType", default)]
    pub warehouse_type: Option<String>,
    /// Название страны
    #[serde(rename = "countryName", default)]
    pub country_name: Option<String>,
    /// Название области/округа
    #[serde(rename = "oblastOkrugName", default)]
    pub oblast_okrug_name: Option<String>,
    /// Название региона
    #[serde(rename = "regionName", default)]
    pub region_name: Option<String>,
    /// Артикул продавца
    #[serde(rename = "supplierArticle", default)]
    pub supplier_article: Option<String>,
    /// nmId (ID номенклатуры WB)
    #[serde(rename = "nmId", default)]
    pub nm_id: Option<i64>,
    /// Баркод
    #[serde(default)]
    pub barcode: Option<String>,
    /// Категория
    #[serde(default)]
    pub category: Option<String>,
    /// Предмет
    #[serde(default)]
    pub subject: Option<String>,
    /// Бренд
    #[serde(default)]
    pub brand: Option<String>,
    /// Размер
    #[serde(rename = "techSize", default)]
    pub tech_size: Option<String>,
    /// Номер поставки
    #[serde(rename = "incomeID", default)]
    pub income_id: Option<i64>,
    /// Флаг поставки
    #[serde(rename = "isSupply", default)]
    pub is_supply: Option<bool>,
    /// Флаг реализации
    #[serde(rename = "isRealization", default)]
    pub is_realization: Option<bool>,
    /// Цена без скидки
    #[serde(rename = "totalPrice", default)]
    pub total_price: Option<f64>,
    /// Процент скидки
    #[serde(rename = "discountPercent", default)]
    pub discount_percent: Option<f64>,
    /// SPP (Согласованная скидка продавца)
    #[serde(default)]
    pub spp: Option<f64>,
    /// Итоговая цена для клиента
    #[serde(rename = "finishedPrice", default)]
    pub finished_price: Option<f64>,
    /// Цена с учетом скидки
    #[serde(rename = "priceWithDisc", default)]
    pub price_with_disc: Option<f64>,
    /// Флаг отмены заказа
    #[serde(rename = "isCancel", default)]
    pub is_cancel: Option<bool>,
    /// Дата отмены
    #[serde(rename = "cancelDate", default)]
    pub cancel_date: Option<String>,
    /// ID стикера
    #[serde(default)]
    pub sticker: Option<String>,
    /// G-номер
    #[serde(rename = "gNumber", default)]
    pub g_number: Option<String>,
    /// SRID - уникальный идентификатор заказа
    #[serde(default)]
    pub srid: Option<String>,
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
