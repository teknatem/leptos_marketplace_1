use anyhow::Result;

use super::repository;

pub async fn get_by_id(id: &str) -> Result<Option<repository::Model>> {
    repository::get_by_id(id).await
}

pub async fn list_by_registrator_ref(registrator_ref: &str) -> Result<Vec<repository::Model>> {
    repository::list_by_registrator_ref(registrator_ref).await
}

pub async fn list_by_general_ledger_ref(
    general_ledger_ref: &str,
) -> Result<Vec<repository::Model>> {
    repository::list_by_general_ledger_ref(general_ledger_ref).await
}

#[allow(clippy::too_many_arguments)]
pub async fn list_with_filters(
    date_from: Option<String>,
    date_to: Option<String>,
    connection_mp_ref: Option<String>,
    nomenclature_ref: Option<String>,
    layer: Option<String>,
    turnover_code: Option<String>,
    registrator_ref: Option<String>,
    general_ledger_ref: Option<String>,
    sort_by: Option<String>,
    sort_desc: bool,
    offset: Option<u64>,
    limit: Option<u64>,
) -> Result<Vec<repository::Model>> {
    repository::list_with_filters(
        date_from,
        date_to,
        connection_mp_ref,
        nomenclature_ref,
        layer,
        turnover_code,
        registrator_ref,
        general_ledger_ref,
        sort_by,
        sort_desc,
        offset,
        limit,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn count_with_filters(
    date_from: Option<String>,
    date_to: Option<String>,
    connection_mp_ref: Option<String>,
    nomenclature_ref: Option<String>,
    layer: Option<String>,
    turnover_code: Option<String>,
    registrator_ref: Option<String>,
    general_ledger_ref: Option<String>,
) -> Result<u64> {
    repository::count_with_filters(
        date_from,
        date_to,
        connection_mp_ref,
        nomenclature_ref,
        layer,
        turnover_code,
        registrator_ref,
        general_ledger_ref,
    )
    .await
}

pub async fn save_entry(entry: &repository::Model) -> Result<()> {
    repository::save_entry(entry).await
}

pub async fn remove_by_registrator_ref(registrator_ref: &str) -> Result<()> {
    repository::delete_by_registrator_ref(registrator_ref).await?;
    Ok(())
}
