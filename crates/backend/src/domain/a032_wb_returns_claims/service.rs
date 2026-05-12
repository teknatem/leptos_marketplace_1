use anyhow::Result;
use contracts::domain::a032_wb_returns_claims::aggregate::WbReturnsClaims;
use uuid::Uuid;

use super::repository;

pub async fn get_by_id(id: Uuid) -> Result<Option<WbReturnsClaims>> {
    repository::get_by_id(id).await
}

pub async fn list_all() -> Result<Vec<WbReturnsClaims>> {
    repository::list_all().await
}

pub async fn list_by_connection(connection_id: &str) -> Result<Vec<WbReturnsClaims>> {
    repository::list_by_connection(connection_id).await
}

/// Upsert from import — основной entry point для u504.
/// Возвращает (uuid, was_inserted).
pub async fn upsert(agg: &WbReturnsClaims) -> Result<(Uuid, bool)> {
    repository::upsert_by_claim_key(agg).await
}
