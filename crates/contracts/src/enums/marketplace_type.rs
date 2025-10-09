use serde::{Deserialize, Serialize};

/// Типы маркетплейсов
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketplaceType {
    Ozon,
    Wildberries,
    YandexMarket,
    Kuper,
    LemanaPro,
}

impl MarketplaceType {
    /// Получить код маркетплейса
    pub fn code(&self) -> &'static str {
        match self {
            MarketplaceType::Ozon => "mp-ozon",
            MarketplaceType::Wildberries => "mp-wb",
            MarketplaceType::YandexMarket => "mp-ym",
            MarketplaceType::Kuper => "mp-kuper",
            MarketplaceType::LemanaPro => "mp-lemana",
        }
    }

    /// Получить человекочитаемое название
    pub fn display_name(&self) -> &'static str {
        match self {
            MarketplaceType::Ozon => "Ozon",
            MarketplaceType::Wildberries => "Wildberries",
            MarketplaceType::YandexMarket => "Яндекс Маркет",
            MarketplaceType::Kuper => "Купер",
            MarketplaceType::LemanaPro => "ЛеманаПро",
        }
    }

    /// Получить все типы маркетплейсов
    pub fn all() -> Vec<MarketplaceType> {
        vec![
            MarketplaceType::Ozon,
            MarketplaceType::Wildberries,
            MarketplaceType::YandexMarket,
            MarketplaceType::Kuper,
            MarketplaceType::LemanaPro,
        ]
    }

    /// Парсинг из строки
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "mp-ozon" => Some(MarketplaceType::Ozon),
            "mp-wb" => Some(MarketplaceType::Wildberries),
            "mp-ym" => Some(MarketplaceType::YandexMarket),
            "mp-kuper" => Some(MarketplaceType::Kuper),
            "mp-lemana" => Some(MarketplaceType::LemanaPro),
            _ => None,
        }
    }
}

impl ToString for MarketplaceType {
    fn to_string(&self) -> String {
        self.code().to_string()
    }
}
