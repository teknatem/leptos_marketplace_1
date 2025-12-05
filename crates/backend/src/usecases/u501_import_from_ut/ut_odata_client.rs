use anyhow::Result;
use contracts::domain::a001_connection_1c::aggregate::Connection1CDatabase;
use std::fs::OpenOptions;
use std::io::Write;

/// HTTP-клиент для работы с OData API УТ 11
pub struct UtODataClient {
    client: reqwest::Client,
}

impl UtODataClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120)) // Увеличен таймаут до 120 секунд для больших запросов
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Записать в лог-файл
    fn log_to_file(&self, message: &str) {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("odata_requests.log")
        {
            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let _ = writeln!(file, "[{}] {}", timestamp, message);
        }
    }

    /// Получить данные из OData коллекции
    pub async fn fetch_collection<T: serde::de::DeserializeOwned>(
        &self,
        connection: &Connection1CDatabase,
        collection_name: &str,
        top: Option<i32>,
        skip: Option<i32>,
    ) -> Result<T> {
        self.fetch_collection_with_filter(connection, collection_name, top, skip, None)
            .await
    }

    /// Получить данные из OData коллекции с фильтром
    pub async fn fetch_collection_with_filter<T: serde::de::DeserializeOwned>(
        &self,
        connection: &Connection1CDatabase,
        collection_name: &str,
        top: Option<i32>,
        skip: Option<i32>,
        filter: Option<&str>,
    ) -> Result<T> {
        self.fetch_collection_with_options(connection, collection_name, top, skip, filter, None, None)
            .await
    }

    /// Получить данные из OData коллекции с фильтром, expand, select и orderby
    pub async fn fetch_collection_with_options<T: serde::de::DeserializeOwned>(
        &self,
        connection: &Connection1CDatabase,
        collection_name: &str,
        top: Option<i32>,
        skip: Option<i32>,
        filter: Option<&str>,
        expand: Option<&str>,
        select: Option<&str>,
    ) -> Result<T> {
        self.fetch_collection_full(connection, collection_name, top, skip, filter, expand, select, None)
            .await
    }

    /// Получить данные из OData коллекции со всеми параметрами включая orderby
    pub async fn fetch_collection_full<T: serde::de::DeserializeOwned>(
        &self,
        connection: &Connection1CDatabase,
        collection_name: &str,
        top: Option<i32>,
        skip: Option<i32>,
        filter: Option<&str>,
        expand: Option<&str>,
        select: Option<&str>,
        orderby: Option<&str>,
    ) -> Result<T> {
        // Формируем полный OData URL: base_url + /odata/standard.odata/ + collection_name
        let base_url = connection.url.trim_end_matches('/');
        let odata_path = if base_url.contains("/odata/") {
            // URL уже содержит /odata/, используем как есть
            base_url.to_string()
        } else {
            // Добавляем /odata/standard.odata/
            format!("{}/odata/standard.odata", base_url)
        };
        let mut url = format!("{}/{}", odata_path, collection_name);

        // Добавить параметры пагинации и фильтрации
        let mut params = Vec::new();
        if let Some(top) = top {
            params.push(format!("$top={}", top));
        }
        if let Some(skip) = skip {
            params.push(format!("$skip={}", skip));
        }
        if let Some(filter) = filter {
            params.push(format!("$filter={}", filter));
        }
        if let Some(expand) = expand {
            params.push(format!("$expand={}", expand));
        }
        if let Some(select) = select {
            params.push(format!("$select={}", select));
        }

        // Если передан orderby - используем его, иначе определяем автоматически
        if let Some(ob) = orderby {
            params.push(format!("$orderby={}", ob));
        } else if skip.is_some() || top.is_some() {
            // ВАЖНО: При использовании пагинации ($skip/$top) ОБЯЗАТЕЛЬНО нужен $orderby для стабильного порядка
            // Определяем поле для сортировки в зависимости от типа коллекции
            let order_by = match collection_name {
                // Справочники и документы имеют Ref_Key
                name if name.starts_with("Catalog_") => Some("Ref_Key"),
                name if name.starts_with("Document_") => Some("Ref_Key"),
                // Регистры сведений (InformationRegister) не имеют Ref_Key
                name if name.starts_with("InformationRegister_") => {
                    if name.contains("Штрихкод") || name.contains("Barcode") {
                        Some("Штрихкод")
                    } else {
                        None
                    }
                },
                _ => None,
            };

            if let Some(field) = order_by {
                params.push(format!("$orderby={} asc", field));
            }
        }

        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        tracing::info!("Fetching OData from: {}", url);
        self.log_to_file(&format!(
            "=== REQUEST ===\nGET {}\nAuthorization: Basic ***",
            url
        ));

        let start_time = std::time::Instant::now();
        
        let response = match self
            .client
            .get(&url)
            .basic_auth(&connection.login, Some(&connection.password))
            .header("Accept", "application/json")
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                let error_msg = if e.is_timeout() {
                    format!("Request timeout after {:?}", e)
                } else if e.is_connect() {
                    format!("Connection error: {:?}", e)
                } else if e.is_request() {
                    format!("Request error: {:?}", e)
                } else {
                    format!("Network error: {:?}", e)
                };
                self.log_to_file(&format!("ERROR: {}", error_msg));
                tracing::error!("{}", error_msg);
                return Err(anyhow::anyhow!("{}", error_msg));
            }
        };

        let elapsed = start_time.elapsed();
        let status = response.status();
        self.log_to_file(&format!("Response status: {} (took {:.2}s)", status, elapsed.as_secs_f64()));
        tracing::info!("Request completed in {:.2}s with status {}", elapsed.as_secs_f64(), status);

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            self.log_to_file(&format!("ERROR Response body:\n{}", body));
            tracing::error!("OData request failed: {}", body);
            anyhow::bail!("OData request failed with status {}: {}", status, body);
        }

        // Получить тело ответа как текст для логирования
        let body = response.text().await?;

        // Логировать ПОЛНЫЙ ответ в файл
        self.log_to_file(&format!("=== RESPONSE BODY ===\n{}\n", body));

        // Логировать первые 500 символов ответа в консоль (UTF-8 безопасно)
        let preview: String = body.chars().take(500).collect::<String>();
        let preview = if preview.len() < body.len() {
            format!("{}...", preview)
        } else {
            preview
        };
        tracing::debug!("OData response preview: {}", preview);

        // Попытаться распарсить JSON
        match serde_json::from_str::<T>(&body) {
            Ok(data) => {
                self.log_to_file("Successfully parsed JSON");
                Ok(data)
            }
            Err(e) => {
                let error_msg = format!("Failed to parse OData JSON: {}", e);
                self.log_to_file(&error_msg);
                tracing::error!("Failed to parse OData response. Error: {}", e);
                tracing::error!("Response body: {}", body);
                anyhow::bail!("Failed to parse OData JSON: {}. Response: {}", e, preview)
            }
        }
    }

    /// Получить количество элементов в коллекции
    pub async fn get_collection_count(
        &self,
        connection: &Connection1CDatabase,
        collection_name: &str,
    ) -> Result<Option<i32>> {
        self.get_collection_count_with_filter(connection, collection_name, None)
            .await
    }

    /// Получить количество элементов в коллекции с фильтром
    pub async fn get_collection_count_with_filter(
        &self,
        connection: &Connection1CDatabase,
        collection_name: &str,
        filter: Option<&str>,
    ) -> Result<Option<i32>> {
        // Формируем полный OData URL
        let base_url = connection.url.trim_end_matches('/');
        let odata_path = if base_url.contains("/odata/") {
            base_url.to_string()
        } else {
            format!("{}/odata/standard.odata", base_url)
        };
        let mut url = format!("{}/{}/$count", odata_path, collection_name);

        // Добавляем фильтр если есть
        if let Some(filter) = filter {
            url.push_str(&format!("?$filter={}", filter));
        }

        tracing::info!("Getting OData count from: {}", url);
        self.log_to_file(&format!("=== COUNT REQUEST ===\nGET {}", url));

        let response = self
            .client
            .get(&url)
            .basic_auth(&connection.login, Some(&connection.password))
            .send()
            .await?;

        if !response.status().is_success() {
            // Некоторые конфигурации УТ не поддерживают $count
            tracing::warn!("Failed to get collection count, continuing without it");
            self.log_to_file("Count request failed (not supported)");
            return Ok(None);
        }

        let count_text = response.text().await?;
        self.log_to_file(&format!("Count response: {}", count_text));
        let count = count_text.trim().parse::<i32>().ok();
        Ok(count)
    }
}

impl Default for UtODataClient {
    fn default() -> Self {
        Self::new()
    }
}
