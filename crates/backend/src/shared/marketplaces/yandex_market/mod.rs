use super::{MarketplaceClient, TestConnectionResult};
use async_trait::async_trait;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMPDto;
use serde::{Deserialize, Serialize};

/// Клиент для работы с API Яндекс.Маркет
pub struct YandexMarketClient;

/// Ответ от API Яндекс.Маркет для получения информации о кампании
#[derive(Debug, Deserialize, Serialize)]
struct YandexCampaignResponse {
    campaigns: Vec<YandexCampaign>,
}

#[derive(Debug, Deserialize, Serialize)]
struct YandexCampaign {
    id: i64,
    domain: String,
}

#[async_trait]
impl MarketplaceClient for YandexMarketClient {
    async fn test_connection(dto: &ConnectionMPDto) -> TestConnectionResult {
        // Валидация входных данных
        if dto.api_key.trim().is_empty() {
            return TestConnectionResult {
                success: false,
                message: "API Key не может быть пустым".into(),
                details: None,
            };
        }

        // Для Яндекс.Маркет может потребоваться business_account_id
        let business_id = match &dto.business_account_id {
            Some(id) if !id.trim().is_empty() => id.clone(),
            _ => {
                // Если не указан, пробуем без него (для простых запросов)
                String::new()
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

        // Endpoint для получения информации о кампаниях (простой тестовый запрос)
        // Используем API v2.1
        let url = "https://api.partner.market.yandex.ru/campaigns";

        // Выполняем запрос
        let mut request_builder = client
            .get(url)
            .header("Authorization", format!("Bearer {}", &dto.api_key))
            .header("Content-Type", "application/json");

        // Если указан business_id, добавляем его в параметры
        if !business_id.is_empty() {
            request_builder = request_builder.query(&[("businessId", &business_id)]);
        }

        let response = match request_builder.send().await {
            Ok(resp) => resp,
            Err(e) => {
                return TestConnectionResult {
                    success: false,
                    message: "Ошибка при выполнении запроса к Яндекс.Маркет API".into(),
                    details: Some(format!("{}", e)),
                }
            }
        };

        // Проверяем статус ответа
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return TestConnectionResult {
                success: false,
                message: format!(
                    "Яндекс.Маркет API вернул ошибку (HTTP {})",
                    status.as_u16()
                ),
                details: Some(error_text),
            };
        }

        // Пытаемся получить текст ответа и проверить валидность
        match response.text().await {
            Ok(text) => {
                // Попробуем распарсить как JSON для проверки валидности
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&text) {
                    // Проверяем, что это похоже на ответ с кампаниями
                    if json_value.get("campaigns").is_some()
                        || json_value.get("result").is_some()
                    {
                        TestConnectionResult {
                            success: true,
                            message: "Подключение к Яндекс.Маркет успешно установлено".into(),
                            details: Some("API ключ валиден".into()),
                        }
                    } else {
                        TestConnectionResult {
                            success: true,
                            message: "Подключение к Яндекс.Маркет установлено".into(),
                            details: Some(
                                "Получен ответ от API, но структура может отличаться".into(),
                            ),
                        }
                    }
                } else {
                    TestConnectionResult {
                        success: false,
                        message: "Ответ от Яндекс.Маркет API не является валидным JSON".into(),
                        details: Some(text),
                    }
                }
            }
            Err(e) => TestConnectionResult {
                success: false,
                message: "Ошибка чтения ответа от Яндекс.Маркет API".into(),
                details: Some(format!("{}", e)),
            },
        }
    }
}
