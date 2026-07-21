//! DTO проекции `p916_mp_sales_funnel_turnovers` (универсальная воронка продаж МП).
//!
//! Строка — знаковое движение воронки от одного регистратора на `товар × дата × кабинет`.
//! Две стадии (`FunnelStage`): маркетинговая (верх воронки из a036) и fulfillment
//! (заказ→завершение из a015/a012). Агрегация — SUM метрик на чтении.

use serde::{Deserialize, Serialize};

/// Стадия воронки (тег строки движения).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunnelStage {
    /// Верх воронки (показы/переходы/корзина/заказы-воронки) — источник a036.
    Marketing,
    /// Заказ → завершение (заказы/отмены/выкупы/возвраты) — источники a015/a012.
    Fulfillment,
}

impl FunnelStage {
    /// Каноническая строка стадии (хранится в БД, поле `stage`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Marketing => "marketing",
            Self::Fulfillment => "fulfillment",
        }
    }

    /// Разбор строки стадии (None — неизвестное значение).
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "marketing" => Some(Self::Marketing),
            "fulfillment" => Some(Self::Fulfillment),
            _ => None,
        }
    }

    /// Человекочитаемая RU-метка для UI.
    pub fn label_ru(self) -> &'static str {
        match self {
            Self::Marketing => "Маркетинговая воронка",
            Self::Fulfillment => "Заказ → завершение",
        }
    }
}

/// Плоское зеркало строки движения воронки (для чтения/выгрузки).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpFunnelTurnoverDto {
    pub id: String,
    pub stage: String,
    /// Ось когорты: дата заказа (для стадии 1 — день воронки), YYYY-MM-DD.
    pub cohort_date: String,
    /// Ось потока: дата транзакции события (для стадии 1 = cohort_date), YYYY-MM-DD.
    pub event_date: String,
    pub connection_mp_ref: String,
    pub marketplace_product_ref: Option<String>,
    pub nomenclature_ref: Option<String>,
    pub nm_id: Option<i64>,
    pub registrator_type: String,
    pub registrator_ref: String,

    // стадия 1 (маркетинговая воронка):
    pub show_count: Option<i64>,
    pub open_count: i64,
    pub cart_count: i64,
    pub wishlist_count: i64,
    pub funnel_order_count: i64,
    pub funnel_order_sum: f64,

    // стадия 2 (fulfillment/когорта):
    pub order_count: i64,
    pub order_sum: f64,
    pub cancel_count: i64,
    pub cancel_sum: f64,
    pub buyout_count: i64,
    pub buyout_sum: f64,
    pub return_count: i64,
    pub return_sum: f64,

    pub created_at_msk: String,
    pub updated_at_msk: String,
}

/// Строка агрегированной воронки `товар × дата` (после SUM по движениям).
/// `date` — выбранная ось (когортная или потоковая) в зависимости от запроса.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MpFunnelAggRow {
    pub date: String,
    pub connection_mp_ref: String,
    pub marketplace_product_ref: Option<String>,
    pub nomenclature_ref: Option<String>,
    pub nm_id: Option<i64>,

    pub show_count: i64,
    pub open_count: i64,
    pub cart_count: i64,
    pub wishlist_count: i64,
    pub funnel_order_count: i64,
    pub funnel_order_sum: f64,

    pub order_count: i64,
    pub order_sum: f64,
    pub cancel_count: i64,
    pub cancel_sum: f64,
    pub buyout_count: i64,
    pub buyout_sum: f64,
    pub return_count: i64,
    pub return_sum: f64,
}

/// Ось агрегации при чтении воронки.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunnelDateAxis {
    /// По дате заказа (когорта/винтаж).
    Cohort,
    /// По дате транзакции (поток/касса).
    Event,
}

impl Default for FunnelDateAxis {
    fn default() -> Self {
        Self::Cohort
    }
}

/// Запрос агрегированной воронки с фильтрами.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MpFunnelListRequest {
    #[serde(default)]
    pub date_from: Option<String>,
    #[serde(default)]
    pub date_to: Option<String>,
    #[serde(default)]
    pub connection_mp_ref: Option<String>,
    #[serde(default)]
    pub nm_id: Option<i64>,
    /// Ось агрегации (по умолчанию — когортная, по дате заказа).
    #[serde(default)]
    pub axis: FunnelDateAxis,
    #[serde(default)]
    pub offset: Option<u64>,
    #[serde(default)]
    pub limit: Option<u64>,
}

/// Запрос «пересобрать воронку за период»: перепровести источники (a015/a012) и
/// пересобрать стадию 1 (a036) из сохранённых документов. Пустой список кабинетов —
/// все кабинеты, встреченные в периоде.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FunnelRebuildRequest {
    pub date_from: String,
    pub date_to: String,
    #[serde(default)]
    pub connection_mp_refs: Vec<String>,
}

/// Диагностическая сводка воронки за период (SUM движений p916 по когортной оси).
/// Конверсии/доли не хранятся — считаются потребителем на чтении.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FunnelPeriodSummary {
    pub date_from: String,
    pub date_to: String,

    // Верх воронки (стадия marketing):
    pub show_count: i64,
    pub open_count: i64,
    pub cart_count: i64,
    pub wishlist_count: i64,
    pub funnel_order_count: i64,
    pub funnel_order_sum: f64,

    // Fulfillment (стадия fulfillment):
    pub order_count: i64,
    pub order_sum: f64,
    pub cancel_count: i64,
    pub cancel_sum: f64,
    pub buyout_count: i64,
    pub buyout_sum: f64,
    pub return_count: i64,
    pub return_sum: f64,

    // Объём данных проекции по стадиям (строк движений):
    pub marketing_rows: i64,
    pub fulfillment_rows: i64,
}
