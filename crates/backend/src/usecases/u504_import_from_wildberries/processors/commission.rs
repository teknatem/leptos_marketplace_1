use super::super::wildberries_api_client::CommissionTariffRow;
use crate::projections::p905_wb_commission_history::repository::{self, CommissionEntry};
use anyhow::Result;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;

pub async fn process_commission_tariff(
    _connection: &ConnectionMP,
    tariff: &CommissionTariffRow,
    today: chrono::NaiveDate,
) -> Result<(bool, bool)> {
    let latest = repository::get_latest_by_subject(tariff.subject_id).await?;
    let current_json = serde_json::to_string(tariff)?;

    let should_create = match latest {
        None => true,
        Some(ref existing) => existing.raw_json != current_json,
    };

    if should_create {
        let entry = CommissionEntry {
            id: uuid::Uuid::new_v4().to_string(),
            date: today,
            subject_id: tariff.subject_id,
            subject_name: tariff.subject_name.clone(),
            parent_id: tariff.parent_id,
            parent_name: tariff.parent_name.clone(),
            kgvp_booking: tariff.kgvp_booking,
            kgvp_marketplace: tariff.kgvp_marketplace,
            kgvp_pickup: tariff.kgvp_pickup,
            kgvp_supplier: tariff.kgvp_supplier,
            kgvp_supplier_express: tariff.kgvp_supplier_express,
            paid_storage_kgvp: tariff.paid_storage_kgvp,
            raw_json: current_json,
            payload_version: 1,
        };

        repository::upsert_entry(&entry).await?;
        Ok((true, latest.is_none())) // (created, is_new)
    } else {
        Ok((false, false))
    }
}
