use super::repository;
use contracts::domain::a008_marketplace_sales::aggregate::{MarketplaceSales, MarketplaceSalesDto};
use uuid::Uuid;

pub async fn create(dto: MarketplaceSalesDto) -> anyhow::Result<Uuid> {
    let code = dto
        .code
        .clone()
        .unwrap_or_else(|| format!("MP-SALE-{}", Uuid::new_v4()));

    let mut aggregate = MarketplaceSales::new_for_insert(
        code,
        dto.description,
        dto.connection_id,
        dto.organization_id,
        dto.marketplace_id,
        dto.accrual_date,
        dto.product_id,
        dto.quantity,
        dto.revenue,
        dto.operation_type,
        dto.comment,
    );

    aggregate
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    aggregate.before_write();

    repository::insert(&aggregate).await
}

pub async fn update(dto: MarketplaceSalesDto) -> anyhow::Result<()> {
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

pub async fn get_by_id(id: Uuid) -> anyhow::Result<Option<MarketplaceSales>> {
    repository::get_by_id(id).await
}

pub async fn list_all() -> anyhow::Result<Vec<MarketplaceSales>> {
    repository::list_all().await
}
