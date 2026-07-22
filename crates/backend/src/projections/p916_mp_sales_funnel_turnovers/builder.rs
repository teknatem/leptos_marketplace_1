//! Построение строк p916 (движений воронки) из регистраторов-источников.
//!
//! Каждый источник маппит свои данные в `Vec<Model>`. `id` строки — детерминированный
//! `uuid v5` от натурального ключа (см. `deterministic_id`), поэтому повторный прогон даёт
//! те же `id` и вставка через `on_conflict(id)` перезаписывает, а не задваивает. Вызывающая
//! сторона дополнительно делает delete-by-registrator/period (убрать исчезнувшие строки).
//! Пустые строки не пишутся (разреженность — контроль размера проекции).
//!
//! Стадия 1 (marketing) — из a036: cohort_date = event_date = день воронки.
//! Стадия 2 (fulfillment) — из a015/a012: cohort_date = дата заказа (когорта),
//! event_date = дата транзакции события. Отменённый заказ порождает ДВЕ строки:
//! «заказ» (на дату заказа) и «отмена» (на дату отмены), обе — один регистратор.

use chrono::{DateTime, FixedOffset, Utc};
use uuid::Uuid;

use contracts::domain::a012_wb_sales::aggregate::WbSales;
use contracts::domain::a015_wb_orders::aggregate::WbOrders;
use contracts::domain::a026_wb_advert_daily::aggregate::WbAdvertDaily;
use contracts::domain::a036_wb_sales_funnel_daily::aggregate::WbSalesFunnelDaily;
use contracts::domain::a040_wb_search_analytics_daily::aggregate::WbSearchAnalyticsDaily;
use contracts::projections::p916_mp_sales_funnel_turnovers::dto::FunnelStage;

use super::repository::Model;

pub const REG_A036: &str = "a036_wb_sales_funnel_daily";
pub const REG_A015: &str = "a015_wb_orders";
pub const REG_A012: &str = "a012_wb_sales";
pub const REG_A040: &str = "a040_wb_search_analytics_daily";
pub const REG_A026: &str = "a026_wb_advert_daily";

/// Namespace для детерминированного `id` строки движения (uuid v5).
/// Фиксированное значение — менять нельзя, иначе `id` перестанут совпадать между прогонами.
const P916_ID_NAMESPACE: Uuid = Uuid::from_u128(0x8f2e_6b41_9a3d_4c17_b0e5_1d2f3a4b5c6d);

/// Детерминированный `id` строки движения из натурального ключа. `kind` — дискриминатор
/// движения ("order"/"cancel"/"buyout"/"return"/"marketing"), чтобы «заказ» и «отмена»
/// одного srid в один день не схлопнулись в один `id`. Одинаковый вход → одинаковый `id`
/// → повторная вставка перезаписывает строку (`on_conflict(id)`), а не задваивает обороты.
#[allow(clippy::too_many_arguments)]
fn deterministic_id(
    registrator_type: &str,
    registrator_ref: &str,
    stage: FunnelStage,
    kind: &str,
    cohort_date: &str,
    event_date: &str,
    connection_mp_ref: &str,
    nm_id: Option<i64>,
    marketplace_product_ref: Option<&str>,
) -> String {
    // Товарный ключ: nm_id (WB) или marketplace_product_ref (YM/OZON), иначе пусто.
    let product_key = nm_id
        .map(|v| v.to_string())
        .or_else(|| marketplace_product_ref.map(|s| s.to_string()))
        .unwrap_or_default();
    let key = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}",
        registrator_type,
        registrator_ref,
        stage.as_str(),
        kind,
        cohort_date,
        event_date,
        connection_mp_ref,
        product_key
    );
    Uuid::new_v5(&P916_ID_NAMESPACE, key.as_bytes()).to_string()
}

/// Текущее время в MSK (+03:00) в формате RFC3339.
pub fn now_msk() -> String {
    let msk = FixedOffset::east_opt(3 * 3600).expect("valid MSK offset");
    Utc::now().with_timezone(&msk).to_rfc3339()
}

/// UTC-момент → MSK-дата `YYYY-MM-DD`. Публично — переиспользуется хуком a012 для
/// форматирования резолвнутой даты заказа (когорта выкупа/возврата).
pub fn msk_date_from_utc(dt: &DateTime<Utc>) -> String {
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

/// Заготовка строки движения: все метрики нулевые, показы (`show_*_count`) = None.
/// `kind` — дискриминатор движения для детерминированного `id` (см. `deterministic_id`).
#[allow(clippy::too_many_arguments)]
fn base_row(
    stage: FunnelStage,
    kind: &str,
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
    let id = deterministic_id(
        registrator_type,
        registrator_ref,
        stage,
        kind,
        &cohort_date,
        &event_date,
        &connection_mp_ref,
        nm_id,
        marketplace_product_ref.as_deref(),
    );
    Model {
        id,
        stage: stage.as_str().to_string(),
        cohort_date,
        event_date,
        connection_mp_ref,
        marketplace_product_ref,
        nomenclature_ref,
        nm_id,
        registrator_type: registrator_type.to_string(),
        registrator_ref: registrator_ref.to_string(),
        show_free_count: None,
        show_paid_count: None,
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
/// `cohort_date = event_date = header.document_date`. Показы (`show_free_count`/`show_paid_count`)
/// a036 не заполняет — они за a040 (органика) и a026 (реклама).
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
            "marketing",
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

/// Стадия 1: бесплатные/органические показы из a040 (поисковая аналитика) — по строке
/// на `line.nm_id`. Заполняется ТОЛЬКО `show_free_count` (органические импрешены);
/// переходы/корзина остаются за a036, платные показы — за a026 (чтобы не задваивать).
/// `cohort_date = event_date = snapshot_date`.
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
            "marketing",
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
        row.show_free_count = Some(line.metrics.impressions);
        rows.push(row);
    }
    rows
}

/// Стадия 1: платные показы из a026 (рекламный суточный отчёт) — по строке на `line.nm_id`.
/// Заполняется ТОЛЬКО `show_paid_count` (рекламные views); органика остаётся за a040,
/// переходы/корзина — за a036. Один документ a026 = один advert_id × дата, строки — nm_id;
/// на чтении SUM складывает views по всем кампаниям дня. `cohort_date = event_date = document_date`.
pub fn from_wb_advert_daily(doc: &WbAdvertDaily, registrator_ref: &str) -> Vec<Model> {
    let now = now_msk();
    let Some(date) = msk_date_str(&doc.header.document_date) else {
        return Vec::new();
    };
    let connection = doc.header.connection_id.clone();

    let mut rows = Vec::new();
    for line in &doc.lines {
        // Разреженность: пропускаем товар без рекламных показов.
        if line.metrics.views == 0 {
            continue;
        }
        let mut row = base_row(
            FunnelStage::Marketing,
            "marketing",
            date.clone(),
            date.clone(),
            connection.clone(),
            None,
            line.nomenclature_ref.clone(),
            Some(line.nm_id),
            REG_A026,
            registrator_ref,
            &now,
        );
        row.show_paid_count = Some(line.metrics.views);
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
        "order",
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
            "cancel",
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
/// `event_date` = дата продажи (`sale_dt`). `cohort_date` = `order_cohort_date`, если известна
/// (дата заказа из a015 по srid — резолвится вызывающей стороной, где доступна БД), иначе
/// фолбэком = дата продажи (для срезов, где заказ не найден).
pub fn from_wb_sales(
    doc: &WbSales,
    registrator_ref: &str,
    order_cohort_date: Option<String>,
) -> Vec<Model> {
    let now = now_msk();
    let connection = doc.header.connection_id.clone();
    let sale_date = msk_date_from_utc(&doc.state.sale_dt);
    let cohort_date = order_cohort_date.unwrap_or_else(|| sale_date.clone());
    let amount = sales_amount(doc);
    let count = doc.line.qty.round() as i64;
    if count == 0 && amount == 0.0 {
        return Vec::new();
    }

    let is_return = doc.is_customer_return || doc.state.event_type.eq_ignore_ascii_case("return");
    let kind = if is_return { "return" } else { "buyout" };

    let mut row = base_row(
        FunnelStage::Fulfillment,
        kind,
        cohort_date,
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
    use contracts::domain::a012_wb_sales::aggregate::{
        WbSales, WbSalesHeader, WbSalesLine, WbSalesSourceMeta, WbSalesState, WbSalesWarehouse,
    };
    use contracts::domain::a015_wb_orders::aggregate::{
        WbOrders, WbOrdersGeography, WbOrdersHeader, WbOrdersLine, WbOrdersSourceMeta, WbOrdersState,
        WbOrdersWarehouse,
    };
    use contracts::domain::a026_wb_advert_daily::aggregate::{
        WbAdvertDailyHeader, WbAdvertDailyLine, WbAdvertDailyMetrics, WbAdvertDailySourceMeta,
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
        assert!(r.show_free_count.is_none());
        assert!(r.show_paid_count.is_none());
    }

    #[test]
    fn advert_daily_maps_only_paid_shows_and_skips_zero_views() {
        let header = WbAdvertDailyHeader {
            document_no: "AD-1".to_string(),
            document_date: "2026-03-03".to_string(),
            advert_id: 555,
            connection_id: "conn-1".to_string(),
            organization_id: "org-1".to_string(),
            marketplace_id: "mp-1".to_string(),
        };
        let paid_line = WbAdvertDailyLine {
            nm_id: 303,
            nm_name: "N".to_string(),
            nomenclature_ref: Some("nom-303".to_string()),
            advert_ids: vec![555],
            app_types: vec![],
            placements: vec![],
            metrics: WbAdvertDailyMetrics {
                views: 1200,
                clicks: 30,
                ..Default::default()
            },
        };
        let zero_line = WbAdvertDailyLine {
            nm_id: 404,
            nm_name: "Z".to_string(),
            nomenclature_ref: None,
            advert_ids: vec![555],
            app_types: vec![],
            placements: vec![],
            metrics: WbAdvertDailyMetrics::default(),
        };
        let doc = WbAdvertDaily::new_for_insert(
            header,
            WbAdvertDailyMetrics::default(),
            WbAdvertDailyMetrics::default(),
            vec![paid_line, zero_line],
            WbAdvertDailySourceMeta {
                source: "wb_advert_stats".to_string(),
                fetched_at: "2026-03-03T00:00:00Z".to_string(),
            },
        );

        let rows = from_wb_advert_daily(&doc, "reg-ad-1");
        assert_eq!(rows.len(), 1); // строка без показов отброшена
        let r = &rows[0];
        assert_eq!(r.stage, "marketing");
        assert_eq!(r.registrator_type, REG_A026);
        assert_eq!(r.nm_id, Some(303));
        assert_eq!(r.show_paid_count, Some(1200));
        assert!(r.show_free_count.is_none()); // только платные показы
        assert_eq!(r.open_count, 0); // переходы/корзина — не из a026
        assert_eq!(r.cohort_date, "2026-03-03");
        assert_eq!(r.event_date, "2026-03-03");
    }

    fn wb_sale(is_return: bool, sale_dt: &str) -> WbSales {
        let line = WbSalesLine {
            line_id: "srid-9".to_string(),
            supplier_article: "ART-9".to_string(),
            nm_id: 888,
            barcode: "bc9".to_string(),
            name: "Товар 9".to_string(),
            qty: 1.0,
            price_list: None,
            discount_total: None,
            price_effective: None,
            amount_line: Some(500.0),
            currency_code: None,
            total_price: None,
            payment_sale_amount: None,
            discount_percent: None,
            spp: None,
            finished_price: Some(500.0),
            is_fact: None,
            sell_out_plan: None,
            sell_out_fact: None,
            acquiring_fee_plan: None,
            acquiring_fee_fact: None,
            other_fee_plan: None,
            other_fee_fact: None,
            supplier_payout_plan: None,
            supplier_payout_fact: None,
            profit_plan: None,
            profit_fact: None,
            cost_of_production: None,
            commission_plan: None,
            commission_fact: None,
            dealer_price_ut: None,
        };
        let state = WbSalesState {
            event_type: if is_return { "return" } else { "sale" }.to_string(),
            status_norm: "DELIVERED".to_string(),
            sale_dt: utc(sale_dt),
            last_change_dt: None,
            is_supply: None,
            is_realization: None,
        };
        let header = WbSalesHeader {
            document_no: "srid-9".to_string(),
            sale_id: Some("S9".to_string()),
            connection_id: "conn-1".to_string(),
            organization_id: "org-1".to_string(),
            marketplace_id: "mp-1".to_string(),
        };
        let source_meta = WbSalesSourceMeta {
            raw_payload_ref: "raw-9".to_string(),
            fetched_at: utc("2026-03-10T06:00:00Z"),
            document_version: 1,
        };
        let mut doc = WbSales::new_for_insert(
            "code-9".to_string(),
            "WB продажа".to_string(),
            header,
            line,
            state,
            WbSalesWarehouse {
                warehouse_name: None,
                warehouse_type: None,
            },
            source_meta,
            true,
        );
        doc.is_customer_return = is_return;
        doc.nomenclature_ref = Some("nom-9".to_string());
        doc.marketplace_product_ref = Some("mp-prod-9".to_string());
        doc
    }

    #[test]
    fn sales_uses_order_cohort_date_when_provided() {
        let doc = wb_sale(false, "2026-03-10T08:00:00Z");
        let rows = from_wb_sales(&doc, "reg-s-1", Some("2026-03-01".to_string()));
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert_eq!(r.buyout_count, 1);
        // Когорта = дата заказа (передана извне), событие = дата продажи (MSK +3).
        assert_eq!(r.cohort_date, "2026-03-01");
        assert_eq!(r.event_date, "2026-03-10");
    }

    #[test]
    fn sales_falls_back_to_sale_date_without_order() {
        let doc = wb_sale(true, "2026-03-10T08:00:00Z");
        let rows = from_wb_sales(&doc, "reg-s-1", None);
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert_eq!(r.return_count, 1);
        // Заказ не найден → когорта фолбэком = дата продажи.
        assert_eq!(r.cohort_date, "2026-03-10");
        assert_eq!(r.event_date, "2026-03-10");
    }

    #[test]
    fn builder_ids_are_deterministic_across_runs() {
        let doc = wb_order(true, Some("2026-03-05T22:00:00Z"));
        let first = from_wb_orders(&doc, "reg-1");
        let second = from_wb_orders(&doc, "reg-1");
        let ids1: Vec<&str> = first.iter().map(|r| r.id.as_str()).collect();
        let ids2: Vec<&str> = second.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids1, ids2); // одинаковый вход → одинаковый id (идемпотентность upsert)
    }

    #[test]
    fn order_and_cancel_rows_have_distinct_ids_even_same_day() {
        // Заказ и отмена в один день: обе оси совпадают, различает только kind в id.
        let doc = wb_order(true, Some("2026-03-01T20:00:00Z"));
        let rows = from_wb_orders(&doc, "reg-1");
        let order = rows.iter().find(|r| r.order_count == 1).unwrap();
        let cancel = rows.iter().find(|r| r.cancel_count == 1).unwrap();
        assert_eq!(order.cohort_date, cancel.cohort_date);
        assert_eq!(order.event_date, cancel.event_date); // один день
        assert_ne!(order.id, cancel.id); // но id разные — нет схлопывания
    }
}
