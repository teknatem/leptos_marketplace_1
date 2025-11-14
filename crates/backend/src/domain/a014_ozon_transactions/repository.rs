use anyhow::Result;
use chrono::Utc;
use contracts::domain::a014_ozon_transactions::aggregate::{
    OzonTransactions, OzonTransactionsId, OzonTransactionsHeader, OzonTransactionsPosting,
    OzonTransactionsItem, OzonTransactionsService, OzonTransactionsSourceMeta,
};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a014_ozon_transactions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub operation_id: i64,
    pub posting_number: String,
    pub header_json: String,
    pub posting_json: String,
    pub items_json: String,
    pub services_json: String,
    pub source_meta_json: String,
    pub is_deleted: bool,
    pub is_posted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for OzonTransactions {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());

        let header: OzonTransactionsHeader =
            serde_json::from_str(&m.header_json).unwrap_or_else(|_| {
                panic!("Failed to deserialize header_json for operation_id: {}", m.operation_id)
            });
        let posting: OzonTransactionsPosting =
            serde_json::from_str(&m.posting_json).unwrap_or_else(|_| {
                panic!("Failed to deserialize posting_json for operation_id: {}", m.operation_id)
            });
        let items: Vec<OzonTransactionsItem> =
            serde_json::from_str(&m.items_json).unwrap_or_else(|_| {
                panic!("Failed to deserialize items_json for operation_id: {}", m.operation_id)
            });
        let services: Vec<OzonTransactionsService> =
            serde_json::from_str(&m.services_json).unwrap_or_else(|_| {
                panic!("Failed to deserialize services_json for operation_id: {}", m.operation_id)
            });
        let source_meta: OzonTransactionsSourceMeta =
            serde_json::from_str(&m.source_meta_json).unwrap_or_else(|_| {
                panic!("Failed to deserialize source_meta_json for operation_id: {}", m.operation_id)
            });

        OzonTransactions {
            base: BaseAggregate::with_metadata(
                OzonTransactionsId(uuid),
                m.code,
                m.description,
                m.comment.clone(),
                metadata,
            ),
            header,
            posting,
            items,
            services,
            source_meta,
            is_posted: m.is_posted,
        }
    }
}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

pub async fn list_all() -> Result<Vec<OzonTransactions>> {
    let all_count = Entity::find().count(conn()).await?;
    let deleted_count = Entity::find()
        .filter(Column::IsDeleted.eq(true))
        .count(conn())
        .await?;

    tracing::info!(
        "A014 list_all: total records={}, deleted={}, active={}",
        all_count,
        deleted_count,
        all_count - deleted_count
    );

    let items: Vec<OzonTransactions> = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

    tracing::info!("A014 list_all: returning {} items", items.len());
    Ok(items)
}

/// Получить список транзакций с фильтрами
pub async fn list_with_filters(
    date_from: Option<String>,
    date_to: Option<String>,
    transaction_type: Option<String>,
    operation_type_name: Option<String>,
    posting_number: Option<String>,
) -> Result<Vec<OzonTransactions>> {
    // Получаем все неудаленные транзакции
    let mut query = Entity::find().filter(Column::IsDeleted.eq(false));

    // Применяем фильтр по posting_number (частичное совпадение)
    if let Some(posting) = posting_number {
        if !posting.is_empty() {
            query = query.filter(Column::PostingNumber.contains(&posting));
        }
    }

    let items: Vec<OzonTransactions> = query
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

    // Применяем фильтры по полям из JSON (парсим в памяти)
    let filtered_items: Vec<OzonTransactions> = items
        .into_iter()
        .filter(|txn| {
            // Фильтр по дате - сравниваем только дату без времени
            if let (Some(from), Some(to)) = (&date_from, &date_to) {
                // Извлекаем только дату из operation_date (формат: "2025-10-11 00:00:00" или "2025-10-11")
                let op_date_only = txn.header.operation_date
                    .split_whitespace()
                    .next()
                    .unwrap_or(&txn.header.operation_date);
                
                if op_date_only < from.as_str() || op_date_only > to.as_str() {
                    return false;
                }
            }

            // Фильтр по transaction_type
            if let Some(ref tt) = transaction_type {
                if !tt.is_empty() && &txn.header.transaction_type != tt {
                    return false;
                }
            }

            // Фильтр по operation_type_name
            if let Some(ref otn) = operation_type_name {
                if !otn.is_empty() && &txn.header.operation_type_name != otn {
                    return false;
                }
            }

            true
        })
        .collect();

    tracing::info!(
        "A014 list_with_filters: returning {} filtered items",
        filtered_items.len()
    );
    Ok(filtered_items)
}

pub async fn get_by_id(id: Uuid) -> Result<Option<OzonTransactions>> {
    let result = Entity::find_by_id(id.to_string()).one(conn()).await?;
    Ok(result.map(Into::into))
}

pub async fn get_by_operation_id(operation_id: i64) -> Result<Option<OzonTransactions>> {
    let result = Entity::find()
        .filter(Column::OperationId.eq(operation_id))
        .one(conn())
        .await?;
    Ok(result.map(Into::into))
}

pub async fn get_by_posting_number(posting_number: &str) -> Result<Vec<OzonTransactions>> {
    let items: Vec<OzonTransactions> = Entity::find()
        .filter(Column::PostingNumber.eq(posting_number))
        .filter(Column::IsDeleted.eq(false))
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    
    tracing::info!("A014 get_by_posting_number: found {} items for posting {}", items.len(), posting_number);
    Ok(items)
}

/// Идемпотентная вставка/обновление по operation_id
pub async fn upsert_by_operation_id(aggregate: &OzonTransactions) -> Result<Uuid> {
    let uuid = aggregate.base.id.value();

    // Проверяем, существует ли транзакция с таким operation_id
    let existing = get_by_operation_id(aggregate.header.operation_id).await?;

    let header_json = serde_json::to_string(&aggregate.header)?;
    let posting_json = serde_json::to_string(&aggregate.posting)?;
    let items_json = serde_json::to_string(&aggregate.items)?;
    let services_json = serde_json::to_string(&aggregate.services)?;
    let source_meta_json = serde_json::to_string(&aggregate.source_meta)?;

    if let Some(existing_txn) = existing {
        // Обновляем существующую транзакцию
        let existing_uuid = existing_txn.base.id.value();
        let active = ActiveModel {
            id: Set(existing_uuid.to_string()),
            code: Set(aggregate.base.code.clone()),
            description: Set(aggregate.base.description.clone()),
            comment: Set(aggregate.base.comment.clone()),
            operation_id: Set(aggregate.header.operation_id),
            posting_number: Set(aggregate.posting.posting_number.clone()),
            header_json: Set(header_json),
            posting_json: Set(posting_json),
            items_json: Set(items_json),
            services_json: Set(services_json),
            source_meta_json: Set(source_meta_json),
            is_deleted: Set(aggregate.base.metadata.is_deleted),
            is_posted: Set(aggregate.is_posted),
            updated_at: Set(Some(aggregate.base.metadata.updated_at)),
            version: Set(aggregate.base.metadata.version + 1),
            created_at: sea_orm::ActiveValue::NotSet,
        };
        active.update(conn()).await?;
        Ok(existing_uuid)
    } else {
        // Вставляем новую транзакцию
        let active = ActiveModel {
            id: Set(uuid.to_string()),
            code: Set(aggregate.base.code.clone()),
            description: Set(aggregate.base.description.clone()),
            comment: Set(aggregate.base.comment.clone()),
            operation_id: Set(aggregate.header.operation_id),
            posting_number: Set(aggregate.posting.posting_number.clone()),
            header_json: Set(header_json),
            posting_json: Set(posting_json),
            items_json: Set(items_json),
            services_json: Set(services_json),
            source_meta_json: Set(source_meta_json),
            is_deleted: Set(aggregate.base.metadata.is_deleted),
            is_posted: Set(aggregate.is_posted),
            created_at: Set(Some(aggregate.base.metadata.created_at)),
            updated_at: Set(Some(aggregate.base.metadata.updated_at)),
            version: Set(aggregate.base.metadata.version),
        };
        active.insert(conn()).await?;
        Ok(uuid)
    }
}

pub async fn insert(aggregate: &OzonTransactions) -> Result<Uuid> {
    let uuid = aggregate.base.id.value();

    let header_json = serde_json::to_string(&aggregate.header)?;
    let posting_json = serde_json::to_string(&aggregate.posting)?;
    let items_json = serde_json::to_string(&aggregate.items)?;
    let services_json = serde_json::to_string(&aggregate.services)?;
    let source_meta_json = serde_json::to_string(&aggregate.source_meta)?;

    let active = ActiveModel {
        id: Set(uuid.to_string()),
        code: Set(aggregate.base.code.clone()),
        description: Set(aggregate.base.description.clone()),
        comment: Set(aggregate.base.comment.clone()),
        operation_id: Set(aggregate.header.operation_id),
        posting_number: Set(aggregate.posting.posting_number.clone()),
        header_json: Set(header_json),
        posting_json: Set(posting_json),
        items_json: Set(items_json),
        services_json: Set(services_json),
        source_meta_json: Set(source_meta_json),
        is_deleted: Set(aggregate.base.metadata.is_deleted),
        is_posted: Set(aggregate.is_posted),
        created_at: Set(Some(aggregate.base.metadata.created_at)),
        updated_at: Set(Some(aggregate.base.metadata.updated_at)),
        version: Set(aggregate.base.metadata.version),
    };
    active.insert(conn()).await?;
    Ok(uuid)
}

pub async fn update(aggregate: &OzonTransactions) -> Result<()> {
    let uuid = aggregate.base.id.value();

    let header_json = serde_json::to_string(&aggregate.header)?;
    let posting_json = serde_json::to_string(&aggregate.posting)?;
    let items_json = serde_json::to_string(&aggregate.items)?;
    let services_json = serde_json::to_string(&aggregate.services)?;
    let source_meta_json = serde_json::to_string(&aggregate.source_meta)?;

    let active = ActiveModel {
        id: Set(uuid.to_string()),
        code: Set(aggregate.base.code.clone()),
        description: Set(aggregate.base.description.clone()),
        comment: Set(aggregate.base.comment.clone()),
        operation_id: Set(aggregate.header.operation_id),
        posting_number: Set(aggregate.posting.posting_number.clone()),
        header_json: Set(header_json),
        posting_json: Set(posting_json),
        items_json: Set(items_json),
        services_json: Set(services_json),
        source_meta_json: Set(source_meta_json),
        is_deleted: Set(aggregate.base.metadata.is_deleted),
        is_posted: Set(aggregate.is_posted),
        updated_at: Set(Some(aggregate.base.metadata.updated_at)),
        version: Set(aggregate.base.metadata.version),
        created_at: sea_orm::ActiveValue::NotSet,
    };
    active.update(conn()).await?;
    Ok(())
}

pub async fn soft_delete(id: Uuid) -> Result<bool> {
    use sea_orm::sea_query::Expr;
    let result = Entity::update_many()
        .col_expr(Column::IsDeleted, Expr::value(true))
        .col_expr(Column::UpdatedAt, Expr::value(Utc::now()))
        .filter(Column::Id.eq(id.to_string()))
        .exec(conn())
        .await?;
    Ok(result.rows_affected > 0)
}
