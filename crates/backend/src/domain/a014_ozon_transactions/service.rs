use super::repository;
use contracts::domain::a014_ozon_transactions::aggregate::{
    OzonTransactions, OzonTransactionsDetailDto, OzonTransactionsDto, OzonTransactionsListDto,
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
pub fn to_list_dto(
    aggregate: &OzonTransactions,
    substatus: Option<String>,
    delivering_date: Option<String>,
) -> OzonTransactionsListDto {
    OzonTransactionsListDto {
        id: aggregate.to_string_id(),
        operation_id: aggregate.header.operation_id,
        operation_type: aggregate.header.operation_type.clone(),
        operation_type_name: aggregate.header.operation_type_name.clone(),
        operation_date: aggregate.header.operation_date.clone(),
        posting_number: aggregate.posting.posting_number.clone(),
        transaction_type: aggregate.header.transaction_type.clone(),
        delivery_schema: aggregate.posting.delivery_schema.clone(),
        amount: aggregate.header.amount,
        accruals_for_sale: aggregate.header.accruals_for_sale,
        sale_commission: aggregate.header.sale_commission,
        delivery_charge: aggregate.header.delivery_charge,
        substatus,
        delivering_date,
        is_posted: aggregate.is_posted,
        posting_ref: aggregate.posting_ref.clone(),
        posting_ref_type: aggregate.posting_ref_type.clone(),
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
        posting_ref: aggregate.posting_ref.clone(),
        posting_ref_type: aggregate.posting_ref_type.clone(),
        created_at: aggregate.base.metadata.created_at,
        updated_at: aggregate.base.metadata.updated_at,
        is_deleted: aggregate.base.metadata.is_deleted,
        version: aggregate.base.metadata.version,
    }
}

/// Получить все транзакции в виде списка DTO
pub async fn list_all_as_dto() -> anyhow::Result<Vec<OzonTransactionsListDto>> {
    let aggregates = list_all().await?;

    // Собираем уникальные posting_numbers
    let posting_numbers: Vec<String> = aggregates
        .iter()
        .map(|a| a.posting.posting_number.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Получаем соответствующие постинги из обоих типов
    let fbs_postings =
        crate::domain::a010_ozon_fbs_posting::repository::get_by_document_nos(&posting_numbers)
            .await?;
    let fbo_postings =
        crate::domain::a011_ozon_fbo_posting::repository::get_by_document_nos(&posting_numbers)
            .await?;

    // Создаем lookup maps: posting_number -> (substatus, delivering_date)
    let mut posting_data: std::collections::HashMap<String, (Option<String>, Option<String>)> =
        std::collections::HashMap::new();

    // Добавляем данные из FBS постингов
    for p in fbs_postings {
        let date_str = p
            .state
            .delivered_at
            .map(|dt| dt.format("%Y-%m-%d").to_string());
        posting_data.insert(
            p.header.document_no.clone(),
            (p.state.substatus_raw.clone(), date_str),
        );
    }

    // Добавляем данные из FBO постингов
    for p in fbo_postings {
        let date_str = p
            .state
            .delivered_at
            .map(|dt| dt.format("%Y-%m-%d").to_string());
        posting_data.insert(
            p.header.document_no.clone(),
            (p.state.substatus_raw.clone(), date_str),
        );
    }

    Ok(aggregates
        .iter()
        .map(|a| {
            let (substatus, delivering_date) = posting_data
                .get(&a.posting.posting_number)
                .cloned()
                .unwrap_or((None, None));
            to_list_dto(a, substatus, delivering_date)
        })
        .collect())
}

/// Получить транзакцию по ID в виде детального DTO
pub async fn get_by_id_as_dto(id: Uuid) -> anyhow::Result<Option<OzonTransactionsDetailDto>> {
    let aggregate = get_by_id(id).await?;
    Ok(aggregate.map(|a| to_detail_dto(&a)))
}

/// Получить транзакции по posting_number в виде списка DTO
pub async fn get_by_posting_number_as_dto(
    posting_number: &str,
) -> anyhow::Result<Vec<OzonTransactionsListDto>> {
    let aggregates = repository::get_by_posting_number(posting_number).await?;

    // Получаем posting для этого posting_number из обоих типов
    let fbs_posting =
        crate::domain::a010_ozon_fbs_posting::repository::get_by_document_no(posting_number)
            .await?;
    let fbo_posting =
        crate::domain::a011_ozon_fbo_posting::repository::get_by_document_no(posting_number)
            .await?;

    let (substatus, delivering_date) = if let Some(p) = fbs_posting {
        (
            p.state.substatus_raw.clone(),
            p.state
                .delivered_at
                .map(|dt| dt.format("%Y-%m-%d").to_string()),
        )
    } else if let Some(p) = fbo_posting {
        (
            p.state.substatus_raw.clone(),
            p.state
                .delivered_at
                .map(|dt| dt.format("%Y-%m-%d").to_string()),
        )
    } else {
        (None, None)
    };

    Ok(aggregates
        .iter()
        .map(|a| to_list_dto(a, substatus.clone(), delivering_date.clone()))
        .collect())
}

/// Получить все транзакции с фильтрами в виде списка DTO
pub async fn list_with_filters_as_dto(
    date_from: Option<String>,
    date_to: Option<String>,
    transaction_type: Option<String>,
    operation_type_name: Option<String>,
    posting_number: Option<String>,
) -> anyhow::Result<Vec<OzonTransactionsListDto>> {
    let aggregates = repository::list_with_filters(
        date_from,
        date_to,
        transaction_type,
        operation_type_name,
        posting_number,
    )
    .await?;

    // Собираем уникальные posting_numbers
    let posting_numbers: Vec<String> = aggregates
        .iter()
        .map(|a| a.posting.posting_number.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Получаем соответствующие постинги из обоих типов
    let fbs_postings =
        crate::domain::a010_ozon_fbs_posting::repository::get_by_document_nos(&posting_numbers)
            .await?;
    let fbo_postings =
        crate::domain::a011_ozon_fbo_posting::repository::get_by_document_nos(&posting_numbers)
            .await?;

    // Создаем lookup maps: posting_number -> (substatus, delivering_date)
    let mut posting_data: std::collections::HashMap<String, (Option<String>, Option<String>)> =
        std::collections::HashMap::new();

    // Добавляем данные из FBS постингов
    for p in fbs_postings {
        let date_str = p
            .state
            .delivered_at
            .map(|dt| dt.format("%Y-%m-%d").to_string());
        posting_data.insert(
            p.header.document_no.clone(),
            (p.state.substatus_raw.clone(), date_str),
        );
    }

    // Добавляем данные из FBO постингов
    for p in fbo_postings {
        let date_str = p
            .state
            .delivered_at
            .map(|dt| dt.format("%Y-%m-%d").to_string());
        posting_data.insert(
            p.header.document_no.clone(),
            (p.state.substatus_raw.clone(), date_str),
        );
    }

    Ok(aggregates
        .iter()
        .map(|a| {
            let (substatus, delivering_date) = posting_data
                .get(&a.posting.posting_number)
                .cloned()
                .unwrap_or((None, None));
            to_list_dto(a, substatus, delivering_date)
        })
        .collect())
}
