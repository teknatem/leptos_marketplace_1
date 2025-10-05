use serde::{Deserialize, Serialize};

/// Источник данных для агрегата
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Origin {
    /// 1C:Enterprise (GUID-based IDs)
    C1,
    /// Bitrix24
    Bitrix,
    /// Ozon Marketplace
    Ozon,
    /// Собственная система
    #[serde(rename = "self")]
    Self_,
}

impl Origin {
    pub fn as_str(&self) -> &'static str {
        match self {
            Origin::C1 => "1c",
            Origin::Bitrix => "bitrix",
            Origin::Ozon => "ozon",
            Origin::Self_ => "self",
        }
    }
}

impl std::fmt::Display for Origin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
