use anyhow::Result;
use contracts::domain::a001_connection_1c::aggregate::Connection1CDatabase;

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

    /// Получить данные из OData коллекции
    pub async fn fetch_collection<T: serde::de::DeserializeOwned>(
        &self,
        connection: &Connection1CDatabase,
        collection_name: &str,
        top: Option<i32>,
        skip: Option<i32>,
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

        // Добавить параметры пагинации
        let mut params = Vec::new();
        if let Some(top) = top {
            params.push(format!("$top={}", top));
        }
        if let Some(skip) = skip {
            params.push(format!("$skip={}", skip));
        }

        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        tracing::info!("Fetching OData from: {}", url);

        let response = self
            .client
            .get(&url)
            .basic_auth(&connection.login, Some(&connection.password))
            .header("Accept", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("OData request failed with status {}: {}", status, body);
        }

        // Получить тело ответа как текст для логирования
        let body = response.text().await?;

        // Логировать первые 500 символов ответа
        let preview = if body.len() > 500 {
            format!("{}...", &body[..500])
        } else {
            body.clone()
        };
        tracing::debug!("OData response preview: {}", preview);

        // Попытаться распарсить JSON
        match serde_json::from_str::<T>(&body) {
            Ok(data) => Ok(data),
            Err(e) => {
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
        // Формируем полный OData URL
        let base_url = connection.url.trim_end_matches('/');
        let odata_path = if base_url.contains("/odata/") {
            base_url.to_string()
        } else {
            format!("{}/odata/standard.odata", base_url)
        };
        let url = format!("{}/{}/$count", odata_path, collection_name);

        tracing::info!("Getting OData count from: {}", url);

        let response = self
            .client
            .get(&url)
            .basic_auth(&connection.login, Some(&connection.password))
            .send()
            .await?;

        if !response.status().is_success() {
            // Некоторые конфигурации УТ не поддерживают $count
            tracing::warn!("Failed to get collection count, continuing without it");
            return Ok(None);
        }

        let count_text = response.text().await?;
        let count = count_text.trim().parse::<i32>().ok();
        Ok(count)
    }
}

impl Default for UtODataClient {
    fn default() -> Self {
        Self::new()
    }
}
