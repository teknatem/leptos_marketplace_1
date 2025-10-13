use super::repository;
use contracts::domain::a009_ozon_returns::aggregate::{OzonReturns, OzonReturnsDto};
use uuid::Uuid;

pub async fn create(dto: OzonReturnsDto) -> anyhow::Result<Uuid> {
    let code = dto
        .code
        .clone()
        .unwrap_or_else(|| format!("OZON-RET-{}", Uuid::new_v4()));

    let mut aggregate = OzonReturns::new_for_insert(
        code,
        dto.description,
        dto.connection_id,
        dto.organization_id,
        dto.marketplace_id,
        dto.return_id,
        dto.return_date,
        dto.return_reason_name,
        dto.return_type,
        dto.order_id,
        dto.order_number,
        dto.sku,
        dto.product_name,
        dto.price,
        dto.quantity,
        dto.posting_number,
        dto.clearing_id,
        dto.return_clearing_id,
        dto.comment,
    );

    aggregate
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    aggregate.before_write();

    repository::insert(&aggregate).await
}

pub async fn update(dto: OzonReturnsDto) -> anyhow::Result<()> {
    let id = dto
        .id
        .as_ref()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| anyhow::anyhow!("Invalid ID"))?;

    let mut aggregate = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Not found"))?;

    aggregate.update(&dto);

    aggregate
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    aggregate.before_write();

    repository::update(&aggregate).await
}

pub async fn delete(id: Uuid) -> anyhow::Result<bool> {
    repository::soft_delete(id).await
}

pub async fn get_by_id(id: Uuid) -> anyhow::Result<Option<OzonReturns>> {
    repository::get_by_id(id).await
}

pub async fn list_all() -> anyhow::Result<Vec<OzonReturns>> {
    repository::list_all().await
}
