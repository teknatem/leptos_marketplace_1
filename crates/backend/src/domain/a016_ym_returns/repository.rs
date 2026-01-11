use anyhow::Result;
use chrono::Utc;
use contracts::domain::a016_ym_returns::aggregate::{
    YmReturn, YmReturnHeader, YmReturnId, YmReturnLine, YmReturnSourceMeta, YmReturnState,
};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use sea_orm::entity::prelude::*;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set, Statement,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::data::db::get_connection;
use contracts::domain::a016_ym_returns::aggregate::YmReturnListItemDto;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a016_ym_returns")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub return_id: i64,
    pub order_id: i64,
    pub header_json: String,
    pub lines_json: String,
    pub state_json: String,
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

impl From<Model> for YmReturn {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());

        let header: YmReturnHeader = serde_json::from_str(&m.header_json).unwrap_or_else(|_| {
            panic!(
                "Failed to deserialize header_json for return_id: {}",
                m.return_id
            )
        });
        let lines: Vec<YmReturnLine> = serde_json::from_str(&m.lines_json).unwrap_or_else(|_| {
            panic!(
                "Failed to deserialize lines_json for return_id: {}",
                m.return_id
            )
        });
        let state: YmReturnState = serde_json::from_str(&m.state_json).unwrap_or_else(|_| {
            panic!(
                "Failed to deserialize state_json for return_id: {}",
                m.return_id
            )
        });
        let source_meta: YmReturnSourceMeta = serde_json::from_str(&m.source_meta_json)
            .unwrap_or_else(|_| {
                panic!(
                    "Failed to deserialize source_meta_json for return_id: {}",
                    m.return_id
                )
            });

        YmReturn {
            base: BaseAggregate::with_metadata(
                YmReturnId(uuid),
                m.code,
                m.description,
                m.comment.clone(),
                metadata,
            ),
            header,
            lines,
            state,
            source_meta,
            is_posted: m.is_posted,
        }
    }
}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

pub async fn list_all() -> Result<Vec<YmReturn>> {
    let items: Vec<YmReturn> = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .order_by_desc(Column::UpdatedAt)
        .limit(1000)
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(items)
}

pub async fn get_by_id(id: Uuid) -> Result<Option<YmReturn>> {
    let result = Entity::find_by_id(id.to_string()).one(conn()).await?;
    Ok(result.map(Into::into))
}

pub async fn get_by_return_id(return_id: i64) -> Result<Option<YmReturn>> {
    let result = Entity::find()
        .filter(Column::ReturnId.eq(return_id))
        .one(conn())
        .await?;
    Ok(result.map(Into::into))
}

pub async fn upsert_document(aggregate: &YmReturn) -> Result<Uuid> {
    let uuid = aggregate.base.id.value();
    let existing = get_by_return_id(aggregate.header.return_id).await?;

    let header_json = serde_json::to_string(&aggregate.header)?;
    let lines_json = serde_json::to_string(&aggregate.lines)?;
    let state_json = serde_json::to_string(&aggregate.state)?;
    let source_meta_json = serde_json::to_string(&aggregate.source_meta)?;

    if let Some(existing_doc) = existing {
        let existing_uuid = existing_doc.base.id.value();
        let active = ActiveModel {
            id: Set(existing_uuid.to_string()),
            code: Set(aggregate.base.code.clone()),
            description: Set(aggregate.base.description.clone()),
            comment: Set(aggregate.base.comment.clone()),
            return_id: Set(aggregate.header.return_id),
            order_id: Set(aggregate.header.order_id),
            header_json: Set(header_json),
            lines_json: Set(lines_json),
            state_json: Set(state_json),
            source_meta_json: Set(source_meta_json),
            is_deleted: Set(aggregate.base.metadata.is_deleted),
            is_posted: Set(aggregate.base.metadata.is_posted),
            updated_at: Set(Some(aggregate.base.metadata.updated_at)),
            version: Set(aggregate.base.metadata.version + 1),
            created_at: sea_orm::ActiveValue::NotSet,
        };
        active.update(conn()).await?;
        Ok(existing_uuid)
    } else {
        let active = ActiveModel {
            id: Set(uuid.to_string()),
            code: Set(aggregate.base.code.clone()),
            description: Set(aggregate.base.description.clone()),
            comment: Set(aggregate.base.comment.clone()),
            return_id: Set(aggregate.header.return_id),
            order_id: Set(aggregate.header.order_id),
            header_json: Set(header_json),
            lines_json: Set(lines_json),
            state_json: Set(state_json),
            source_meta_json: Set(source_meta_json),
            is_deleted: Set(aggregate.base.metadata.is_deleted),
            is_posted: Set(aggregate.base.metadata.is_posted),
            created_at: Set(Some(aggregate.base.metadata.created_at)),
            updated_at: Set(Some(aggregate.base.metadata.updated_at)),
            version: Set(aggregate.base.metadata.version),
        };
        active.insert(conn()).await?;
        Ok(uuid)
    }
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

// ============================================
// SQL-based list with pagination
// ============================================

#[derive(Debug, Clone)]
pub struct YmReturnsListQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub return_type: Option<String>,
    pub search_return_id: Option<String>,
    pub search_order_id: Option<String>,
    pub sort_by: String,
    pub sort_desc: bool,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug)]
pub struct YmReturnsListResult {
    pub items: Vec<YmReturnListItemDto>,
    pub total: usize,
}

/// SQL-based list with pagination and filtering
pub async fn list_sql(query: YmReturnsListQuery) -> Result<YmReturnsListResult> {
    let db = get_connection();

    // Build WHERE clause
    let mut conditions = vec!["is_deleted = 0".to_string()];

    if let Some(ref date_from) = query.date_from {
        conditions.push(format!(
            "json_extract(state_json, '$.created_at_source') >= '{}'",
            date_from
        ));
    }
    if let Some(ref date_to) = query.date_to {
        conditions.push(format!(
            "json_extract(state_json, '$.created_at_source') <= '{}T23:59:59'",
            date_to
        ));
    }
    if let Some(ref return_type) = query.return_type {
        conditions.push(format!(
            "json_extract(header_json, '$.return_type') = '{}'",
            return_type
        ));
    }
    if let Some(ref search_return_id) = query.search_return_id {
        if !search_return_id.is_empty() {
            conditions.push(format!(
                "CAST(return_id AS TEXT) LIKE '%{}%'",
                search_return_id
            ));
        }
    }
    if let Some(ref search_order_id) = query.search_order_id {
        if !search_order_id.is_empty() {
            conditions.push(format!(
                "CAST(order_id AS TEXT) LIKE '%{}%'",
                search_order_id
            ));
        }
    }

    let where_clause = conditions.join(" AND ");

    // Map sort field to SQL expression
    let sort_column = match query.sort_by.as_str() {
        "return_id" => "return_id",
        "order_id" => "order_id",
        "return_type" => "json_extract(header_json, '$.return_type')",
        "refund_status" => "json_extract(state_json, '$.refund_status')",
        "created_at_source" => "json_extract(state_json, '$.created_at_source')",
        "fetched_at" => "json_extract(source_meta_json, '$.fetched_at')",
        _ => "json_extract(state_json, '$.created_at_source')",
    };
    let sort_order = if query.sort_desc { "DESC" } else { "ASC" };

    // Count total
    let count_sql = format!(
        "SELECT COUNT(*) as cnt FROM a016_ym_returns WHERE {}",
        where_clause
    );
    let count_stmt = Statement::from_string(sea_orm::DatabaseBackend::Sqlite, count_sql);
    let count_result = db.query_one(count_stmt).await?;
    let total: usize = count_result
        .map(|row| row.try_get::<i64>("", "cnt").unwrap_or(0) as usize)
        .unwrap_or(0);

    // Fetch paginated data
    let select_sql = format!(
        r#"
        SELECT 
            id,
            return_id,
            order_id,
            json_extract(header_json, '$.return_type') as return_type,
            json_extract(state_json, '$.refund_status') as refund_status,
            json_extract(state_json, '$.created_at_source') as created_at_source,
            json_extract(source_meta_json, '$.fetched_at') as fetched_at,
            lines_json,
            is_posted
        FROM a016_ym_returns
        WHERE {}
        ORDER BY {} {}
        LIMIT {} OFFSET {}
        "#,
        where_clause, sort_column, sort_order, query.limit, query.offset
    );

    let stmt = Statement::from_string(sea_orm::DatabaseBackend::Sqlite, select_sql);
    let rows = db.query_all(stmt).await?;

    let items: Vec<YmReturnListItemDto> = rows
        .into_iter()
        .filter_map(|row| {
            let id: String = row.try_get("", "id").ok()?;
            let return_id: i64 = row.try_get("", "return_id").ok()?;
            let order_id: i64 = row.try_get("", "order_id").ok()?;
            let return_type: String = row.try_get("", "return_type").unwrap_or_default();
            let refund_status: String = row.try_get("", "refund_status").unwrap_or_default();
            let created_at_source: String =
                row.try_get("", "created_at_source").unwrap_or_default();
            let fetched_at: String = row.try_get("", "fetched_at").unwrap_or_default();
            let lines_json: String = row.try_get("", "lines_json").unwrap_or_default();
            let is_posted: bool = row.try_get::<i32>("", "is_posted").unwrap_or(0) == 1;

            // Parse lines to calculate totals
            let lines: Vec<serde_json::Value> =
                serde_json::from_str(&lines_json).unwrap_or_default();
            let mut total_items = 0i32;
            let mut total_amount = 0.0f64;

            for line in &lines {
                if let Some(count) = line.get("count").and_then(|c| c.as_i64()) {
                    total_items += count as i32;
                }
                if let Some(price) = line.get("price").and_then(|p| p.as_f64()) {
                    let count = line.get("count").and_then(|c| c.as_i64()).unwrap_or(1) as f64;
                    total_amount += price * count;
                }
            }

            Some(YmReturnListItemDto {
                id,
                return_id,
                order_id,
                return_type,
                refund_status,
                total_items,
                total_amount,
                created_at_source,
                fetched_at,
                is_posted,
            })
        })
        .collect();

    Ok(YmReturnsListResult { items, total })
}
