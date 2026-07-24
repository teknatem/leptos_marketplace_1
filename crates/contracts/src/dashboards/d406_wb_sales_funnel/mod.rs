//! DTO дашборда d406 «Воронка продаж WB» — потребитель проекции p916.
//!
//! Плоская таблица `товар × дата` по выбранной оси (когорта/событие) с метриками воронки
//! (показы всего/платные/бесплатные → переходы → корзина → заказы → выкупы/возвраты/отмены)
//! и производными конверсиями, а также итогами за период. Конверсии считаются на чтении
//! (в проекции не хранятся).

use serde::{Deserialize, Serialize};

pub use crate::projections::p916_mp_sales_funnel_turnovers::dto::FunnelDateAxis;

/// Канал трафика для фильтра воронки. Ответ всегда содержит и total, и `paid_*`;
/// выбор канала применяется на клиенте (all=total, paid=paid_*, free=total−paid).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunnelChannel {
    /// Весь трафик (total).
    All,
    /// Только платный (реклама).
    Paid,
    /// Только органический (total − paid).
    Free,
}

impl Default for FunnelChannel {
    fn default() -> Self {
        Self::All
    }
}

/// Параметры запроса воронки.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WbSalesFunnelRequest {
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
    /// Канал трафика (эхо; фактический выбор применяется на клиенте).
    #[serde(default)]
    pub channel: FunnelChannel,
}

/// Производные конверсии/доли воронки (проценты 0..100). None — знаменатель = 0.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WbSalesFunnelConversions {
    /// Переход → корзина = cart / open.
    pub open_to_cart: Option<f64>,
    /// Корзина → заказ = order / cart.
    pub cart_to_order: Option<f64>,
    /// Заказ → выкуп = buyout / order.
    pub order_to_buyout: Option<f64>,
    /// Доля отмен = cancel / order.
    pub cancel_rate: Option<f64>,
}

impl WbSalesFunnelConversions {
    /// Собрать конверсии из аддитивных метрик (0 в знаменателе → None).
    /// Под канальным фильтром вызывается от метрик выбранного канала (all/paid/free).
    pub fn from_metrics(open: i64, cart: i64, order: i64, buyout: i64, cancel: i64) -> Self {
        let pct = |num: i64, den: i64| -> Option<f64> {
            if den > 0 {
                Some(num as f64 / den as f64 * 100.0)
            } else {
                None
            }
        };
        Self {
            open_to_cart: pct(cart, open),
            cart_to_order: pct(order, cart),
            order_to_buyout: pct(buyout, order),
            cancel_rate: pct(cancel, order),
        }
    }
}

/// Аддитивные метрики воронки (SUM за строку/период). «Всего показов» = free + paid.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WbSalesFunnelMetrics {
    pub show_free_count: i64,
    pub show_paid_count: i64,
    pub show_total_count: i64,
    /// Доступность органических показов (a040) в срезе: `false` → показывать `N/A`, не `0`.
    #[serde(default)]
    pub show_free_available: bool,
    /// Доступность платных показов (a026) в срезе: `false` → `N/A`.
    #[serde(default)]
    pub show_paid_available: bool,
    /// Доступность «всего показов» = есть хотя бы один источник показов (free||paid).
    #[serde(default)]
    pub show_total_available: bool,
    /// Доступность рекламных данных (a026/p913) в срезе — для N/A платной/бесплатной стороны
    /// на переходах/корзине/заказах/выкупах/отменах/возвратах.
    #[serde(default)]
    pub advert_available: bool,
    pub open_count: i64,
    pub cart_count: i64,
    /// Платные переходы (реклама a026, clicks); free = open − paid (на клиенте).
    #[serde(default)]
    pub paid_open_count: i64,
    /// Платная корзина (реклама a026, atbs); free = cart − paid (на клиенте).
    #[serde(default)]
    pub paid_cart_count: i64,
    pub wishlist_count: i64,
    /// Заказы «глазами воронки» a036 (маркетинговый счётчик, ≠ order_count).
    pub funnel_order_count: i64,
    pub funnel_order_sum: f64,
    /// Фактические заказы (a015).
    pub order_count: i64,
    pub order_sum: f64,
    /// Платные (srid ∈ p913): заказы/выкупы/отмены/возвраты + суммы. Free = total − paid (клиент).
    #[serde(default)]
    pub paid_order_count: i64,
    #[serde(default)]
    pub paid_order_sum: f64,
    pub cancel_count: i64,
    pub cancel_sum: f64,
    #[serde(default)]
    pub paid_cancel_count: i64,
    #[serde(default)]
    pub paid_cancel_sum: f64,
    pub buyout_count: i64,
    pub buyout_sum: f64,
    #[serde(default)]
    pub paid_buyout_count: i64,
    #[serde(default)]
    pub paid_buyout_sum: f64,
    pub return_count: i64,
    pub return_sum: f64,
    #[serde(default)]
    pub paid_return_count: i64,
    #[serde(default)]
    pub paid_return_sum: f64,
}

/// Строка воронки `товар × дата` с именами (джойн a004/a007) и конверсиями.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WbSalesFunnelRow {
    /// Дата выбранной оси, YYYY-MM-DD.
    pub date: String,
    pub connection_mp_ref: String,
    pub nm_id: Option<i64>,
    pub marketplace_product_ref: Option<String>,
    pub nomenclature_ref: Option<String>,
    /// Человекочитаемое название маркетплейса (a006→a005), напр. «Wildberries».
    pub marketplace: Option<String>,
    /// Код типа маркетплейса для бейджа (напр. `mp-wb`/`mp-ozon`/`mp-ym`).
    pub marketplace_code: Option<String>,
    /// Артикул товара (a004.article), если найден на чтении.
    pub article: Option<String>,
    /// Наименование товара (a004.description) — для тултипа; без склейки с артикулом.
    pub product_name: Option<String>,
    pub brand: Option<String>,
    pub metrics: WbSalesFunnelMetrics,
    pub conversions: WbSalesFunnelConversions,
}

/// Ответ дашборда: эхо фильтров, строки и итоги за период.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WbSalesFunnelResponse {
    pub filters: WbSalesFunnelRequest,
    pub rows: Vec<WbSalesFunnelRow>,
    pub totals: WbSalesFunnelMetrics,
    pub totals_conversions: WbSalesFunnelConversions,
}

/// Канал заказа для drilldown-фильтра воронки.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunnelOrderChannel {
    /// Пришёл по рекламному каналу (srid ∈ атрибуции p913).
    Paid,
    /// Органический (заказ не найден в атрибуции рекламы).
    Free,
    /// Оба канала.
    All,
}

impl Default for FunnelOrderChannel {
    fn default() -> Self {
        Self::All
    }
}

/// Параметры drilldown: конкретные заказы одной ячейки воронки (`nm_id × дата`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WbSalesFunnelOrdersRequest {
    pub connection_mp_ref: String,
    pub nm_id: i64,
    /// Дата заказа (когорта), YYYY-MM-DD.
    pub date: String,
    /// Фильтр по каналу (по умолчанию — все).
    #[serde(default)]
    pub channel: FunnelOrderChannel,
}

/// Один заказ в drilldown воронки с меткой канала.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WbSalesFunnelOrderItem {
    /// srid заказа (a015 document_no).
    pub srid: String,
    /// Дата заказа (MSK), YYYY-MM-DD.
    pub order_date: String,
    /// Сумма заказа (allocation_basis: price_with_disc / finished_price / price).
    pub amount: f64,
    pub is_cancel: bool,
    /// `true` — платный (реклама), `false` — органический.
    pub is_paid: bool,
    /// Код рекламной кампании (advert_id), если заказ платный.
    pub advert_campaign: Option<String>,
}

/// Ответ drilldown: список заказов и счётчики по каналам.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WbSalesFunnelOrdersResponse {
    pub items: Vec<WbSalesFunnelOrderItem>,
    pub paid_count: i64,
    pub free_count: i64,
}
