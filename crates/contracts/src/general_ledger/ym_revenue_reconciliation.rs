//! Сверка выручки YM: слой `fina` (p907 взаиморасчёты) против слоя `ybuh`
//! (a034 официальный отчёт о реализации). Сравнивается нетто-выручка
//! (`customer_revenue` + `customer_revenue_storno`) за период по дням/кабинетам.

use serde::{Deserialize, Serialize};

/// Группировка периода для сверки.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum YmRevenueReconGroup {
    Day,
    Month,
}

impl Default for YmRevenueReconGroup {
    fn default() -> Self {
        Self::Day
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmRevenueReconQuery {
    pub date_from: String,
    pub date_to: String,
    #[serde(default)]
    pub connection_mp_ref: Option<String>,
    #[serde(default)]
    pub group: YmRevenueReconGroup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmRevenueReconRow {
    /// Ключ периода: дата (YYYY-MM-DD) или месяц (YYYY-MM).
    pub period: String,
    pub connection_mp_ref: Option<String>,
    pub connection_name: Option<String>,
    /// Нетто-выручка слоя fina (p907).
    pub fina_net: f64,
    /// Нетто-выручка слоя ybuh (a034 отчёт о реализации).
    pub ybuh_net: f64,
    /// Разница сумм fina − ybuh.
    pub delta: f64,
    /// Нетто-количество слоя fina (p907).
    pub fina_qty: f64,
    /// Нетто-количество слоя ybuh (a034).
    pub ybuh_qty: f64,
    /// Разница количеств fina − ybuh.
    pub qty_delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmRevenueReconResponse {
    pub rows: Vec<YmRevenueReconRow>,
    pub total_fina_net: f64,
    pub total_ybuh_net: f64,
    pub total_delta: f64,
    pub total_fina_qty: f64,
    pub total_ybuh_qty: f64,
    pub total_qty_delta: f64,
}
