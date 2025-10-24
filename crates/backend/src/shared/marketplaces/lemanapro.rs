use async_trait::async_trait;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMPDto;

use super::{MarketplaceClient, TestConnectionResult};

/// Клиент для работы с API LemanaPro
pub struct LemanaProClient;

#[async_trait]
impl MarketplaceClient for LemanaProClient {
    /// Тестирование подключения к LemanaPro API
    async fn test_connection(dto: &ConnectionMPDto) -> TestConnectionResult {
        // Проверка наличия Bearer токена
        if dto.api_key.trim().is_empty() {
            return TestConnectionResult {
                success: false,
                message: "Bearer токен не указан".to_string(),
                details: Some("Для подключения к LemanaPro необходим Bearer токен (API Key)".to_string()),
            };
        }

        // Используем production API для теста (можно сделать конфигурируемым)
        let base_url = "https://api.lemanapro.ru";
        let url = format!("{}/b2bintegration-products/v1/products", base_url);

        // Создаем HTTP клиент
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                return TestConnectionResult {
                    success: false,
                    message: "Ошибка создания HTTP клиента".to_string(),
                    details: Some(e.to_string()),
                }
            }
        };

        // Делаем тестовый запрос (первая страница с 1 товаром)
        let response = match client
            .get(&url)
            .header("Authorization", format!("Bearer {}", dto.api_key))
            .header("Accept", "application/json")
            .query(&[("page", "1"), ("perPage", "1")])
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                return TestConnectionResult {
                    success: false,
                    message: "Ошибка подключения к API LemanaPro".to_string(),
                    details: Some(format!("Не удалось выполнить запрос: {}", e)),
                }
            }
        };

        let status = response.status();
        
        if status.is_success() {
            // Пытаемся распарсить ответ
            match response.text().await {
                Ok(body) => {
                    // Проверяем, что это валидный JSON с products
                    if body.contains("\"products\"") {
                        TestConnectionResult {
                            success: true,
                            message: "Подключение успешно".to_string(),
                            details: Some("API LemanaPro доступен, токен валиден".to_string()),
                        }
                    } else {
                        TestConnectionResult {
                            success: false,
                            message: "Некорректный формат ответа".to_string(),
                            details: Some("API вернул неожиданный формат данных".to_string()),
                        }
                    }
                }
                Err(e) => TestConnectionResult {
                    success: false,
                    message: "Ошибка чтения ответа".to_string(),
                    details: Some(e.to_string()),
                },
            }
        } else if status.as_u16() == 401 {
            TestConnectionResult {
                success: false,
                message: "Ошибка авторизации".to_string(),
                details: Some("Bearer токен недействителен или истек".to_string()),
            }
        } else {
            let error_body = response.text().await.unwrap_or_default();
            TestConnectionResult {
                success: false,
                message: format!("Ошибка API: HTTP {}", status.as_u16()),
                details: Some(error_body),
            }
        }
    }
}


