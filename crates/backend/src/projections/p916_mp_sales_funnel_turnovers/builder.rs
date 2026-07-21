//! Построение строк p916 (движений воронки) из регистраторов-источников.
//!
//! Каждый источник маппит свои данные в `Vec<Model>`. Идемпотентность обеспечивает
//! вызывающая сторона (delete-by-registrator перед insert). Пустые строки не пишутся
//! (разреженность — контроль размера проекции).
//!
//! Стадия 1 (marketing) — из a036: cohort_date = event_date = день воронки.
//! Стадия 2 (fulfillment) — из a015/a012: cohort_date = дата заказа (когорта),
//! event_date = дата транзакции события. Отменённый заказ порождает ДВЕ строки:
//! «заказ» (на дату заказа) и «отмена» (на дату отмены), обе — один регистратор.

use chrono::{DateTime, FixedOffset, Utc};
use uuid::Uuid;

use contracts::domain::a012_wb_sales::aggregate::WbSales;
use contracts::domain::a015_wb_orders::aggregate::WbOrders;
use contracts::domain::a036_wb_sales_funnel_daily::aggregate::WbSalesFunnelDaily;
use contracts::domain::a040_wb_search_analytics_daily::aggregate::WbSearchAnalyticsDaily;
use contracts::projections::p916_mp_sales_funnel_turnovers::dto::FunnelStage;

use super::repository::Model;

pub const REG_A036: &str = "a036_wb_sales_funnel_daily";
pub const REG_A015: &str = "a015_wb_orders";
pub const REG_A012: &str = "a012_wb_sales";
pub const REG_A040: &str = "a040_wb_search_analytics_daily";

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

/// Заготовка строки движения: все метрики нулевые, `show_count = None`.
#[allow(clippy::too_many_arguments)]
fn base_row(
    stage: FunnelStage,
    cohort_date: String,
    event_date: String,
    connection_mp_ref: String,
    marketplace_product_ref: Option<String>,
    nomenclature_ref: Option<String>,
    nm_id: Option<i64>,
    registrator_type: &str,
    registrator_ref: &str,
    now: &str,
) -> Model {
    Model {
        id: Uuid::new_v4().to_string(),
        stage: stage.as_str().to_string(),
        cohort_date,
        event_date,
        connection_mp_ref,
        marketplace_product_ref,
        nomenclature_ref,
        nm_id,
        registrator_type: registrator_type.to_string(),
        registrator_ref: registrator_ref.to_string(),
        show_count: None,
        open_count: 0,
        cart_count: 0,
        wishlist_count: 0,
        funnel_order_count: 0,
        funnel_order_sum: 0.0,
        order_count: 0,
        order_sum: 0.0,
        cancel_count: 0,
        cancel_sum: 0.0,
        buyout_count: 0,
        buyout_sum: 0.0,
        return_count: 0,
        return_sum: 0.0,
        created_at_msk: now.to_string(),
        updated_at_msk: now.to_string(),
    }
}

/// Стадия 1: строки маркетинговой воронки из a036 — по одной на `line.nm_id`.
/// `cohort_date = event_date = header.document_date`. `show_count` пока не заполняется
/// (резерв под показы из будущей поисковой аналитики).
pub fn from_wb_funnel_daily(doc: &WbSalesFunnelDaily, registrator_ref: &str) -> Vec<Model> {
    let now = now_msk();
    let Some(date) = msk_date_str(&doc.header.document_date) else {
        return Vec::new();
    };
    let connection = doc.header.connection_id.clone();

    let mut rows = Vec::new();
    for line in &doc.lines {
        let m = &line.metrics;
        // Разреженность: пропускаем товар без активности верха воронки.
        if m.open_count == 0
            && m.cart_count == 0
            && m.add_to_wishlist_count == 0
            && m.order_count == 0
            && m.order_sum == 0.0
        {
            continue;
        }

        let mut row = base_row(
            FunnelStage::Marketing,
            date.clone(),
            date.clone(),
            connection.clone(),
            None, // marketplace_product_ref в a036 отсутствует; мост по nm_id
            line.nomenclature_ref.clone(),
            Some(line.nm_id),
            REG_A036,
            registrator_ref,
            &now,
        );
        row.open_count = m.open_count;
        row.cart_count = m.cart_count;
        row.wishlist_count = m.add_to_wishlist_count;
        row.funnel_order_count = m.order_count;
        row.funnel_order_sum = m.order_sum;
        rows.push(row);
    }
    rows
}

/// Стадия 1: показы из a040 (поисковая аналитика) — по строке на `line.nm_id`.
/// Заполняется ТОЛЬКО `show_count` (импрешены), остальные метрики верха воронки
/// остаются за a036 (чтобы не задваивать переходы). `cohort_date = event_date = snapshot_date`.
pub fn from_wb_search_analytics(doc: &WbSearchAnalyticsDaily, registrator_ref: &str) -> Vec<Model> {
    let now = now_msk();
    let Some(date) = msk_date_str(&doc.header.snapshot_date) else {
        return Vec::new();
    };
    let connection = doc.header.connection_id.clone();

    let mut rows = Vec::new();
    for line in &doc.lines {
        // Разреженность: пропускаем товар без показов.
        if line.metrics.impressions == 0 {
            continue;
        }
        let mut row = base_row(
            FunnelStage::Marketing,
            date.clone(),
            date.clone(),
            connection.clone(),
            None,
            line.nomenclature_ref.clone(),
            Some(line.nm_id),
            REG_A040,
            registrator_ref,
            &now,
        );
        row.show_count = Some(line.metrics.impressions);
        rows.push(row);
    }
    rows
}

/// Стадия 2: движение заказа из a015. Всегда одна строка «заказ» на дату заказа;
/// при отмене — дополнительная строка «отмена» на дату отмены (когорта = дата заказа).
pub fn from_wb_orders(doc: &WbOrders, registrator_ref: &str) -> Vec<Model> {
    let now = now_msk();
    let connection = doc.header.connection_id.clone();
    let order_date = msk_date_from_utc(&doc.state.order_dt);
    let amount = doc.line.allocation_basis();
    let mp_ref = doc.marketplace_product_ref.clone();
    let nom_ref = doc.nomenclature_ref.clone();
    let nm_id = Some(doc.line.nm_id);

    let mut rows = Vec::new();

    // Строка «заказ»: обе оси = дата заказа.
    let mut order_row = base_row(
        FunnelStage::Fulfillment,
        order_date.clone(),
        order_date.clone(),
        connection.clone(),
        mp_ref.clone(),
        nom_ref.clone(),
        nm_id,
        REG_A015,
        registrator_ref,
        &now,
    );
    order_row.order_count = 1;
    order_row.order_sum = amount;
    rows.push(order_row);

    // Строка «отмена»: когорта = дата заказа, событие = дата отмены (фолбэк — дата заказа).
    if doc.state.is_cancel {
        let cancel_event_date = doc
            .state
            .cancel_dt
            .as_ref()
            .map(msk_date_from_utc)
            .unwrap_or_else(|| order_date.clone());
        let mut cancel_row = base_row(
            FunnelStage::Fulfillment,
            order_date.clone(),
            cancel_event_date,
            connection.clone(),
            mp_ref.clone(),
            nom_ref.clone(),
            nm_id,
            REG_A015,
            registrator_ref,
            &now,
        );
        cancel_row.cancel_count = 1;
        cancel_row.cancel_sum = amount;
        rows.push(cancel_row);
    }

    rows
}

/// Сумма продажи/возврата a012: `amount_line` → `sell_out_fact` → `finished_price * qty`.
fn sales_amount(doc: &WbSales) -> f64 {
    let line = &doc.line;
    if let Some(v) = line.amount_line.filter(|v| *v != 0.0) {
        return v;
    }
    if let Some(v) = line.sell_out_fact.filter(|v| *v != 0.0) {
        return v;
    }
    line.finished_price.unwrap_or(0.0) * line.qty
}

/// Стадия 2: движение выкупа/возврата из a012. Одна строка.
/// `event_date` = дата продажи (`sale_dt`). Отдельной даты заказа у a012 нет →
/// `cohort_date` фолбэком тоже = дата продажи (известное ограничение; корректная
/// когортная привязка выкупа к дате заказа через srid→a015 — будущее улучшение).
pub fn from_wb_sales(doc: &WbSales, registrator_ref: &str) -> Vec<Model> {
    let now = now_msk();
    let connection = doc.header.connection_id.clone();
    let sale_date = msk_date_from_utc(&doc.state.sale_dt);
    let amount = sales_amount(doc);
    let count = doc.line.qty.round() as i64;
    if count == 0 && amount == 0.0 {
        return Vec::new();
    }

    let is_return = doc.is_customer_return || doc.state.event_type.eq_ignore_ascii_case("return");

    let mut row = base_row(
        FunnelStage::Fulfillment,
        sale_date.clone(),
        sale_date,
        connection,
        doc.marketplace_product_ref.clone(),
        doc.nomenclature_ref.clone(),
        Some(doc.line.nm_id),
        REG_A012,
        registrator_ref,
        &now,
    );
    if is_return {
        row.return_count = count;
        row.return_sum = amount;
    } else {
        row.buyout_count = count;
        row.buyout_sum = amount;
    }
    vec![row]
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use contracts::domain::a015_wb_orders::aggregate::{
        WbOrders, WbOrdersGeography, WbOrdersHeader, WbOrdersLine, WbOrdersSourceMeta, WbOrdersState,
        WbOrdersWarehouse,
    };
    use contracts::domain::a036_wb_sales_funnel_daily::aggregate::{
        WbSalesFunnelDailyHeader, WbSalesFunnelDailyLine, WbSalesFunnelDailyMetrics,
        WbSalesFunnelDailySourceMeta,
    };

    fn utc(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    fn wb_order(is_cancel: bool, cancel_dt: Option<&str>) -> WbOrders {
        let line = WbOrdersLine {
            line_id: "srid-1".to_string(),
            supplier_article: "ART-1".to_string(),
            nm_id: 777,
            barcode: "bc".to_string(),
            category: None,
            subject: None,
            brand: None,
            tech_size: None,
            qty: 1.0,
            total_price: Some(1200.0),
            discount_percent: None,
            spp: None,
            finished_price: Some(900.0),
            price_with_disc: Some(1000.0),
            price: None,
            sale_price: None,
            dealer_price_ut: None,
            margin_pro: None,
            currency_code: None,
            fx_rate: None,
        };
        let state = WbOrdersState {
            order_dt: utc("2026-03-01T05:00:00Z"),
            last_change_dt: None,
            is_cancel,
            cancel_dt: cancel_dt.map(utc),
            is_supply: None,
            is_realization: None,
        };
        let header = WbOrdersHeader {
            document_no: "srid-1".to_string(),
            connection_id: "conn-1".to_string(),
            organization_id: "org-1".to_string(),
            marketplace_id: "mp-1".to_string(),
        };
        let source_meta = WbOrdersSourceMeta {
            income_id: None,
            sticker: None,
            g_number: None,
            raw_payload_ref: "raw-1".to_string(),
            marketplace_raw_payload_ref: None,
            fetched_at: utc("2026-03-01T06:00:00Z"),
            document_version: 1,
        };
        let mut doc = WbOrders::new_for_insert(
            "code-1".to_string(),
            "WB заказ".to_string(),
            header,
            line,
            state,
            WbOrdersWarehouse {
                warehouse_name: None,
                warehouse_type: None,
            },
            WbOrdersGeography {
                country_name: None,
                oblast_okrug_name: None,
                region_name: None,
            },
            source_meta,
            true,
            Some("2026-03-01".to_string()),
        );
        doc.nomenclature_ref = Some("nom-1".to_string());
        doc.marketplace_product_ref = Some("mp-prod-1".to_string());
        doc
    }

    #[test]
    fn order_without_cancel_emits_single_order_row() {
        let doc = wb_order(false, None);
        let rows = from_wb_orders(&doc, "reg-1");
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert_eq!(r.stage, "fulfillment");
        assert_eq!(r.order_count, 1);
        assert_eq!(r.order_sum, 1000.0); // price_with_disc через allocation_basis
        assert_eq!(r.cohort_date, "2026-03-01");
        assert_eq!(r.event_date, "2026-03-01");
        assert_eq!(r.cancel_count, 0);
        assert_eq!(r.nm_id, Some(777));
    }

    #[test]
    fn cancelled_order_emits_order_and_cancel_rows_on_split_dates() {
        let doc = wb_order(true, Some("2026-03-05T22:00:00Z"));
        let rows = from_wb_orders(&doc, "reg-1");
        assert_eq!(rows.len(), 2);

        let order = rows.iter().find(|r| r.order_count == 1).unwrap();
        assert_eq!(order.cancel_count, 0);
        assert_eq!(order.cohort_date, "2026-03-01");
        assert_eq!(order.event_date, "2026-03-01");

        let cancel = rows.iter().find(|r| r.cancel_count == 1).unwrap();
        assert_eq!(cancel.order_count, 0);
        // Когорта = дата заказа, событие = дата отмены (MSK +3 → 2026-03-06).
        assert_eq!(cancel.cohort_date, "2026-03-01");
        assert_eq!(cancel.event_date, "2026-03-06");
        assert_eq!(cancel.cancel_sum, 1000.0);
    }

    #[test]
    fn funnel_daily_skips_empty_and_maps_metrics() {
        let header = WbSalesFunnelDailyHeader {
            document_no: "F-1".to_string(),
            document_date: "2026-03-02".to_string(),
            connection_id: "conn-1".to_string(),
            organization_id: "org-1".to_string(),
            marketplace_id: "mp-1".to_string(),
            currency: "RUB".to_string(),
        };
        let active_line = WbSalesFunnelDailyLine {
            nm_id: 101,
            title: "T".to_string(),
            vendor_code: "V".to_string(),
            brand_name: "B".to_string(),
            subject_id: 1,
            subject_name: "S".to_string(),
            nomenclature_ref: Some("nom-101".to_string()),
            metrics: WbSalesFunnelDailyMetrics {
                open_count: 10,
                cart_count: 4,
                order_count: 2,
                order_sum: 3000.0,
                add_to_wishlist_count: 1,
                ..Default::default()
            },
        };
        let empty_line = WbSalesFunnelDailyLine {
            nm_id: 202,
            title: "T2".to_string(),
            vendor_code: "V2".to_string(),
            brand_name: "B2".to_string(),
            subject_id: 2,
            subject_name: "S2".to_string(),
            nomenclature_ref: None,
            metrics: WbSalesFunnelDailyMetrics::default(),
        };
        let doc = contracts::domain::a036_wb_sales_funnel_daily::aggregate::WbSalesFunnelDaily::new_for_insert(
            header,
            WbSalesFunnelDailyMetrics::default(),
            vec![active_line, empty_line],
            WbSalesFunnelDailySourceMeta {
                source: "wb".to_string(),
                fetched_at: "2026-03-02T00:00:00Z".to_string(),
            },
        );

        let rows = from_wb_funnel_daily(&doc, "reg-f-1");
        assert_eq!(rows.len(), 1); // пустая строка отброшена
        let r = &rows[0];
        assert_eq!(r.stage, "marketing");
        assert_eq!(r.nm_id, Some(101));
        assert_eq!(r.open_count, 10);
        assert_eq!(r.cart_count, 4);
        assert_eq!(r.wishlist_count, 1);
        assert_eq!(r.funnel_order_count, 2);
        assert_eq!(r.funnel_order_sum, 3000.0);
        assert_eq!(r.cohort_date, "2026-03-02");
        assert_eq!(r.event_date, "2026-03-02");
        assert!(r.show_count.is_none());
    }
}
