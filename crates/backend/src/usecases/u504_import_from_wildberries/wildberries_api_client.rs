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
        results.push(self.test_minimal_request(connection, "Minimal request (no settings)", 1000).await);

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
            self.test_stocks_endpoint(
                connection,
                "Alternative: Supplier stocks API",
            )
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
            self.test_get_all_subjects(
                connection,
                "Get ALL subjects/categories used by seller",
            )
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
        self.log_to_file(&format!("\n========== DIAGNOSTIC TEST: {} ==========", test_name));

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
        self.log_to_file(&format!("\n========== DIAGNOSTIC TEST: {} ==========", test_name));

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
        self.log_to_file(&format!("\n========== DIAGNOSTIC TEST: {} ==========", test_name));
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

        self.log_to_file(&format!("Response body (first 500 chars): {}", 
            body.chars().take(500).collect::<String>()));

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
                    error: Some("Response has different structure (not standard cards format)".to_string()),
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
        self.log_to_file(&format!("\n========== DIAGNOSTIC TEST: {} ==========", test_name));

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

        self.log_to_file(&format!("Response body (first 500 chars): {}", 
            body.chars().take(500).collect::<String>()));

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
        self.log_to_file(&format!("\n========== DIAGNOSTIC TEST: {} ==========", test_name));
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

        self.log_to_file(&format!("Response body preview: {}", 
            body.chars().take(1000).collect::<String>()));

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
                        self.log_to_file("This confirms: either need to query each category separately,");
                        self.log_to_file("OR continue pagination to get products from other categories.");
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
        self.log_to_file(&format!("\n========== DIAGNOSTIC TEST: {} ==========", test_name));
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
                error: Some(format!("Second request returned status {}: {}", status2, body)),
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
        self.log_to_file(&format!("\n========== DIAGNOSTIC TEST: {} ==========", test_name));
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

        self.log_to_file(&format!("Response body preview: {}", 
            body.chars().take(500).collect::<String>()));

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
        self.log_to_file(&format!("\n========== DIAGNOSTIC TEST: {} ==========", test_name));
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
                    self.log_to_file("⚠️ WARNING: Still only ONE subjectID! API might be filtering by category.");
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
                        first_nm_id,
                        last_nm_id
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
    pub async fn fetch_sales(
        &self,
        connection: &ConnectionMP,
        date_from: chrono::NaiveDate,
    ) -> Result<Vec<WbSaleRow>> {
        let url = "https://statistics-api.wildberries.ru/api/v1/supplier/sales";
        
        if connection.api_key.trim().is_empty() {
            anyhow::bail!("API Key is required for Wildberries API");
        }

        let date_from_str = date_from.format("%Y-%m-%d").to_string();
        
        self.log_to_file(&format!(
            "=== REQUEST ===\nGET {}?dateFrom={}\nAuthorization: ****",
            url, date_from_str
        ));

        let response = self
            .client
            .get(url)
            .header("Authorization", &connection.api_key)
            .query(&[("dateFrom", date_from_str)])
            .send()
            .await?;

        let status = response.status();
        self.log_to_file(&format!("Response status: {}", status));

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            self.log_to_file(&format!("ERROR Response body:\n{}", body));
            tracing::error!("Wildberries Sales API request failed: {}", body);
            anyhow::bail!("Wildberries Sales API failed with status {}: {}", status, body);
        }

        let body = response.text().await?;
        self.log_to_file(&format!("=== RESPONSE BODY ===\n{}\n", body));

        match serde_json::from_str::<Vec<WbSaleRow>>(&body) {
            Ok(data) => {
                self.log_to_file(&format!("Successfully parsed {} sale rows", data.len()));
                Ok(data)
            }
            Err(e) => {
                self.log_to_file(&format!("Failed to parse JSON: {}", e));
                tracing::error!("Failed to parse Wildberries sales response: {}", e);
                anyhow::bail!("Failed to parse sales response: {}", e)
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
