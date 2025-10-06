use super::repository;
use contracts::domain::a003_counterparty::aggregate::{Counterparty, CounterpartyDto};
use uuid::Uuid;

pub async fn create(dto: CounterpartyDto) -> anyhow::Result<Uuid> {
    let code = dto
        .code
        .clone()
        .unwrap_or_else(|| format!("CNT-{}", Uuid::new_v4()));
    let mut aggregate = Counterparty::new_for_insert(
        code,
        dto.description,
        dto.is_folder,
        dto.parent_id,
        dto.inn.unwrap_or_default(),
        dto.kpp.unwrap_or_default(),
        dto.comment,
    );

    aggregate
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;
    aggregate.before_write();

    repository::insert(&aggregate).await
}

pub async fn update(dto: CounterpartyDto) -> anyhow::Result<()> {
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

pub async fn get_by_id(id: Uuid) -> anyhow::Result<Option<Counterparty>> {
    repository::get_by_id(id).await
}

pub async fn list_all() -> anyhow::Result<Vec<Counterparty>> {
    repository::list_all().await
}
