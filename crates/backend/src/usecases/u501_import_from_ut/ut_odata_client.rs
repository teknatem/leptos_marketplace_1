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

        // ВАЖНО: При использовании пагинации ($skip/$top) ОБЯЗАТЕЛЬНО нужен $orderby для стабильного порядка
        if skip.is_some() || top.is_some() {
            // Определяем поле для сортировки в зависимости от коллекции
            let order_by = match collection_name {
                name if name.starts_with("Catalog_") => "Code",
                name if name.starts_with("Document_") => "Number",
                _ => "Ref_Key",
            };
            params.push(format!("$orderby={} asc", order_by));
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

        let response = self
            .client
            .get(&url)
            .basic_auth(&connection.login, Some(&connection.password))
            .header("Accept", "application/json")
            .send()
            .await?;

        let status = response.status();
        self.log_to_file(&format!("Response status: {}", status));

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
