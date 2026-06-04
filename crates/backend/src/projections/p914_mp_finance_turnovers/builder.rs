//! Построение строк p914 (слой `fina`) как зеркала GL-проводок.
//!
//! Каждая строка p914 строится 1:1 из GL-проводки слоя fina и совпадает с ней
//! по сумме, дате транзакции, `turnover_code` и основным измерениям. Источники
//! (p903/p907) поставляют дополнительные разрезы через [`FinanceTurnoverContext`].

use contracts::shared::analytics::TurnoverLayer;

use super::repository::Model;
use crate::general_ledger::repository::Model as GeneralLedgerModel;

/// Дополнительные разрезы строки p914, которых нет в GL-проводке и которые
/// поставляет источник (строка p903/p907).
#[derive(Debug, Clone, Default)]
pub struct FinanceTurnoverContext {
    pub nomenclature_ref: Option<String>,
    pub marketplace_product_ref: Option<String>,
    pub order_key: String,
    /// UUID агрегата-заказа (a015/a013), если оборот связан с заказом.
    pub order_ref: Option<String>,
    /// Тип документа-заказа: a015_wb_orders / a013_ym_order.
    pub order_registrator_type: Option<String>,
    /// URL — юрлицо, FIZ — физлицо.
    pub customer_kind: Option<String>,
    /// Схема исполнения (FBO/FBS/FBW и т.п.).
    pub fulfillment_type: Option<String>,
    pub quantity: Option<f64>,
}

/// Выводит `event_kind` из `turnover_code` проводки. Едина для всех источников,
/// чтобы один и тот же оборот всегда классифицировался одинаково.
pub fn event_kind_for_turnover(turnover_code: &str) -> &'static str {
    use contracts::shared::analytics::EventKind;
    if turnover_code == "customer_return" || turnover_code.ends_with("_storno") {
        EventKind::Returned.as_str()
    } else if turnover_code.starts_with("customer_revenue") {
        EventKind::Sold.as_str()
    } else if turnover_code.starts_with("qty_ordered") {
        EventKind::Ordered.as_str()
    } else if turnover_code.contains("adjustment") {
        EventKind::Adjustment.as_str()
    } else {
        EventKind::Fee.as_str()
    }
}

/// Текущее время в часовом поясе MSK (+03:00) в формате RFC3339.
pub fn now_msk() -> String {
    use chrono::{FixedOffset, Utc};
    let msk = FixedOffset::east_opt(3 * 3600).expect("valid MSK offset");
    Utc::now().with_timezone(&msk).to_rfc3339()
}

/// Строит строку p914 из GL-проводки слоя fina и контекста источника.
///
/// Возвращает `None`, если проводка не относится к слою fina — p914 зеркалит
/// только fina-движения.
pub fn from_general_ledger_entry(
    gl: &GeneralLedgerModel,
    ctx: &FinanceTurnoverContext,
) -> Option<Model> {
    if gl.layer != TurnoverLayer::Fina.as_str() {
        return None;
    }

    let now = now_msk();
    Some(Model {
        id: gl.id.clone(),
        transaction_date: gl.entry_date.clone(),
        general_ledger_ref: Some(gl.id.clone()),
        registrator_type: gl.registrator_type.clone(),
        registrator_ref: gl.registrator_ref.clone(),
        connection_mp_ref: gl.connection_mp_ref.clone().unwrap_or_default(),
        nomenclature_ref: ctx.nomenclature_ref.clone(),
        marketplace_product_ref: ctx.marketplace_product_ref.clone(),
        turnover_code: gl.turnover_code.clone(),
        order_key: ctx.order_key.clone(),
        order_ref: ctx.order_ref.clone(),
        order_registrator_type: ctx.order_registrator_type.clone(),
        event_kind: event_kind_for_turnover(&gl.turnover_code).to_string(),
        customer_kind: ctx.customer_kind.clone(),
        fulfillment_type: ctx.fulfillment_type.clone(),
        layer: gl.layer.clone(),
        entity: gl.entity.clone(),
        amount: gl.amount,
        quantity: ctx.quantity,
        created_at_msk: now.clone(),
        updated_at_msk: now,
    })
}

/// Строит набор строк p914 из набора GL-проводок одного источника, используя
/// общий контекст. Не-fina проводки пропускаются.
pub fn from_general_ledger_entries(
    entries: &[GeneralLedgerModel],
    ctx: &FinanceTurnoverContext,
) -> Vec<Model> {
    entries
        .iter()
        .filter_map(|gl| from_general_ledger_entry(gl, ctx))
        .collect()
}
