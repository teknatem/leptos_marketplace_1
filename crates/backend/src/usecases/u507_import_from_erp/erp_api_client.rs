use anyhow::Result;
use contracts::domain::a001_connection_1c::aggregate::Connection1CDatabase;
use contracts::domain::common::AggregateId;
use serde::{Deserialize, Serialize};

/// HTTP-клиент для работы с HTTP API ERP (1С)
pub struct ErpApiClient {
    client: reqwest::Client,
}

impl ErpApiClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .no_proxy()
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Получить список документов выпуска продукции за период
    pub async fn fetch_production_output(
        &self,
        connection: &Connection1CDatabase,
        date_from: &str,
        date_to: &str,
    ) -> Result<ProductionOutputResponse> {
        // Из URL вида "http://host:port/base/odata/standard.odata" получаем "http://host:port/base"
        // Если URL без /odata (напр. http://host:port/base) — используем как есть
        let base_url = connection.url.trim_end_matches('/');
        let trade_base = if let Some(pos) = base_url.find("/odata") {
            &base_url[..pos]
        } else {
            base_url
        };
        let url = format!(
            "{}/hs/mpi_api/production_output?date1={}&date2={}",
            trade_base,
            date_from,
            date_to
        );

        tracing::info!(
            "ERP API: GET {} (login='{}', password={})",
            url,
            connection.login,
            if connection.password.is_empty() { "EMPTY" } else { "***" }
        );

        if connection.login.trim().is_empty() {
            return Err(anyhow::anyhow!(
                "Логин не задан в настройках подключения 1С (id={})",
                connection.base.id.as_string()
            ));
        }

        let password_opt: Option<&str> = if connection.password.is_empty() {
            None
        } else {
            Some(&connection.password)
        };

        let response = self
            .client
            .get(&url)
            .basic_auth(&connection.login, password_opt)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Сетевая ошибка при запросе {}: {}", url, e))?;

        let status = response.status();
        tracing::info!("ERP API response: {} for {}", status, url);

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            let hint = if status.as_u16() == 401 {
                format!(
                    " | Совет: проверьте логин/пароль для пользователя '{}' в настройках 1С. \
                     Откройте URL в браузере: {} — если браузер запрашивает логин/пароль, \
                     введите те же данные что в настройках подключения.",
                    connection.login, url
                )
            } else {
                String::new()
            };
            return Err(anyhow::anyhow!(
                "HTTP {} при запросе {}: {}{}",
                status,
                url,
                body,
                hint
            ));
        }

        let result: ProductionOutputResponse = response.json().await
            .map_err(|e| anyhow::anyhow!("Ошибка парсинга JSON ответа от {}: {}", url, e))?;

        tracing::info!("ERP API: got {} documents", result.count);
        Ok(result)
    }
}

impl Default for ErpApiClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Ответ от API /hs/mpi_api/production_output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionOutputResponse {
    pub count: i32,
    #[serde(default)]
    pub data: Vec<ProductionOutputItem>,
}

/// Элемент выпуска продукции из API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionOutputItem {
    /// UUID документа из 1С
    pub id: String,
    /// Количество
    pub count: i64,
    /// Сумма себестоимости итого
    pub amount: f64,
    /// Наименование продукта
    pub description: String,
    /// Артикул
    pub article: String,
    /// Дата документа (YYYY-MM-DD)
    pub document_date: String,
    /// Номер документа
    pub document_no: String,
}
