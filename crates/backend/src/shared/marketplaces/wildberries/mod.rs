use super::{MarketplaceClient, TestConnectionResult};
use async_trait::async_trait;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMPDto;

/// Клиент для работы с API Wildberries
pub struct WildberriesClient;

#[async_trait]
impl MarketplaceClient for WildberriesClient {
    async fn test_connection(dto: &ConnectionMPDto) -> TestConnectionResult {
        // Валидация входных данных
        if dto.api_key.trim().is_empty() {
            return TestConnectionResult {
                success: false,
                message: "API Key не может быть пустым".into(),
                details: None,
            };
        }

        // Создаем HTTP клиент
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                return TestConnectionResult {
                    success: false,
                    message: "Ошибка создания HTTP клиента".into(),
                    details: Some(format!("{}", e)),
                }
            }
        };

        // Очищаем API ключ от невидимых символов и пробелов
        let api_key = dto.api_key.trim().replace(['\n', '\r', '\t'], "");

        // Проверяем, что ключ не содержит невалидных символов для HTTP заголовка
        if !api_key.is_ascii() {
            return TestConnectionResult {
                success: false,
                message: "API ключ содержит недопустимые символы".into(),
                details: Some("Ключ должен содержать только ASCII символы".into()),
            };
        }

        // Используем ping endpoint для проверки доступа
        // Wildberries API использует домен seller-analytics-api
        let url = "https://seller-analytics-api.wildberries.ru/ping";

        // Выполняем GET запрос
        let response = match client
            .get(url)
            .header("Authorization", api_key.as_str())
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                // Формируем детальное сообщение об ошибке
                let error_msg = if e.is_timeout() {
                    format!("Превышено время ожидания ответа от Wildberries API (>10 сек)")
                } else if e.is_connect() {
                    format!("Не удалось установить соединение с Wildberries API. Ошибка: {}", e)
                } else if e.is_request() || e.is_builder() {
                    format!("Ошибка формирования запроса: {}", e)
                } else {
                    format!("Неизвестная ошибка: {}", e)
                };

                return TestConnectionResult {
                    success: false,
                    message: error_msg.clone(),
                    details: Some(format!("URL: {}, Ошибка: {:?}", url, e)),
                }
            }
        };

        // Проверяем статус ответа
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return TestConnectionResult {
                success: false,
                message: format!("Wildberries API вернул ошибку (HTTP {})", status.as_u16()),
                details: Some(error_text),
            };
        }

        // Пытаемся получить текст ответа
        match response.text().await {
            Ok(text) => {
                // Попробуем распарсить как JSON для проверки валидности
                if serde_json::from_str::<serde_json::Value>(&text).is_ok() {
                    TestConnectionResult {
                        success: true,
                        message: "Подключение к Wildberries успешно установлено".into(),
                        details: Some("API ключ валиден".into()),
                    }
                } else {
                    TestConnectionResult {
                        success: false,
                        message: "Ответ от Wildberries API не является валидным JSON".into(),
                        details: Some(text),
                    }
                }
            }
            Err(e) => TestConnectionResult {
                success: false,
                message: "Ошибка чтения ответа от Wildberries API".into(),
                details: Some(format!("{}", e)),
            },
        }
    }
}
