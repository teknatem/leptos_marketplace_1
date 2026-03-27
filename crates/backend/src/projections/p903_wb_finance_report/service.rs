use anyhow::Result;
use chrono::NaiveDate;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set, TransactionTrait};
use serde::Serialize;

use crate::shared::data::db::get_connection;

const DETAIL_KIND: &str = "p903_wb_finance_report";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconcileResult {
    pub changed: bool,
    pub source_rows: usize,
    pub general_ledger_rows: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct SnapshotRow {
    rr_dt: String,
    rrd_id: i64,
    connection_mp_ref: String,
    organization_ref: String,
    acquiring_fee: Option<f64>,
    acquiring_percent: Option<f64>,
    additional_payment: Option<f64>,
    bonus_type_name: Option<String>,
    commission_percent: Option<f64>,
    delivery_amount: Option<f64>,
    delivery_rub: Option<f64>,
    nm_id: Option<i64>,
    penalty: Option<f64>,
    ppvz_vw: Option<f64>,
    ppvz_vw_nds: Option<f64>,
    ppvz_sales_commission: Option<f64>,
    quantity: Option<i32>,
    rebill_logistic_cost: Option<f64>,
    retail_amount: Option<f64>,
    retail_price: Option<f64>,
    retail_price_withdisc_rub: Option<f64>,
    return_amount: Option<f64>,
    sa_name: Option<String>,
    storage_fee: Option<f64>,
    subject_name: Option<String>,
    supplier_oper_name: Option<String>,
    cashback_amount: Option<f64>,
    ppvz_for_pay: Option<f64>,
    ppvz_kvw_prc: Option<f64>,
    ppvz_kvw_prc_base: Option<f64>,
    srv_dbs: Option<i32>,
    srid: Option<String>,
}

fn snapshot_from_model(
    row: &crate::projections::p903_wb_finance_report::repository::Model,
) -> SnapshotRow {
    SnapshotRow {
        rr_dt: row.rr_dt.clone(),
        rrd_id: row.rrd_id,
        connection_mp_ref: row.connection_mp_ref.clone(),
        organization_ref: row.organization_ref.clone(),
        acquiring_fee: row.acquiring_fee,
        acquiring_percent: row.acquiring_percent,
        additional_payment: row.additional_payment,
        bonus_type_name: row.bonus_type_name.clone(),
        commission_percent: row.commission_percent,
        delivery_amount: row.delivery_amount,
        delivery_rub: row.delivery_rub,
        nm_id: row.nm_id,
        penalty: row.penalty,
        ppvz_vw: row.ppvz_vw,
        ppvz_vw_nds: row.ppvz_vw_nds,
        ppvz_sales_commission: row.ppvz_sales_commission,
        quantity: row.quantity,
        rebill_logistic_cost: row.rebill_logistic_cost,
        retail_amount: row.retail_amount,
        retail_price: row.retail_price,
        retail_price_withdisc_rub: row.retail_price_withdisc_rub,
        return_amount: row.return_amount,
        sa_name: row.sa_name.clone(),
        storage_fee: row.storage_fee,
        subject_name: row.subject_name.clone(),
        supplier_oper_name: row.supplier_oper_name.clone(),
        cashback_amount: row.cashback_amount,
        ppvz_for_pay: row.ppvz_for_pay,
        ppvz_kvw_prc: row.ppvz_kvw_prc,
        ppvz_kvw_prc_base: row.ppvz_kvw_prc_base,
        srv_dbs: row.srv_dbs,
        srid: row.srid.clone(),
    }
}

fn snapshot_from_entry(
    row: &crate::projections::p903_wb_finance_report::repository::WbFinanceReportEntry,
) -> SnapshotRow {
    SnapshotRow {
        rr_dt: row.rr_dt.format("%Y-%m-%d").to_string(),
        rrd_id: row.rrd_id,
        connection_mp_ref: row.connection_mp_ref.clone(),
        organization_ref: row.organization_ref.clone(),
        acquiring_fee: row.acquiring_fee,
        acquiring_percent: row.acquiring_percent,
        additional_payment: row.additional_payment,
        bonus_type_name: row.bonus_type_name.clone(),
        commission_percent: row.commission_percent,
        delivery_amount: row.delivery_amount,
        delivery_rub: row.delivery_rub,
        nm_id: row.nm_id,
        penalty: row.penalty,
        ppvz_vw: row.ppvz_vw,
        ppvz_vw_nds: row.ppvz_vw_nds,
        ppvz_sales_commission: row.ppvz_sales_commission,
        quantity: row.quantity,
        rebill_logistic_cost: row.rebill_logistic_cost,
        retail_amount: row.retail_amount,
        retail_price: row.retail_price,
        retail_price_withdisc_rub: row.retail_price_withdisc_rub,
        return_amount: row.return_amount,
        sa_name: row.sa_name.clone(),
        storage_fee: row.storage_fee,
        subject_name: row.subject_name.clone(),
        supplier_oper_name: row.supplier_oper_name.clone(),
        cashback_amount: row.cashback_amount,
        ppvz_for_pay: row.ppvz_for_pay,
        ppvz_kvw_prc: row.ppvz_kvw_prc,
        ppvz_kvw_prc_base: row.ppvz_kvw_prc_base,
        srv_dbs: row.srv_dbs,
        srid: row.srid.clone(),
    }
}

fn active_model_from_model(
    row: &crate::projections::p903_wb_finance_report::repository::Model,
) -> crate::projections::p903_wb_finance_report::repository::ActiveModel {
    crate::projections::p903_wb_finance_report::repository::ActiveModel {
        rr_dt: Set(row.rr_dt.clone()),
        rrd_id: Set(row.rrd_id),
        source_row_ref: Set(row.source_row_ref.clone()),
        connection_mp_ref: Set(row.connection_mp_ref.clone()),
        organization_ref: Set(row.organization_ref.clone()),
        acquiring_fee: Set(row.acquiring_fee),
        acquiring_percent: Set(row.acquiring_percent),
        additional_payment: Set(row.additional_payment),
        bonus_type_name: Set(row.bonus_type_name.clone()),
        commission_percent: Set(row.commission_percent),
        delivery_amount: Set(row.delivery_amount),
        delivery_rub: Set(row.delivery_rub),
        nm_id: Set(row.nm_id),
        penalty: Set(row.penalty),
        ppvz_vw: Set(row.ppvz_vw),
        ppvz_vw_nds: Set(row.ppvz_vw_nds),
        ppvz_sales_commission: Set(row.ppvz_sales_commission),
        quantity: Set(row.quantity),
        rebill_logistic_cost: Set(row.rebill_logistic_cost),
        retail_amount: Set(row.retail_amount),
        retail_price: Set(row.retail_price),
        retail_price_withdisc_rub: Set(row.retail_price_withdisc_rub),
        return_amount: Set(row.return_amount),
        sa_name: Set(row.sa_name.clone()),
        storage_fee: Set(row.storage_fee),
        subject_name: Set(row.subject_name.clone()),
        supplier_oper_name: Set(row.supplier_oper_name.clone()),
        cashback_amount: Set(row.cashback_amount),
        ppvz_for_pay: Set(row.ppvz_for_pay),
        ppvz_kvw_prc: Set(row.ppvz_kvw_prc),
        ppvz_kvw_prc_base: Set(row.ppvz_kvw_prc_base),
        srv_dbs: Set(row.srv_dbs),
        srid: Set(row.srid.clone()),
        loaded_at_utc: Set(row.loaded_at_utc.clone()),
        payload_version: Set(row.payload_version),
        extra: Set(row.extra.clone()),
    }
}

fn model_from_entry(
    row: &crate::projections::p903_wb_finance_report::repository::WbFinanceReportEntry,
    loaded_at_utc: &str,
) -> crate::projections::p903_wb_finance_report::repository::Model {
    crate::projections::p903_wb_finance_report::repository::Model {
        rr_dt: row.rr_dt.format("%Y-%m-%d").to_string(),
        rrd_id: row.rrd_id,
        source_row_ref: row.source_row_ref.clone(),
        connection_mp_ref: row.connection_mp_ref.clone(),
        organization_ref: row.organization_ref.clone(),
        acquiring_fee: row.acquiring_fee,
        acquiring_percent: row.acquiring_percent,
        additional_payment: row.additional_payment,
        bonus_type_name: row.bonus_type_name.clone(),
        commission_percent: row.commission_percent,
        delivery_amount: row.delivery_amount,
        delivery_rub: row.delivery_rub,
        nm_id: row.nm_id,
        penalty: row.penalty,
        ppvz_vw: row.ppvz_vw,
        ppvz_vw_nds: row.ppvz_vw_nds,
        ppvz_sales_commission: row.ppvz_sales_commission,
        quantity: row.quantity,
        rebill_logistic_cost: row.rebill_logistic_cost,
        retail_amount: row.retail_amount,
        retail_price: row.retail_price,
        retail_price_withdisc_rub: row.retail_price_withdisc_rub,
        return_amount: row.return_amount,
        sa_name: row.sa_name.clone(),
        storage_fee: row.storage_fee,
        subject_name: row.subject_name.clone(),
        supplier_oper_name: row.supplier_oper_name.clone(),
        cashback_amount: row.cashback_amount,
        ppvz_for_pay: row.ppvz_for_pay,
        ppvz_kvw_prc: row.ppvz_kvw_prc,
        ppvz_kvw_prc_base: row.ppvz_kvw_prc_base,
        srv_dbs: row.srv_dbs,
        srid: row.srid.clone(),
        loaded_at_utc: loaded_at_utc.to_string(),
        payload_version: row.payload_version,
        extra: row.extra.clone(),
    }
}

async fn load_day_models(
    connection_mp_ref: &str,
    date: NaiveDate,
) -> Result<Vec<crate::projections::p903_wb_finance_report::repository::Model>> {
    let db = get_connection();
    let date_str = date.format("%Y-%m-%d").to_string();

    Ok(
        crate::projections::p903_wb_finance_report::repository::Entity::find()
            .filter(
                crate::projections::p903_wb_finance_report::repository::Column::RrDt.eq(&date_str),
            )
            .filter(
                crate::projections::p903_wb_finance_report::repository::Column::ConnectionMpRef
                    .eq(connection_mp_ref),
            )
            .order_by_asc(crate::projections::p903_wb_finance_report::repository::Column::RrdId)
            .all(db)
            .await?,
    )
}

fn build_general_ledger_entries(
    models: &[crate::projections::p903_wb_finance_report::repository::Model],
) -> Result<Vec<crate::projections::general_ledger::repository::Model>> {
    let posting_id = uuid::Uuid::new_v4().to_string();
    models
        .iter()
        .map(|item| {
            crate::projections::p903_wb_finance_report::general_ledger_builder::build_general_ledger_entries(
                item,
                &posting_id,
            )
        })
        .collect::<Result<Vec<_>>>()
        .map(|items| items.into_iter().flatten().collect())
}

pub async fn reconcile_day(
    connection_mp_ref: &str,
    date: NaiveDate,
    entries: &[crate::projections::p903_wb_finance_report::repository::WbFinanceReportEntry],
) -> Result<ReconcileResult> {
    let db = get_connection();
    let date_str = date.format("%Y-%m-%d").to_string();

    let existing = load_day_models(connection_mp_ref, date).await?;

    let current_snapshot: Vec<SnapshotRow> = existing.iter().map(snapshot_from_model).collect();
    let next_snapshot: Vec<SnapshotRow> = entries.iter().map(snapshot_from_entry).collect();
    if current_snapshot == next_snapshot {
        return Ok(ReconcileResult {
            changed: false,
            source_rows: existing.len(),
            general_ledger_rows:
                crate::projections::general_ledger::repository::count_by_detail_ids(
                    DETAIL_KIND,
                    &existing
                        .iter()
                        .map(|item| item.source_row_ref.clone())
                        .collect::<Vec<_>>(),
                )
                .await?
                .values()
                .copied()
                .sum(),
        });
    }

    let txn = db.begin().await?;

    let source_row_refs = existing
        .iter()
        .map(|item| item.source_row_ref.clone())
        .collect::<Vec<_>>();
    crate::projections::general_ledger::repository::delete_by_detail_ids_with_conn(
        &txn,
        DETAIL_KIND,
        &source_row_refs,
    )
    .await?;

    crate::projections::p903_wb_finance_report::repository::Entity::delete_many()
        .filter(crate::projections::p903_wb_finance_report::repository::Column::RrDt.eq(&date_str))
        .filter(
            crate::projections::p903_wb_finance_report::repository::Column::ConnectionMpRef
                .eq(connection_mp_ref),
        )
        .exec(&txn)
        .await?;

    let loaded_at_utc = chrono::Utc::now().to_rfc3339();
    let models = entries
        .iter()
        .map(|item| model_from_entry(item, &loaded_at_utc))
        .collect::<Vec<_>>();

    for model in &models {
        crate::projections::p903_wb_finance_report::repository::Entity::insert(
            active_model_from_model(model),
        )
        .exec(&txn)
        .await?;
    }

    let general_ledger_entries = build_general_ledger_entries(&models)?;

    for entry in &general_ledger_entries {
        crate::projections::general_ledger::repository::save_entry_with_conn(&txn, entry).await?;
    }

    txn.commit().await?;

    Ok(ReconcileResult {
        changed: true,
        source_rows: models.len(),
        general_ledger_rows: general_ledger_entries.len(),
    })
}

pub async fn rebuild_day_from_existing(
    connection_mp_ref: &str,
    date: NaiveDate,
) -> Result<ReconcileResult> {
    let db = get_connection();
    let existing = load_day_models(connection_mp_ref, date).await?;
    if existing.is_empty() {
        return Ok(ReconcileResult {
            changed: false,
            source_rows: 0,
            general_ledger_rows: 0,
        });
    }

    let txn = db.begin().await?;
    let source_row_refs = existing
        .iter()
        .map(|item| item.source_row_ref.clone())
        .collect::<Vec<_>>();
    crate::projections::general_ledger::repository::delete_by_detail_ids_with_conn(
        &txn,
        DETAIL_KIND,
        &source_row_refs,
    )
    .await?;

    let general_ledger_entries = build_general_ledger_entries(&existing)?;
    for entry in &general_ledger_entries {
        crate::projections::general_ledger::repository::save_entry_with_conn(&txn, entry).await?;
    }

    txn.commit().await?;

    Ok(ReconcileResult {
        changed: true,
        source_rows: existing.len(),
        general_ledger_rows: general_ledger_entries.len(),
    })
}

pub async fn rebuild_range_from_existing(date_from: &str, date_to: &str) -> Result<usize> {
    let parsed_from = NaiveDate::parse_from_str(date_from, "%Y-%m-%d")?;
    let parsed_to = NaiveDate::parse_from_str(date_to, "%Y-%m-%d")?;
    let rows = crate::projections::p903_wb_finance_report::repository::list_by_date_range(
        date_from, date_to,
    )
    .await?;

    let mut day_keys = rows
        .into_iter()
        .map(|row| (row.connection_mp_ref, row.rr_dt))
        .collect::<Vec<_>>();
    day_keys.sort();
    day_keys.dedup();

    for (connection_mp_ref, date_str) in &day_keys {
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?;
        if date < parsed_from || date > parsed_to {
            continue;
        }
        rebuild_day_from_existing(connection_mp_ref, date).await?;
    }

    Ok(day_keys.len())
}
