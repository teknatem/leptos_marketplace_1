pub mod ozon;
pub mod wildberries;
pub mod yandex_market;
pub mod lemanapro;

use async_trait::async_trait;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMPDto;
use contracts::enums::marketplace_type::MarketplaceType;

/// Результат тестирования подключения к маркетплейсу
#[derive(Debug, Clone)]
pub struct TestConnectionResult {
    pub success: bool,
    pub message: String,
    pub details: Option<String>,
}

/// Трейт для клиента маркетплейса
#[async_trait]
pub trait MarketplaceClient {
    /// Тестирование подключения к маркетплейсу
    async fn test_connection(dto: &ConnectionMPDto) -> TestConnectionResult;
}

/// Определение типа маркетплейса по коду или описанию
pub fn get_marketplace_type(marketplace_code: &str) -> Option<MarketplaceType> {
    let code_lower = marketplace_code.to_lowercase();

    // Сначала проверяем точные совпадения кодов
    if let Some(mp_type) = MarketplaceType::from_code(&code_lower) {
        return Some(mp_type);
    }

    // Затем проверяем частичные совпадения
    if code_lower.contains("ozon") || code_lower.contains("озон") {
        return Some(MarketplaceType::Ozon);
    }
    if code_lower.contains("wildberries") || code_lower.contains("вб") || code_lower.contains("wb") {
        return Some(MarketplaceType::Wildberries);
    }
    if code_lower.contains("yandex") || code_lower.contains("яндекс") || code_lower.contains("market") || code_lower.contains("ym") {
        return Some(MarketplaceType::YandexMarket);
    }
    if code_lower.contains("kuper") || code_lower.contains("купер") {
        return Some(MarketplaceType::Kuper);
    }
    if code_lower.contains("lemana") || code_lower.contains("лемана") {
        return Some(MarketplaceType::LemanaPro);
    }

    None
}


/// Основная функция для тестирования подключения к маркетплейсу
pub async fn test_marketplace_connection(
    marketplace_code: &str,
    dto: &ConnectionMPDto,
) -> TestConnectionResult {
    let marketplace_type = match get_marketplace_type(marketplace_code) {
        Some(t) => t,
        None => {
            return TestConnectionResult {
                success: false,
                message: format!("Неизвестный тип маркетплейса: {}", marketplace_code),
                details: None,
            }
        }
    };

    match marketplace_type {
        MarketplaceType::Ozon => ozon::OzonClient::test_connection(dto).await,
        MarketplaceType::Wildberries => wildberries::WildberriesClient::test_connection(dto).await,
        MarketplaceType::YandexMarket => {
            yandex_market::YandexMarketClient::test_connection(dto).await
        }
        MarketplaceType::Kuper => TestConnectionResult {
            success: false,
            message: "Интеграция с Купер пока не реализована".into(),
            details: None,
        },
        MarketplaceType::LemanaPro => lemanapro::LemanaProClient::test_connection(dto).await,
    }
}
