use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::repository::NomenclaturePriceEntry;

/// Ответ от HTTP API /hs/mpi_api/prices_plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricesPlanResponse {
    /// Общее количество записей
    pub count: i32,

    /// Плановые цены
    #[serde(default)]
    pub initial: Vec<PriceItem>,

    /// История цен
    #[serde(default)]
    pub history: Vec<PriceItem>,
}

/// Элемент цены из API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceItem {
    /// Период в формате "DD.MM.YYYY"
    pub period: String,

    /// Цена в формате "1234,56" (с запятой как разделитель)
    pub price: String,

    /// UUID номенклатуры
    #[serde(alias = "nomenclature_ref")]
    pub nomenklature_ref: String,
}

impl PriceItem {
    /// Преобразование в entry для записи в БД
    pub fn to_entry(&self) -> Result<NomenclaturePriceEntry, String> {
        // Парсинг UUID номенклатуры
        let nomenclature_ref = if !self.nomenklature_ref.is_empty() {
            Uuid::parse_str(&self.nomenklature_ref)
                .map_err(|e| {
                    format!(
                        "Invalid nomenclature UUID '{}': {}",
                        self.nomenklature_ref, e
                    )
                })?
                .to_string()
        } else {
            return Err("Nomenclature ref is empty".to_string());
        };

        // Парсинг периода из "DD.MM.YYYY" в "YYYY-MM-DD"
        let period = self.parse_period()?;

        // Парсинг цены из "1234,56" в f64
        let price = self.parse_price()?;

        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        Ok(NomenclaturePriceEntry {
            id,
            period,
            nomenclature_ref,
            price,
            created_at: now,
            updated_at: now,
        })
    }

    /// Парсинг периода из "DD.MM.YYYY" в "YYYY-MM-DD"
    fn parse_period(&self) -> Result<String, String> {
        let parts: Vec<&str> = self.period.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid period format: {}", self.period));
        }

        let day = parts[0];
        let month = parts[1];
        let year = parts[2];

        // Валидация
        if day.len() != 2 || month.len() != 2 || year.len() != 4 {
            return Err(format!("Invalid period format: {}", self.period));
        }

        Ok(format!("{}-{}-{}", year, month, day))
    }

    /// Парсинг цены из "1234,56" в f64
    fn parse_price(&self) -> Result<f64, String> {
        // Заменяем запятую на точку и убираем пробелы
        let price_str = self.price.replace(',', ".").replace(' ', "");
        price_str
            .parse::<f64>()
            .map_err(|e| format!("Invalid price '{}': {}", self.price, e))
    }

    /// Получить отладочную информацию о записи
    pub fn debug_info(&self) -> String {
        format!(
            "period={}, nomenclature={}, price={}",
            self.period, self.nomenklature_ref, self.price
        )
    }
}
