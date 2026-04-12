use anyhow::Result;
use chrono::Utc;
use contracts::domain::a028_missing_cost_registry::aggregate::{
    MissingCostRegistry, MissingCostRegistryId,
};
use contracts::domain::common::{AggregateId, BaseAggregate, EntityMetadata};
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::Expr;
use sea_orm::{ConnectionTrait, EntityTrait, QueryFilter, QuerySelect, Set, Statement};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a028_missing_cost_registry")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub document_no: String,
    pub document_date: String,
    pub lines_json: Option<String>,
    pub is_deleted: bool,
    pub is_posted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for MissingCostRegistry {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());

        MissingCostRegistry {
            base: BaseAggregate::with_metadata(
                MissingCostRegistryId::new(uuid),
                m.code,
                m.description,
                m.comment,
                metadata,
            ),
            document_no: m.document_no,
            document_date: m.document_date,
            lines_json: m.lines_json,
        }
    }
}

pub async fn get_by_id(id: Uuid) -> Result<Option<MissingCostRegistry>> {
    Ok(Entity::find_by_id(id.to_string())
        .one(get_connection())
        .await?
        .map(Into::into))
}

pub async fn get_by_document_date(document_date: &str) -> Result<Option<MissingCostRegistry>> {
    Ok(Entity::find()
        .filter(Column::DocumentDate.eq(document_date))
        .filter(Column::IsDeleted.eq(false))
        .one(get_connection())
        .await?
        .map(Into::into))
}

pub async fn insert_document(doc: &MissingCostRegistry) -> Result<()> {
    let active_model = ActiveModel {
        id: Set(doc.base.id.as_string()),
        code: Set(doc.base.code.clone()),
        description: Set(doc.base.description.clone()),
        comment: Set(doc.base.comment.clone()),
        document_no: Set(doc.document_no.clone()),
        document_date: Set(doc.document_date.clone()),
        lines_json: Set(doc.lines_json.clone()),
        is_deleted: Set(doc.base.metadata.is_deleted),
        is_posted: Set(doc.base.metadata.is_posted),
        created_at: Set(Some(doc.base.metadata.created_at)),
        updated_at: Set(Some(doc.base.metadata.updated_at)),
        version: Set(doc.base.metadata.version.max(1)),
    };
    Entity::insert(active_model).exec(get_connection()).await?;
    Ok(())
}

pub async fn update_document(doc: &MissingCostRegistry) -> Result<()> {
    let active_model = ActiveModel {
        id: Set(doc.base.id.as_string()),
        code: Set(doc.base.code.clone()),
        description: Set(doc.base.description.clone()),
        comment: Set(doc.base.comment.clone()),
        document_no: Set(doc.document_no.clone()),
        document_date: Set(doc.document_date.clone()),
        lines_json: Set(doc.lines_json.clone()),
        is_deleted: Set(doc.base.metadata.is_deleted),
        is_posted: Set(doc.base.metadata.is_posted),
        created_at: sea_orm::ActiveValue::NotSet,
        updated_at: Set(Some(Utc::now())),
        version: Set(doc.base.metadata.version + 1),
    };
    Entity::update(active_model).exec(get_connection()).await?;
    Ok(())
}

pub async fn update_document_if_version(
    doc: &MissingCostRegistry,
    expected_version: i32,
) -> Result<bool> {
    let now = Utc::now();
    let result = Entity::update_many()
        .col_expr(Column::Code, Expr::value(doc.base.code.clone()))
        .col_expr(
            Column::Description,
            Expr::value(doc.base.description.clone()),
        )
        .col_expr(Column::Comment, Expr::value(doc.base.comment.clone()))
        .col_expr(Column::DocumentNo, Expr::value(doc.document_no.clone()))
        .col_expr(Column::DocumentDate, Expr::value(doc.document_date.clone()))
        .col_expr(Column::LinesJson, Expr::value(doc.lines_json.clone()))
        .col_expr(Column::IsDeleted, Expr::value(doc.base.metadata.is_deleted))
        .col_expr(Column::IsPosted, Expr::value(doc.base.metadata.is_posted))
        .col_expr(Column::UpdatedAt, Expr::value(Some(now)))
        .col_expr(Column::Version, Expr::value(expected_version + 1))
        .filter(Column::Id.eq(doc.base.id.as_string()))
        .filter(Column::Version.eq(expected_version))
        .exec(get_connection())
        .await?;
    Ok(result.rows_affected > 0)
}

#[derive(Debug, Clone)]
pub struct MissingCostRegistryListQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub search_query: Option<String>,
    pub sort_by: String,
    pub sort_desc: bool,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone)]
pub struct MissingCostRegistryListRow {
    pub id: String,
    pub document_no: String,
    pub document_date: String,
    pub lines_json: Option<String>,
    pub updated_at: String,
    pub is_posted: bool,
}

#[derive(Debug, Clone)]
pub struct MissingCostRegistryListResult {
    pub items: Vec<MissingCostRegistryListRow>,
    pub total: usize,
}

pub async fn list_sql(
    query: MissingCostRegistryListQuery,
) -> Result<MissingCostRegistryListResult> {
    let db = get_connection();

    let mut conditions = vec!["p.is_deleted = 0".to_string()];

    if let Some(ref date_from) = query.date_from {
        if !date_from.is_empty() {
            conditions.push(format!(
                "p.document_date >= '{}'",
                date_from.replace('\'', "''")
            ));
        }
    }
    if let Some(ref date_to) = query.date_to {
        if !date_to.is_empty() {
            conditions.push(format!(
                "p.document_date <= '{}'",
                date_to.replace('\'', "''")
            ));
        }
    }
    if let Some(ref search) = query.search_query {
        if !search.is_empty() {
            let escaped = search.replace('\'', "''");
            conditions.push(format!(
                "(p.document_no LIKE '%{0}%' OR p.description LIKE '%{0}%')",
                escaped
            ));
        }
    }

    let where_clause = conditions.join(" AND ");

    let sort_column = match query.sort_by.as_str() {
        "document_no" => "p.document_no",
        "updated_at" => "p.updated_at",
        _ => "p.document_date",
    };
    let sort_dir = if query.sort_desc { "DESC" } else { "ASC" };

    let count_sql = format!(
        "SELECT COUNT(*) as cnt FROM a028_missing_cost_registry p WHERE {}",
        where_clause
    );
    let list_sql = format!(
        "SELECT p.id, p.document_no, p.document_date, p.lines_json, p.updated_at, p.is_posted \
         FROM a028_missing_cost_registry p \
         WHERE {} \
         ORDER BY {} {} \
         LIMIT {} OFFSET {}",
        where_clause, sort_column, sort_dir, query.limit, query.offset
    );

    let count_result = db
        .query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            count_sql,
        ))
        .await?;

    let total = count_result
        .and_then(|row| row.try_get::<i64>("", "cnt").ok())
        .unwrap_or(0) as usize;

    let rows = db
        .query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            list_sql,
        ))
        .await?;

    let items = rows
        .into_iter()
        .map(|row| MissingCostRegistryListRow {
            id: row.try_get("", "id").unwrap_or_default(),
            document_no: row.try_get("", "document_no").unwrap_or_default(),
            document_date: row.try_get("", "document_date").unwrap_or_default(),
            lines_json: row.try_get("", "lines_json").ok(),
            updated_at: row.try_get::<String>("", "updated_at").unwrap_or_default(),
            is_posted: row.try_get::<bool>("", "is_posted").unwrap_or(false),
        })
        .collect();

    Ok(MissingCostRegistryListResult { items, total })
}

pub async fn list_ids_by_document_date_range(
    date_from: &str,
    date_to: &str,
    only_posted: bool,
) -> Result<Vec<String>> {
    let mut query = Entity::find()
        .select_only()
        .column(Column::Id)
        .filter(Column::IsDeleted.eq(false))
        .filter(Column::DocumentDate.gte(date_from))
        .filter(Column::DocumentDate.lte(date_to));

    if only_posted {
        query = query.filter(Column::IsPosted.eq(true));
    }

    Ok(query.into_tuple::<String>().all(get_connection()).await?)
}
