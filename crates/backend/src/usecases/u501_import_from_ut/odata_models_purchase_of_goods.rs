use contracts::domain::a023_purchase_of_goods::aggregate::{
    PurchaseOfGoods, PurchaseOfGoodsLine,
};
use serde::{Deserialize, Serialize};

/// OData модель строки табличной части Товары документа ПриобретениеТоваровУслуг
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UtPurchaseLineOData {
    #[serde(rename = "Номенклатура_Key", default)]
    pub nomenclature_key: String,

    #[serde(rename = "Количество", default)]
    pub quantity: f64,

    #[serde(rename = "Цена", default)]
    pub price: f64,

    #[serde(rename = "СуммаСНДС", default)]
    pub amount_with_vat: f64,

    #[serde(rename = "СуммаНДС", default)]
    pub vat_amount: f64,

    #[serde(flatten)]
    #[serde(skip_serializing)]
    pub other: std::collections::HashMap<String, serde_json::Value>,
}

/// OData модель документа ПриобретениеТоваровУслуг из УТ 11
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtPurchaseOfGoodsOData {
    #[serde(rename = "Ref_Key", default)]
    pub ref_key: String,

    #[serde(rename = "Date", default)]
    pub date: String,

    #[serde(rename = "Number", default)]
    pub number: String,

    #[serde(rename = "Posted", default)]
    pub posted: bool,

    #[serde(rename = "DeletionMark", default)]
    pub deletion_mark: bool,

    /// UUID контрагента
    #[serde(rename = "Контрагент_Key", default)]
    pub counterparty_key: String,

    /// UUID склада (для постфильтрации по складу в Rust)
    #[serde(rename = "Склад_Key", default)]
    pub warehouse_key: String,

    /// Табличная часть Товары (возвращается автоматически, без $expand)
    #[serde(rename = "Товары", default)]
    pub goods: Vec<UtPurchaseLineOData>,

    #[serde(flatten)]
    #[serde(skip_serializing)]
    pub other: std::collections::HashMap<String, serde_json::Value>,
}

impl UtPurchaseOfGoodsOData {
    /// Извлечь дату документа в формате YYYY-MM-DD из OData datetime строки
    /// OData возвращает дату в формате "2024-01-15T00:00:00" или "2024-01-15T00:00:00Z"
    pub fn document_date(&self) -> String {
        self.date
            .split('T')
            .next()
            .unwrap_or(&self.date)
            .to_string()
    }

    /// Преобразование OData модели в агрегат PurchaseOfGoods
    pub fn to_aggregate(&self, connection_id: &str) -> Result<PurchaseOfGoods, String> {
        use uuid::Uuid;

        let id = Uuid::parse_str(&self.ref_key)
            .map_err(|e| format!("Invalid Ref_Key '{}': {}", self.ref_key, e))?;

        let lines: Vec<PurchaseOfGoodsLine> = self
            .goods
            .iter()
            .filter_map(|g| {
                if g.nomenclature_key.is_empty() {
                    return None;
                }
                Some(PurchaseOfGoodsLine {
                    nomenclature_key: g.nomenclature_key.clone(),
                    quantity: g.quantity,
                    price: g.price,
                    amount_with_vat: g.amount_with_vat,
                    vat_amount: g.vat_amount,
                })
            })
            .collect();

        let mut doc = PurchaseOfGoods::new_from_odata(
            id,
            self.number.clone(),
            self.document_date(),
            self.counterparty_key.clone(),
            lines,
            connection_id.to_string(),
        );
        doc.base.metadata.is_deleted = self.deletion_mark;

        Ok(doc)
    }
}

/// Ответ OData для списка документов ПриобретениеТоваровУслуг
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtPurchaseOfGoodsListResponse {
    pub value: Vec<UtPurchaseOfGoodsOData>,
}
