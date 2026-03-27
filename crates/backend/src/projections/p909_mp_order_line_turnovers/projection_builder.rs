use anyhow::Result;
use chrono::Utc;
use contracts::domain::a012_wb_sales::aggregate::WbSales;
use contracts::domain::a015_wb_orders::aggregate::WbOrders;
use contracts::shared::analytics::{EventKind, TurnoverLayer};
use uuid::Uuid;

use super::repository::Model;
use crate::projections::general_ledger::repository::Model as GeneralLedgerModel;
use crate::shared::analytics::normalization::opt_nonzero;
use crate::shared::analytics::turnover_registry::get_turnover_class;

const ORDER_REGISTRATOR_TYPE: &str = "a015_wb_orders";
const OPER_REGISTRATOR_TYPE: &str = "a012_wb_sales";
const FACT_REGISTRATOR_TYPE: &str = "p903_wb_finance_report";

pub struct PostingResult {
    pub turnovers: Vec<Model>,
    pub general_ledger_entries: Vec<GeneralLedgerModel>,
}

fn now_str() -> String {
    Utc::now().to_rfc3339()
}

fn business_date_from_datetime(value: chrono::DateTime<Utc>) -> String {
    value.format("%Y-%m-%d").to_string()
}

fn normalize_business_date_str(value: &str) -> String {
    if value.len() >= 10 {
        value[..10].to_string()
    } else {
        value.to_string()
    }
}

fn classifier_kinds(turnover_code: &str) -> (String, String) {
    let class = get_turnover_class(turnover_code).unwrap_or_else(|| {
        panic!("Missing turnover class for code '{}'", turnover_code);
    });

    (
        class.value_kind.as_str().to_string(),
        class.agg_kind.as_str().to_string(),
    )
}

#[allow(clippy::too_many_arguments)]
fn new_row(
    connection_mp_ref: &str,
    order_key: &str,
    line_key: &str,
    line_event_key: &str,
    event_kind: EventKind,
    entry_date: &str,
    layer: TurnoverLayer,
    turnover_code: &str,
    amount: Option<f64>,
    nomenclature_ref: Option<String>,
    marketplace_product_ref: Option<String>,
    registrator_type: &str,
    registrator_ref: String,
) -> Option<Model> {
    let amount = opt_nonzero(amount)?;
    let now = now_str();
    let (value_kind, agg_kind) = classifier_kinds(turnover_code);

    Some(Model {
        id: format!(
            "{connection_mp_ref}:{line_event_key}:{turnover_code}:{}",
            layer.as_str()
        ),
        connection_mp_ref: connection_mp_ref.to_string(),
        order_key: order_key.to_string(),
        line_key: line_key.to_string(),
        line_event_key: line_event_key.to_string(),
        event_kind: event_kind.as_str().to_string(),
        entry_date: entry_date.to_string(),
        layer: layer.as_str().to_string(),
        turnover_code: turnover_code.to_string(),
        value_kind,
        agg_kind,
        amount,
        nomenclature_ref,
        marketplace_product_ref,
        registrator_type: registrator_type.to_string(),
        registrator_ref,
        link_status: "single".to_string(),
        general_ledger_ref: None,
        created_at: now.clone(),
        updated_at: now,
    })
}

fn make_general_ledger_entry(row: &Model, posting_id: &str) -> Option<GeneralLedgerModel> {
    let class = get_turnover_class(&row.turnover_code)?;
    if !class.generates_journal_entry || row.amount.abs() <= f64::EPSILON {
        return None;
    }

    Some(GeneralLedgerModel {
        id: Uuid::new_v4().to_string(),
        posting_id: posting_id.to_string(),
        entry_date: row.entry_date.clone(),
        layer: row.layer.clone(),
        registrator_type: row.registrator_type.clone(),
        registrator_ref: row.registrator_ref.clone(),
        debit_account: class.debit_account.to_string(),
        credit_account: class.credit_account.to_string(),
        amount: row.amount,
        qty: None,
        turnover_code: row.turnover_code.clone(),
        detail_kind: "p909_mp_order_line_turnovers".to_string(),
        detail_id: row.id.clone(),
        resource_name: row.turnover_code.clone(),
        resource_sign: 1,
        created_at: now_str(),
    })
}

fn push_row_with_journal(
    turnovers: &mut Vec<Model>,
    general_ledger_entries: &mut Vec<GeneralLedgerModel>,
    row: Option<Model>,
    posting_id: &str,
) {
    let Some(mut row) = row else {
        return;
    };

    if let Some(je) = make_general_ledger_entry(&row, posting_id) {
        row.general_ledger_ref = Some(je.id.clone());
        general_ledger_entries.push(je);
    }

    turnovers.push(row);
}

fn push_row(target: &mut Vec<Model>, row: Option<Model>) {
    if let Some(row) = row {
        target.push(row);
    }
}

pub fn order_source_ref(document_id: &str) -> String {
    format!("a015:{document_id}")
}

pub fn oper_source_ref(document_id: &str) -> String {
    format!("a012:{document_id}")
}

fn finance_source_ref(rr_dt: &str, rrd_id: i64) -> String {
    format!("p903:{rr_dt}:{rrd_id}")
}

pub fn finance_source_ref_from_model(
    entry: &crate::projections::p903_wb_finance_report::repository::Model,
) -> String {
    finance_source_ref(&entry.rr_dt, entry.rrd_id)
}

pub fn is_finance_row_linked(
    entry: &crate::projections::p903_wb_finance_report::repository::Model,
) -> bool {
    entry
        .srid
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
}

pub fn from_wb_order(document: &WbOrders, document_id: &str) -> Result<Vec<Model>> {
    let registrator_ref = order_source_ref(document_id);
    let line_key = document.line.line_id.clone();
    let order_key = document.header.document_no.clone();
    let line_event_key = format!("ordered:{line_key}");
    let entry_date = business_date_from_datetime(document.state.order_dt);

    let mut rows = Vec::new();
    push_row(
        &mut rows,
        new_row(
            &document.header.connection_id,
            &order_key,
            &line_key,
            &line_event_key,
            EventKind::Ordered,
            &entry_date,
            TurnoverLayer::Oper,
            "qty_ordered",
            Some(document.line.qty),
            document.nomenclature_ref.clone(),
            document.marketplace_product_ref.clone(),
            ORDER_REGISTRATOR_TYPE,
            registrator_ref,
        ),
    );

    Ok(rows)
}

pub fn from_wb_sales(
    document: &WbSales,
    document_id: &str,
    posting_id: &str,
) -> Result<PostingResult> {
    let registrator_ref = oper_source_ref(document_id);
    let line_key = document.line.line_id.clone();
    let order_key = document.header.document_no.clone();
    let is_return = document.is_customer_return;
    let event_kind = if is_return {
        EventKind::Returned
    } else {
        EventKind::Sold
    };
    let line_event_key = format!("{}:{line_key}", event_kind.as_str());
    let entry_date = business_date_from_datetime(document.state.sale_dt);
    let sign = if is_return { -1.0 } else { 1.0 };

    let qty_abs = document.line.qty.abs();
    let revenue_base = document
        .line
        .price_effective
        .or(document.line.amount_line)
        .map(|v| v.abs());

    let spp_rate = document.line.spp.unwrap_or(0.0);
    let spp_amount = if spp_rate > f64::EPSILON {
        revenue_base.map(|b| -(b * spp_rate / 100.0) * sign)
    } else {
        None
    };

    let abs_price_diff = match (document.line.finished_price, document.line.amount_line) {
        (Some(fin), Some(base)) => Some(fin.abs() - base.abs()),
        _ => None,
    };

    let mut turnovers = Vec::new();
    let mut general_ledger_entries = Vec::new();

    let mut push = |turnover_code: &str, amount: Option<f64>| {
        push_row_with_journal(
            &mut turnovers,
            &mut general_ledger_entries,
            new_row(
                &document.header.connection_id,
                &order_key,
                &line_key,
                &line_event_key,
                event_kind,
                &entry_date,
                TurnoverLayer::Oper,
                turnover_code,
                amount,
                document.nomenclature_ref.clone(),
                document.marketplace_product_ref.clone(),
                OPER_REGISTRATOR_TYPE,
                registrator_ref.clone(),
            ),
            posting_id,
        );
    };

    push("qty_sold", Some(qty_abs * sign));
    push("customer_revenue", revenue_base.map(|v| v * sign));
    if let Some(spp) = spp_amount {
        push("spp_discount", Some(spp));
    }
    match abs_price_diff {
        Some(d) if d > f64::EPSILON => push("mp_commission", Some(d * sign)),
        Some(d) if d < -f64::EPSILON => push("wb_coinvestment", Some(-d * sign)),
        _ => {}
    }
    drop(push);

    push_common_oper_rows(
        &mut turnovers,
        &mut general_ledger_entries,
        document,
        &registrator_ref,
        posting_id,
        &order_key,
        &line_key,
        &line_event_key,
        event_kind,
        &entry_date,
        sign,
    );

    Ok(PostingResult {
        turnovers,
        general_ledger_entries,
    })
}

#[allow(clippy::too_many_arguments)]
fn push_common_oper_rows(
    turnovers: &mut Vec<Model>,
    general_ledger_entries: &mut Vec<GeneralLedgerModel>,
    document: &WbSales,
    registrator_ref: &str,
    posting_id: &str,
    order_key: &str,
    line_key: &str,
    line_event_key: &str,
    event_kind: EventKind,
    entry_date: &str,
    sign: f64,
) {
    for (turnover_code, amount) in [
        ("mp_acquiring", document.line.acquiring_fee_plan),
        ("seller_payout", document.line.supplier_payout_plan),
        (
            "item_cost",
            document
                .line
                .dealer_price_ut
                .or(document.line.cost_of_production)
                .map(|price| price * document.line.qty.abs() * sign),
        ),
    ] {
        push_row_with_journal(
            turnovers,
            general_ledger_entries,
            new_row(
                &document.header.connection_id,
                order_key,
                line_key,
                line_event_key,
                event_kind,
                entry_date,
                TurnoverLayer::Oper,
                turnover_code,
                amount,
                document.nomenclature_ref.clone(),
                document.marketplace_product_ref.clone(),
                OPER_REGISTRATOR_TYPE,
                registrator_ref.to_string(),
            ),
            posting_id,
        );
    }
}

pub fn from_wb_finance_row(
    entry: &crate::projections::p903_wb_finance_report::repository::Model,
    posting_id: &str,
) -> Result<PostingResult> {
    let Some(line_key) = entry.srid.clone() else {
        return Ok(PostingResult {
            turnovers: vec![],
            general_ledger_entries: vec![],
        });
    };

    let registrator_ref = finance_source_ref_from_model(entry);
    let is_return = entry.supplier_oper_name.as_deref() == Some("Возврат")
        || entry.return_amount.unwrap_or(0.0).abs() > f64::EPSILON;
    let is_sale = entry.supplier_oper_name.as_deref() == Some("Продажа")
        || entry.retail_amount.unwrap_or(0.0).abs() > f64::EPSILON;
    let event_kind = if is_return {
        EventKind::Returned
    } else if is_sale {
        EventKind::Sold
    } else if entry.penalty.unwrap_or(0.0).abs() > f64::EPSILON
        || entry.storage_fee.unwrap_or(0.0).abs() > f64::EPSILON
        || entry.rebill_logistic_cost.unwrap_or(0.0).abs() > f64::EPSILON
    {
        EventKind::Fee
    } else {
        EventKind::Other
    };

    let line_event_key = match event_kind {
        EventKind::Sold => format!("sold:{line_key}"),
        EventKind::Returned => format!("returned:{line_key}"),
        _ => format!("fee:{line_key}:{}:{}", entry.rr_dt, entry.rrd_id),
    };
    let entry_date = normalize_business_date_str(&entry.rr_dt);

    let mut turnovers = Vec::new();
    let mut general_ledger_entries = Vec::new();

    if event_kind == EventKind::Sold {
        push_row_with_journal(
            &mut turnovers,
            &mut general_ledger_entries,
            new_row(
                &entry.connection_mp_ref,
                &line_key,
                &line_key,
                &line_event_key,
                event_kind,
                &entry_date,
                TurnoverLayer::Fact,
                "qty_sold",
                entry.quantity.map(|value| value as f64),
                None,
                None,
                FACT_REGISTRATOR_TYPE,
                registrator_ref.clone(),
            ),
            posting_id,
        );
        push_row_with_journal(
            &mut turnovers,
            &mut general_ledger_entries,
            new_row(
                &entry.connection_mp_ref,
                &line_key,
                &line_key,
                &line_event_key,
                event_kind,
                &entry_date,
                TurnoverLayer::Fact,
                "customer_revenue",
                entry.retail_amount,
                None,
                None,
                FACT_REGISTRATOR_TYPE,
                registrator_ref.clone(),
            ),
            posting_id,
        );
    }

    if event_kind == EventKind::Returned {
        let return_amount = entry.return_amount.or(entry.retail_amount);
        push_row_with_journal(
            &mut turnovers,
            &mut general_ledger_entries,
            new_row(
                &entry.connection_mp_ref,
                &line_key,
                &line_key,
                &line_event_key,
                event_kind,
                &entry_date,
                TurnoverLayer::Fact,
                "qty_returned",
                entry.quantity.map(|value| value as f64),
                None,
                None,
                FACT_REGISTRATOR_TYPE,
                registrator_ref.clone(),
            ),
            posting_id,
        );
        push_row_with_journal(
            &mut turnovers,
            &mut general_ledger_entries,
            new_row(
                &entry.connection_mp_ref,
                &line_key,
                &line_key,
                &line_event_key,
                event_kind,
                &entry_date,
                TurnoverLayer::Fact,
                "customer_return",
                return_amount,
                None,
                None,
                FACT_REGISTRATOR_TYPE,
                registrator_ref.clone(),
            ),
            posting_id,
        );
    }

    push_common_fact_rows(
        &mut turnovers,
        &mut general_ledger_entries,
        entry,
        &registrator_ref,
        posting_id,
        &line_key,
        &line_event_key,
        event_kind,
        &entry_date,
    );

    Ok(PostingResult {
        turnovers,
        general_ledger_entries,
    })
}

#[allow(clippy::too_many_arguments)]
fn push_common_fact_rows(
    rows: &mut Vec<Model>,
    general_ledger_entries: &mut Vec<GeneralLedgerModel>,
    entry: &crate::projections::p903_wb_finance_report::repository::Model,
    registrator_ref: &str,
    posting_id: &str,
    line_key: &str,
    line_event_key: &str,
    event_kind: EventKind,
    entry_date: &str,
) {
    for (turnover_code, amount) in [
        (
            "mp_commission",
            Some(entry.ppvz_vw.unwrap_or(0.0) + entry.ppvz_vw_nds.unwrap_or(0.0)),
        ),
        ("mp_acquiring", entry.acquiring_fee),
        ("mp_logistics", entry.rebill_logistic_cost),
        ("mp_storage", entry.storage_fee),
        ("mp_penalty", entry.penalty),
        ("seller_payout", entry.ppvz_for_pay),
        ("commission_percent", entry.commission_percent),
    ] {
        push_row_with_journal(
            rows,
            general_ledger_entries,
            new_row(
                &entry.connection_mp_ref,
                line_key,
                line_key,
                line_event_key,
                event_kind,
                entry_date,
                TurnoverLayer::Fact,
                turnover_code,
                amount,
                None,
                None,
                FACT_REGISTRATOR_TYPE,
                registrator_ref.to_string(),
            ),
            posting_id,
        );
    }
}

pub fn attach_order_context(existing: &mut Model, document: &WbOrders, _document_id: &str) {
    if existing.nomenclature_ref.is_none() {
        existing.nomenclature_ref = document.nomenclature_ref.clone();
    }
    if existing.marketplace_product_ref.is_none() {
        existing.marketplace_product_ref = document.marketplace_product_ref.clone();
    }
    if existing.order_key.is_empty() {
        existing.order_key = document.header.document_no.clone();
    }
    existing.updated_at = now_str();
}
