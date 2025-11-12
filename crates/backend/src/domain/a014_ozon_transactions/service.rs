use super::repository;
use contracts::domain::a014_ozon_transactions::aggregate::{
    OzonTransactions, OzonTransactionsDto, OzonTransactionsListDto, OzonTransactionsDetailDto,
};
use uuid::Uuid;

pub async fn create(dto: OzonTransactionsDto) -> anyhow::Result<Uuid> {
    let mut aggregate = OzonTransactions::new_for_insert(
        dto.code,
        dto.description,
        dto.header,
        dto.posting,
        dto.items,
        dto.services,
        dto.source_meta,
        dto.is_posted,
    );

    aggregate
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    aggregate.before_write();

    repository::insert(&aggregate).await
}

pub async fn update(id: Uuid, dto: OzonTransactionsDto) -> anyhow::Result<()> {
    let mut aggregate = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Transaction not found"))?;

    // Обновляем поля
    aggregate.base.code = dto.code;
    aggregate.base.description = dto.description;
    aggregate.header = dto.header;
    aggregate.posting = dto.posting;
    aggregate.items = dto.items;
    aggregate.services = dto.services;
    aggregate.source_meta = dto.source_meta;
    aggregate.is_posted = dto.is_posted;

    aggregate
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    aggregate.before_write();

    repository::update(&aggregate).await
}

pub async fn delete(id: Uuid) -> anyhow::Result<bool> {
    repository::soft_delete(id).await
}

pub async fn get_by_id(id: Uuid) -> anyhow::Result<Option<OzonTransactions>> {
    repository::get_by_id(id).await
}

pub async fn list_all() -> anyhow::Result<Vec<OzonTransactions>> {
    repository::list_all().await
}

/// Конвертация агрегата в DTO для списка
pub fn to_list_dto(aggregate: &OzonTransactions) -> OzonTransactionsListDto {
    OzonTransactionsListDto {
        id: aggregate.to_string_id(),
        operation_id: aggregate.header.operation_id,
        operation_type: aggregate.header.operation_type.clone(),
        operation_type_name: aggregate.header.operation_type_name.clone(),
        operation_date: aggregate.header.operation_date.clone(),
        posting_number: aggregate.posting.posting_number.clone(),
        transaction_type: aggregate.header.transaction_type.clone(),
        amount: aggregate.header.amount,
        is_posted: aggregate.is_posted,
        created_at: aggregate.base.metadata.created_at,
        updated_at: aggregate.base.metadata.updated_at,
    }
}

/// Конвертация агрегата в DTO для деталей
pub fn to_detail_dto(aggregate: &OzonTransactions) -> OzonTransactionsDetailDto {
    OzonTransactionsDetailDto {
        id: aggregate.to_string_id(),
        code: aggregate.base.code.clone(),
        description: aggregate.base.description.clone(),
        header: aggregate.header.clone(),
        posting: aggregate.posting.clone(),
        items: aggregate.items.clone(),
        services: aggregate.services.clone(),
        source_meta: aggregate.source_meta.clone(),
        is_posted: aggregate.is_posted,
        created_at: aggregate.base.metadata.created_at,
        updated_at: aggregate.base.metadata.updated_at,
        is_deleted: aggregate.base.metadata.is_deleted,
        version: aggregate.base.metadata.version,
    }
}

/// Получить все транзакции в виде списка DTO
pub async fn list_all_as_dto() -> anyhow::Result<Vec<OzonTransactionsListDto>> {
    let aggregates = list_all().await?;
    Ok(aggregates.iter().map(to_list_dto).collect())
}

/// Получить транзакцию по ID в виде детального DTO
pub async fn get_by_id_as_dto(id: Uuid) -> anyhow::Result<Option<OzonTransactionsDetailDto>> {
    let aggregate = get_by_id(id).await?;
    Ok(aggregate.map(|a| to_detail_dto(&a)))
}
