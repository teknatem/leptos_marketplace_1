//! Построение строк p915 (событий заказа) из регистраторов-источников.
//!
//! Каждый источник маппит свои данные в `Vec<Model>`. Идемпотентность
//! обеспечивает вызывающая сторона (delete-by-registrator перед insert).

use chrono::{DateTime, FixedOffset, Utc};
use uuid::Uuid;

use contracts::domain::a013_ym_order::aggregate::YmOrder;
use contracts::domain::a034_ym_realization::aggregate::YmRealization;
use contracts::projections::p915_mp_order_events::event::OrderEventType;
use contracts::shared::analytics::TurnoverLayer;

use super::repository::Model;
use crate::projections::p907_ym_payment_report::repository::Model as PaymentRow;

const REG_A013: &str = "a013_ym_order";
const REG_A034: &str = "a034_ym_realization";
const REG_P907: &str = "p907_ym_payment_report";

/// Источники платежа покупателя в p907 (совпадают с константами GL-билдера p907).
const BUYER_PAYMENT_SOURCE: &str = "Платёж покупателя";
const BUYER_PAYMENT_RETURN_SOURCE: &str = "Возврат платежа покупателя";

/// Текущее время в MSK (+03:00) в формате RFC3339.
pub fn now_msk() -> String {
    let msk = FixedOffset::east_opt(3 * 3600).expect("valid MSK offset");
    Utc::now().with_timezone(&msk).to_rfc3339()
}

/// UTC-момент → MSK-дата `YYYY-MM-DD`.
fn msk_date_from_utc(dt: &DateTime<Utc>) -> String {
    let msk = FixedOffset::east_opt(3 * 3600).expect("valid MSK offset");
    dt.with_timezone(&msk).format("%Y-%m-%d").to_string()
}

/// Дата-строка источника → `YYYY-MM-DD` (первые 10 символов). Пустая → None.
fn msk_date_str(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.len() < 10 {
        return None;
    }
    Some(trimmed.chars().take(10).collect())
}

#[allow(clippy::too_many_arguments)]
fn make_event(
    order_id: String,
    marketplace_product: Option<String>,
    event_date: String,
    event_type: OrderEventType,
    layer: &str,
    amount: Option<f64>,
    registrator_type: &str,
    registrator_ref: &str,
    connection_mp_ref: String,
    now: &str,
) -> Model {
    Model {
        id: Uuid::new_v4().to_string(),
        order_id,
        marketplace_product,
        event_date,
        event_type: event_type.as_str().to_string(),
        layer: layer.to_string(),
        amount,
        registrator_type: registrator_type.to_string(),
        registrator_ref: registrator_ref.to_string(),
        connection_mp_ref,
        created_at_msk: now.to_string(),
        updated_at_msk: now.to_string(),
    }
}

/// События уровня заказа из a013_ym_order (слой `oper`):
/// `order_placed` (creation_date) и `delivery` (delivery_date).
/// `marketplace_product = None` — события уровня заказа.
/// `shipment` пока НЕ создаётся: источник даты отгрузки отсутствует.
pub fn from_ym_order(order: &YmOrder, registrator_ref: &str) -> Vec<Model> {
    let now = now_msk();
    let layer = TurnoverLayer::Oper.as_str();
    let order_id = order.header.document_no.clone();
    let connection = order.header.connection_id.clone();
    let amount = order.header.total_amount;

    let mut events = Vec::new();

    if let Some(creation) = order.state.creation_date.as_ref() {
        events.push(make_event(
            order_id.clone(),
            None,
            msk_date_from_utc(creation),
            OrderEventType::OrderPlaced,
            layer,
            amount,
            REG_A013,
            registrator_ref,
            connection.clone(),
            &now,
        ));
    }

    // TODO: shipment — нет источника даты отгрузки в a013_ym_order.

    if let Some(delivery) = order.state.delivery_date.as_ref() {
        events.push(make_event(
            order_id.clone(),
            None,
            msk_date_from_utc(delivery),
            OrderEventType::Delivery,
            layer,
            amount,
            REG_A013,
            registrator_ref,
            connection.clone(),
            &now,
        ));
    }

    events
}

/// События реализации/возврата товара из a034_ym_realization (слой `ybuh`):
/// по строке `sales_lines` → `realization`, по строке `return_lines` →
/// `goods_return`. Дата = `header.document_date` (отдельной даты возврата
/// у строки нет). `marketplace_product` = распознанный a007 строки.
pub fn from_ym_realization(doc: &YmRealization, registrator_ref: &str) -> Vec<Model> {
    let now = now_msk();
    let layer = TurnoverLayer::Ybuh.as_str();
    let connection = doc.header.connection_id.clone();
    let Some(event_date) = msk_date_str(&doc.header.document_date) else {
        return Vec::new();
    };

    let mut events = Vec::new();

    let build_line = |event_type: OrderEventType,
                      line: &contracts::domain::a034_ym_realization::aggregate::YmRealizationLine,
                      events: &mut Vec<Model>| {
        let order_id = line
            .order_id
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty());
        let Some(order_id) = order_id else {
            tracing::debug!(
                "p915: пропуск строки a034 без order_id (sku={})",
                line.shop_sku
            );
            return;
        };
        events.push(make_event(
            order_id.to_string(),
            line.marketplace_product_ref.clone(),
            event_date.clone(),
            event_type,
            layer,
            Some(line.revenue_amount),
            REG_A034,
            registrator_ref,
            connection.clone(),
            &now,
        ));
    };

    for line in &doc.sales_lines {
        build_line(OrderEventType::Realization, line, &mut events);
    }
    for line in &doc.return_lines {
        build_line(OrderEventType::GoodsReturn, line, &mut events);
    }

    events
}

/// События оплаты/возврата оплаты из строки p907 (слой `fina`):
/// «Платёж покупателя» → `payment`, «Возврат платежа покупателя» →
/// `payment_return`; прочие источники события не порождают.
/// `marketplace_product = None` (оплата — уровень заказа, по решению дизайна).
pub fn from_ym_payment_row(row: &PaymentRow) -> Vec<Model> {
    let event_type = match row.transaction_source.as_deref().map(str::trim) {
        Some(BUYER_PAYMENT_SOURCE) => OrderEventType::Payment,
        Some(BUYER_PAYMENT_RETURN_SOURCE) => OrderEventType::PaymentReturn,
        _ => return Vec::new(),
    };

    let Some(order_id) = row.order_id else {
        return Vec::new();
    };
    let Some(event_date) = row.transaction_date.as_deref().and_then(msk_date_str) else {
        return Vec::new();
    };

    let now = now_msk();
    vec![make_event(
        order_id.to_string(),
        None,
        event_date,
        event_type,
        TurnoverLayer::Fina.as_str(),
        row.transaction_sum,
        REG_P907,
        &row.id,
        row.connection_mp_ref.clone(),
        &now,
    )]
}

/// Расчёт по заказу в банковском ордере: оплата поставщику (`is_return = false`)
/// или удержание ранее перечисленной оплаты при возврате (`is_return = true`).
pub struct SettledOrderEvent<'a> {
    pub order_id: &'a str,
    pub amount: f64,
    pub is_return: bool,
}

/// События «Дата оплаты поставщику» / «Возврат оплаты поставщику» (слой `fina`)
/// из проведённого банковского ордера a035_ym_settlement_recon: по одному
/// событию на расчёт. `payment_date` = bank_order_date (YYYY-MM-DD),
/// `amount` = сумма расчёта (для возврата отрицательная).
/// `marketplace_product = None` (событие уровня заказа).
pub fn from_supplier_settlement(
    orders: &[SettledOrderEvent<'_>],
    payment_date: &str,
    connection_mp_ref: &str,
    registrator_type: &str,
    registrator_ref: &str,
) -> Vec<Model> {
    let Some(event_date) = msk_date_str(payment_date) else {
        return Vec::new();
    };
    let now = now_msk();
    let layer = TurnoverLayer::Fina.as_str();
    orders
        .iter()
        .filter(|o| !o.order_id.trim().is_empty())
        .map(|o| {
            let event_type = if o.is_return {
                OrderEventType::SupplierPaymentReturn
            } else {
                OrderEventType::SupplierPayment
            };
            make_event(
                o.order_id.to_string(),
                None,
                event_date.clone(),
                event_type,
                layer,
                Some(o.amount),
                registrator_type,
                registrator_ref,
                connection_mp_ref.to_string(),
                &now,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::domain::a034_ym_realization::aggregate::{
        YmRealizationHeader, YmRealizationLine, YmRealizationSourceMeta, YmRealization,
    };

    fn realization_line(order_id: Option<&str>, is_return: bool, amount: f64) -> YmRealizationLine {
        YmRealizationLine {
            order_id: order_id.map(|s| s.to_string()),
            shop_sku: "SKU-1".to_string(),
            your_sku: None,
            marketplace_product_ref: Some("mp-uuid-1".to_string()),
            market_sku: None,
            offer_name: "Товар".to_string(),
            quantity: 1.0,
            revenue_amount: amount,
            is_return,
        }
    }

    fn realization(sales: Vec<YmRealizationLine>, returns: Vec<YmRealizationLine>) -> YmRealization {
        let header = YmRealizationHeader {
            document_no: "DOC-1".to_string(),
            document_date: "2026-05-10".to_string(),
            connection_id: "conn-1".to_string(),
            organization_id: "org-1".to_string(),
            marketplace_id: "mp-1".to_string(),
        };
        YmRealization::new_for_insert(
            header,
            sales,
            returns,
            YmRealizationSourceMeta {
                source: "ym_goods_realization".to_string(),
                fetched_at: "2026-05-10T00:00:00Z".to_string(),
            },
        )
    }

    #[test]
    fn realization_emits_realization_and_return_events() {
        let doc = realization(
            vec![realization_line(Some("100"), false, 1000.0)],
            vec![realization_line(Some("100"), true, 150.0)],
        );
        let events = from_ym_realization(&doc, "reg-a034");
        assert_eq!(events.len(), 2);

        let sale = events.iter().find(|e| e.event_type == "realization").unwrap();
        assert_eq!(sale.layer, "ybuh");
        assert_eq!(sale.order_id, "100");
        assert_eq!(sale.marketplace_product.as_deref(), Some("mp-uuid-1"));
        assert_eq!(sale.amount, Some(1000.0));
        assert_eq!(sale.event_date, "2026-05-10");
        assert_eq!(sale.registrator_type, "a034_ym_realization");

        let ret = events.iter().find(|e| e.event_type == "goods_return").unwrap();
        assert_eq!(ret.amount, Some(150.0));
    }

    #[test]
    fn realization_skips_lines_without_order_id() {
        let doc = realization(vec![realization_line(None, false, 500.0)], vec![]);
        let events = from_ym_realization(&doc, "reg-a034");
        assert!(events.is_empty());
    }

    #[test]
    fn payment_row_emits_payment_event() {
        let mut row = test_payment_row();
        row.transaction_source = Some("Платёж покупателя".to_string());
        row.order_id = Some(777);
        row.transaction_date = Some("2026-05-11 12:30".to_string());
        row.transaction_sum = Some(2500.0);

        let events = from_ym_payment_row(&row);
        assert_eq!(events.len(), 1);
        let ev = &events[0];
        assert_eq!(ev.event_type, "payment");
        assert_eq!(ev.layer, "fina");
        assert_eq!(ev.order_id, "777");
        assert_eq!(ev.event_date, "2026-05-11");
        assert_eq!(ev.amount, Some(2500.0));
        assert!(ev.marketplace_product.is_none());
    }

    #[test]
    fn supplier_settlement_emits_payment_and_return_events() {
        let orders = [
            SettledOrderEvent { order_id: "100", amount: 15894.0, is_return: false },
            SettledOrderEvent { order_id: "200", amount: -15894.0, is_return: true },
        ];
        let events =
            from_supplier_settlement(&orders, "2026-05-29", "conn-1", "a035_ym_settlement_recon", "reg-a035");
        assert_eq!(events.len(), 2);

        let pay = events.iter().find(|e| e.order_id == "100").unwrap();
        assert_eq!(pay.event_type, "supplier_payment");
        assert_eq!(pay.layer, "fina");
        assert_eq!(pay.amount, Some(15894.0));
        assert_eq!(pay.event_date, "2026-05-29");

        let ret = events.iter().find(|e| e.order_id == "200").unwrap();
        assert_eq!(ret.event_type, "supplier_payment_return");
        assert_eq!(ret.amount, Some(-15894.0));
        assert_eq!(ret.event_date, "2026-05-29");
    }

    #[test]
    fn payment_row_ignores_other_sources() {
        let mut row = test_payment_row();
        row.transaction_source = Some("Премия".to_string());
        row.order_id = Some(1);
        row.transaction_date = Some("2026-05-11 00:00".to_string());
        assert!(from_ym_payment_row(&row).is_empty());
    }

    fn test_payment_row() -> PaymentRow {
        PaymentRow {
            record_key: "rk-1".to_string(),
            id: "p907-uuid-1".to_string(),
            connection_mp_ref: "conn-1".to_string(),
            organization_ref: String::new(),
            business_id: None,
            partner_id: None,
            shop_name: None,
            inn: None,
            model: None,
            transaction_id: None,
            transaction_date: None,
            transaction_type: None,
            transaction_source: None,
            transaction_sum: None,
            payment_status: None,
            order_id: None,
            shop_order_id: None,
            order_creation_date: None,
            order_delivery_date: None,
            order_type: None,
            shop_sku: None,
            offer_or_service_name: None,
            count: None,
            act_id: None,
            act_date: None,
            bank_order_id: None,
            bank_order_date: None,
            bank_sum: None,
            claim_number: None,
            bonus_account_year_month: None,
            comments: None,
            marketplace_product_ref: None,
            marketplace_order_ref: None,
            nomenclature_ref: None,
            loaded_at_utc: String::new(),
            payload_version: 1,
        }
    }
}
