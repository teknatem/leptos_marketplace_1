use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::repository::NomenclatureBarcodeEntry;

/// OData модель для развернутой номенклатуры (когда используем $expand)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtExpandedNomenclature {
    /// Артикул из справочника номенклатуры
    #[serde(rename = "Артикул", alias = "Article", default)]
    pub article: Option<String>,

    /// Игнорировать остальные поля номенклатуры
    #[serde(flatten)]
    #[serde(skip_serializing)]
    pub other: std::collections::HashMap<String, serde_json::Value>,
}

/// OData модель регистра сведений "Штрихкоды номенклатуры" из УТ 11
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtNomenclatureBarcodeOData {
    /// Штрихкод (измерение)
    #[serde(rename = "Штрихкод", alias = "Barcode", default)]
    pub barcode: String,

    /// Владелец штрихкода - ссылка на номенклатуру (измерение)
    /// Это может быть "Владелец_Key" или "Номенклатура_Key" в зависимости от конфигурации
    #[serde(rename = "Владелец_Key", alias = "Owner_Key", alias = "Номенклатура_Key", alias = "Nomenclature_Key", default)]
    pub owner_key: String,

    /// Номенклатура (если используем $expand=Номенклатура)
    #[serde(rename = "Номенклатура", alias = "Nomenclature", default)]
    pub nomenclature: Option<UtExpandedNomenclature>,

    /// Артикул (опционально, может быть в регистре как ресурс ИЛИ получен через $expand)
    #[serde(rename = "Артикул", alias = "Article", default)]
    pub article: Option<String>,

    /// Игнорировать дополнительные поля
    #[serde(flatten)]
    #[serde(skip_serializing)]
    pub other: std::collections::HashMap<String, serde_json::Value>,
}

impl UtNomenclatureBarcodeOData {
    /// Преобразование OData модели в entry для upsert
    pub fn to_entry(&self) -> Result<NomenclatureBarcodeEntry, String> {
        // Валидация штрихкода
        if self.barcode.is_empty() {
            return Err("Barcode cannot be empty".to_string());
        }

        // Парсинг UUID номенклатуры (владельца) - теперь может быть None
        let nomenclature_ref = if !self.owner_key.is_empty() {
            Some(
                Uuid::parse_str(&self.owner_key)
                    .map_err(|e| format!("Invalid nomenclature UUID: {}", e))?
                    .to_string()
            )
        } else {
            None
        };

        // Получить артикул: сначала из развернутой номенклатуры, потом из прямого поля
        let article = self.nomenclature
            .as_ref()
            .and_then(|n| n.article.clone())
            .or_else(|| self.article.clone());

        let now = chrono::Utc::now();

        Ok(NomenclatureBarcodeEntry {
            barcode: self.barcode.clone(),
            source: "1C".to_string(),
            nomenclature_ref,
            article,
            created_at: now,
            updated_at: now,
            is_active: true,
        })
    }

    /// Проверка, нужно ли обновлять существующую запись
    pub fn should_update(&self, existing: &crate::projections::p901_nomenclature_barcodes::repository::Model) -> bool {
        let nomenclature_ref = if !self.owner_key.is_empty() {
            Uuid::parse_str(&self.owner_key)
                .ok()
                .map(|u| u.to_string())
        } else {
            None
        };

        // Получить артикул: сначала из развернутой номенклатуры, потом из прямого поля
        let article = self.nomenclature
            .as_ref()
            .and_then(|n| n.article.clone())
            .or_else(|| self.article.clone());

        // Обновляем если изменилась номенклатура, артикул или запись неактивна
        existing.nomenclature_ref != nomenclature_ref
            || existing.article != article
            || !existing.is_active
    }
}

/// Ответ OData для списка штрихкодов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtNomenclatureBarcodeListResponse {
    pub value: Vec<UtNomenclatureBarcodeOData>,
}
