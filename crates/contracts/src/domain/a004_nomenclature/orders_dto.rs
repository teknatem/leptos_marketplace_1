use serde::{Deserialize, Serialize};

/// Строка объединённого списка заказов WB/YM, относящихся к номенклатуре
/// (напрямую по nomenclature_ref или через base_nomenclature_ref для
/// деривативных/вариантных позиций).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NomenclatureOrderRowDto {
    /// Id агрегата-заказа (a015_wb_orders / a013_ym_order) — для гиперссылки
    pub id: String,
    /// "WB" | "YM"
    pub marketplace: String,
    pub document_no: String,
    /// document_date (WB) / creation_date (YM)
    pub order_date: Option<String>,

    // Статус — заполняется только для соответствующего маркетплейса
    pub is_cancel: Option<bool>,
    pub is_supply: Option<bool>,
    pub is_realization: Option<bool>,
    pub line_status: Option<String>,
    pub status_norm: Option<String>,

    pub qty: f64,
    pub price_before_discount: Option<f64>,
    pub price_after_discount: Option<f64>,
    pub final_buyer_price: Option<f64>,
    pub dealer_price_ut: Option<f64>,
    pub margin_pro: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NomenclatureOrdersResponse {
    pub items: Vec<NomenclatureOrderRowDto>,
    pub total_count: usize,
}
