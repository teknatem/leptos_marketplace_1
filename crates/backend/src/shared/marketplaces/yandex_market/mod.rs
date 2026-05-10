use super::{MarketplaceClient, TestConnectionResult};
use async_trait::async_trait;
use contracts::domain::a006_connection_mp::aggregate::{AuthorizationType, ConnectionMPDto};
use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Клиент для работы с API Яндекс.Маркет
pub struct YandexMarketClient;

fn apply_yandex_auth(
    request: RequestBuilder,
    dto: &ConnectionMPDto,
) -> Result<RequestBuilder, TestConnectionResult> {
    let token = dto.api_key.trim();
    if token.is_empty() {
        return Err(TestConnectionResult {
            success: false,
            message: "API Key не может быть пустым".into(),
            details: None,
        });
    }

    match &dto.authorization_type {
        AuthorizationType::ApiKey => Ok(request.header("Api-Key", token)),
        AuthorizationType::OAuth2 => {
            Ok(request.header("Authorization", format!("Bearer {}", token)))
        }
        AuthorizationType::BasicAuth => Err(TestConnectionResult {
            success: false,
            message: "Basic Auth не поддерживается для Яндекс.Маркет API".into(),
            details: Some(
                "Выберите тип авторизации API Key или OAuth 2.0 для подключения Яндекс.Маркет."
                    .into(),
            ),
        }),
    }
}

fn compact_json(text: &str) -> Value {
    serde_json::from_str::<Value>(text).unwrap_or_else(|_| json!({ "raw": text }))
}

fn build_seller_info_details(
    dto: &ConnectionMPDto,
    campaigns: &Value,
    token_info: Option<&Value>,
) -> String {
    let campaigns_list = campaigns
        .get("campaigns")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let stores: Vec<Value> = campaigns_list
        .iter()
        .map(|campaign| {
            json!({
                "campaign_id": campaign.get("id").cloned().unwrap_or(Value::Null),
                "domain": campaign.get("domain").cloned().unwrap_or(Value::Null),
                "placement_type": campaign.get("placementType").cloned().unwrap_or(Value::Null),
                "api_availability": campaign.get("apiAvailability").cloned().unwrap_or(Value::Null),
                "business": campaign.get("business").cloned().unwrap_or(Value::Null),
                "client_id": campaign.get("clientId").cloned().unwrap_or(Value::Null),
            })
        })
        .collect();

    let configured_business_id = dto
        .business_account_id
        .as_deref()
        .filter(|value| !value.trim().is_empty());
    let configured_campaign_id = dto
        .supplier_id
        .as_deref()
        .filter(|value| !value.trim().is_empty());

    let summary = json!({
        "marketplace": "Yandex Market",
        "authorization_type": dto.authorization_type.as_str(),
        "configured_business_id": configured_business_id,
        "configured_campaign_id": configured_campaign_id,
        "stores_count": stores.len(),
        "stores": stores,
        "paging": campaigns.get("paging").cloned().unwrap_or(Value::Null),
        "pager": campaigns.get("pager").cloned().unwrap_or(Value::Null),
        "api_key_token": token_info
            .and_then(|info| info.pointer("/result/apiKey").cloned())
            .unwrap_or_else(|| {
                if matches!(&dto.authorization_type, AuthorizationType::ApiKey) {
                    Value::String("Не удалось получить сведения о токене".to_string())
                } else {
                    Value::String("Метод /v2/auth/token доступен только для API-Key токена".to_string())
                }
            }),
    });

    serde_json::to_string_pretty(&summary).unwrap_or_else(|_| summary.to_string())
}

impl YandexMarketClient {
    pub async fn seller_info(dto: &ConnectionMPDto) -> TestConnectionResult {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
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

        let campaigns_request = match apply_yandex_auth(
            client
                .get("https://api.partner.market.yandex.ru/v2/campaigns")
                .header("Content-Type", "application/json")
                .query(&[("limit", "100")]),
            dto,
        ) {
            Ok(request) => request,
            Err(result) => return result,
        };

        let campaigns_response = match campaigns_request.send().await {
            Ok(resp) => resp,
            Err(e) => {
                return TestConnectionResult {
                    success: false,
                    message: "Ошибка запроса к Яндекс.Маркет API".into(),
                    details: Some(format!("{}", e)),
                }
            }
        };

        let campaigns_status = campaigns_response.status();
        let campaigns_body = campaigns_response.text().await.unwrap_or_default();
        if !campaigns_status.is_success() {
            return TestConnectionResult {
                success: false,
                message: format!(
                    "Яндекс.Маркет API вернул ошибку (HTTP {})",
                    campaigns_status.as_u16()
                ),
                details: Some(campaigns_body),
            };
        }

        let campaigns_json = compact_json(&campaigns_body);
        let mut token_json: Option<Value> = None;

        if matches!(&dto.authorization_type, AuthorizationType::ApiKey) {
            let token_request = match apply_yandex_auth(
                client
                    .post("https://api.partner.market.yandex.ru/v2/auth/token")
                    .header("Content-Type", "application/json"),
                dto,
            ) {
                Ok(request) => request,
                Err(result) => return result,
            };

            match token_request.send().await {
                Ok(response) => {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    token_json = Some(if status.is_success() {
                        compact_json(&body)
                    } else {
                        json!({
                            "status": status.as_u16(),
                            "error": body,
                        })
                    });
                }
                Err(e) => {
                    token_json = Some(json!({
                        "error": format!("Ошибка запроса /v2/auth/token: {}", e),
                    }));
                }
            }
        }

        TestConnectionResult {
            success: true,
            message: "Информация о кабинете Яндекс.Маркет получена".into(),
            details: Some(build_seller_info_details(
                dto,
                &campaigns_json,
                token_json.as_ref(),
            )),
        }
    }
}

/// Ответ от API Яндекс.Маркет для получения информации о кампании
#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
struct YandexCampaignResponse {
    campaigns: Vec<YandexCampaign>,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
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
        let mut request_builder = match apply_yandex_auth(
            client.get(url).header("Content-Type", "application/json"),
            dto,
        ) {
            Ok(request) => request,
            Err(result) => return result,
        };

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
                message: format!("Яндекс.Маркет API вернул ошибку (HTTP {})", status.as_u16()),
                details: Some(error_text),
            };
        }

        // Пытаемся получить текст ответа и проверить валидность
        match response.text().await {
            Ok(text) => {
                // Попробуем распарсить как JSON для проверки валидности
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&text) {
                    // Проверяем, что это похоже на ответ с кампаниями
                    if json_value.get("campaigns").is_some() || json_value.get("result").is_some() {
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
