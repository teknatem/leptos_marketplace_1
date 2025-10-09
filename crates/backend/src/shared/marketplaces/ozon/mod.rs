use super::{MarketplaceClient, TestConnectionResult};
use async_trait::async_trait;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMPDto;

/// Клиент для работы с API Ozon
pub struct OzonClient;

#[async_trait]
impl MarketplaceClient for OzonClient {
    async fn test_connection(dto: &ConnectionMPDto) -> TestConnectionResult {
        // Валидация входных данных
        if dto.api_key.trim().is_empty() {
            return TestConnectionResult {
                success: false,
                message: "API Key не может быть пустым".into(),
                details: None,
            };
        }

        // Для Ozon требуется Client-Id (supplier_id)
        let client_id = match &dto.supplier_id {
            Some(id) if !id.trim().is_empty() => id.clone(),
            _ => {
                return TestConnectionResult {
                    success: false,
                    message: "Для Ozon требуется ID Поставщика (Client-Id)".into(),
                    details: None,
                }
            }
        };

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

        // Используем endpoint для получения ролей пользователя
        // Это легкий метод API для проверки валидности ключей
        let url = "https://api-seller.ozon.ru/v1/roles";

        // Выполняем POST запрос (большинство методов Ozon используют POST)
        let response = match client
            .post(url)
            .header("Client-Id", &client_id)
            .header("Api-Key", &dto.api_key)
            .header("Content-Type", "application/json")
            .body("{}")
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                // Формируем детальное сообщение об ошибке
                let error_msg = if e.is_timeout() {
                    format!("Превышено время ожидания ответа от Ozon API (>10 сек). Проверьте доступ к api-seller.ozon.ru")
                } else if e.is_connect() {
                    format!("Не удалось установить соединение с Ozon API. Ошибка: {}", e)
                } else if e.is_request() {
                    format!("Ошибка формирования запроса: {}", e)
                } else {
                    format!("Неизвестная ошибка: {}", e)
                };

                return TestConnectionResult {
                    success: false,
                    message: error_msg.clone(),
                    details: Some(format!("URL: {}, Client-Id: {}, Ошибка: {:?}", url, client_id, e)),
                }
            }
        };

        // Проверяем статус ответа
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return TestConnectionResult {
                success: false,
                message: format!("Ozon API вернул ошибку (HTTP {})", status.as_u16()),
                details: Some(error_text),
            };
        }

        // Пытаемся получить текст ответа и проверить, что это валидный JSON
        match response.text().await {
            Ok(text) => {
                // Проверяем, что ответ является валидным JSON
                if serde_json::from_str::<serde_json::Value>(&text).is_ok() {
                    TestConnectionResult {
                        success: true,
                        message: "Подключение к Ozon успешно установлено".into(),
                        details: Some("API ключ и Client-Id валидны".into()),
                    }
                } else {
                    TestConnectionResult {
                        success: false,
                        message: "Ответ от Ozon API не является валидным JSON".into(),
                        details: Some(text),
                    }
                }
            }
            Err(e) => TestConnectionResult {
                success: false,
                message: "Ошибка чтения ответа от Ozon API".into(),
                details: Some(format!("{}", e)),
            },
        }
    }
}
