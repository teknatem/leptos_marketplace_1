use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::common::AggregateId;
use crate::projections::p903_wb_finance_report::repository::{self, WbFinanceReportEntry};
use super::super::wildberries_api_client::WbFinanceReportRow;

pub async fn process_finance_report_row(
    connection: &ConnectionMP,
    organization_id: &str,
    row: &WbFinanceReportRow,
) -> Result<bool> {
    if row.rrd_id.is_none() || row.rr_dt.is_none() {
        anyhow::bail!("Missing rrd_id or rr_dt");
    }

    let rrd_id = row.rrd_id.unwrap();
    let rr_dt_str = row.rr_dt.clone().unwrap();

    let rr_dt = chrono::NaiveDate::parse_from_str(&rr_dt_str, "%Y-%m-%d")?;

    let extra_json = serde_json::to_string(row).ok();

    let entry = WbFinanceReportEntry {
        rr_dt,
        rrd_id,
        connection_mp_ref: connection.base.id.as_string(),
        organization_ref: organization_id.to_string(),
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
        srv_dbs: row.srv_dbs.map(|b| if b { 1 } else { 0 }),
        srid: row.srid.clone(),
        payload_version: 1,
        extra: extra_json,
    };

    repository::upsert_entry(&entry).await?;
    
    Ok(true)
}

