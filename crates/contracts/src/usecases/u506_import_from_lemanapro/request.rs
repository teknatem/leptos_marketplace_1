use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Запрос на импорт данных из ЛеманаПро
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRequest {
    /// ID подключения к маркетплейсу (ConnectionMP)
    pub connection_id: String,

    /// Список агрегатов для импорта (например, ["a007_marketplace_product"])
    pub target_aggregates: Vec<String>,

    /// Режим импорта (опционально, для будущего расширения)
    #[serde(default)]
    pub mode: ImportMode,

    /// Начало периода (включительно)
    #[serde(with = "serde_date")]
    #[serde(rename = "dateFrom")]
    pub date_from: NaiveDate,

    /// Конец периода (включительно)
    #[serde(with = "serde_date")]
    #[serde(rename = "dateTo")]
    pub date_to: NaiveDate,
}

// Local serde helper for NaiveDate as YYYY-MM-DD
mod serde_date {
    use chrono::NaiveDate;
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%Y-%m-%d";

    pub fn serialize<S>(date: &NaiveDate, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = date.format(FORMAT).to_string();
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDate::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ImportMode {
    /// Импорт из UI (интерактивный)
    #[default]
    Interactive,

    /// Фоновый импорт (по расписанию)
    Background,
}


