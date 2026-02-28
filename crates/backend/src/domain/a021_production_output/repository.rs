use anyhow::Result;
use chrono::Utc;
use contracts::domain::a021_production_output::aggregate::{ProductionOutput, ProductionOutputId};
use contracts::domain::common::{AggregateId, BaseAggregate, EntityMetadata};
use sea_orm::entity::prelude::*;
use sea_orm::{EntityTrait, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a021_production_output")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub document_no: String,
    pub document_date: String,
    pub article: String,
    pub count: i64,
    pub amount: f64,
    pub cost_of_production: Option<f64>,
    pub nomenclature_ref: Option<String>,
    pub connection_id: String,
    pub fetched_at: String,
    pub is_deleted: bool,
    pub is_posted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for ProductionOutput {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());
        let fetched_at = m
            .fetched_at
            .parse::<chrono::DateTime<Utc>>()
            .unwrap_or_else(|_| Utc::now());

        ProductionOutput {
            base: BaseAggregate::with_metadata(
                ProductionOutputId::new(uuid),
                m.code,
                m.description,
                m.comment,
                metadata,
            ),
            document_no: m.document_no,
            document_date: m.document_date,
            article: m.article,
            count: m.count,
            amount: m.amount,
            cost_of_production: m.cost_of_production,
            nomenclature_ref: m.nomenclature_ref,
            connection_id: m.connection_id,
            fetched_at,
        }
    }
}

pub async fn get_by_id(id: Uuid) -> Result<Option<ProductionOutput>> {
    let db = get_connection();
    let model = Entity::find_by_id(id.to_string()).one(db).await?;
    Ok(model.map(|m| m.into()))
}

/// Upsert документа по ID (1С UUID является первичным ключом)
pub async fn upsert_document(doc: &ProductionOutput) -> Result<bool> {
    let db = get_connection();
    let id_str = doc.base.id.as_string();
    let fetched_at_str = doc.fetched_at.to_rfc3339();

    let existing = Entity::find_by_id(&id_str).one(db).await?;

    if let Some(_existing_model) = existing {
        // UPDATE
        let active_model = ActiveModel {
            id: Set(id_str),
            code: Set(doc.base.code.clone()),
            description: Set(doc.base.description.clone()),
            comment: Set(doc.base.comment.clone()),
            document_no: Set(doc.document_no.clone()),
            document_date: Set(doc.document_date.clone()),
            article: Set(doc.article.clone()),
            count: Set(doc.count),
            amount: Set(doc.amount),
            cost_of_production: Set(doc.cost_of_production),
            nomenclature_ref: Set(doc.nomenclature_ref.clone()),
            connection_id: Set(doc.connection_id.clone()),
            fetched_at: Set(fetched_at_str),
            is_deleted: Set(doc.base.metadata.is_deleted),
            is_posted: Set(doc.base.metadata.is_posted),
            updated_at: Set(Some(Utc::now())),
            version: Set(doc.base.metadata.version + 1),
            created_at: sea_orm::ActiveValue::NotSet,
        };
        Entity::update(active_model).exec(db).await?;
        Ok(false) // обновлено
    } else {
        // INSERT
        let active_model = ActiveModel {
            id: Set(id_str),
            code: Set(doc.base.code.clone()),
            description: Set(doc.base.description.clone()),
            comment: Set(doc.base.comment.clone()),
            document_no: Set(doc.document_no.clone()),
            document_date: Set(doc.document_date.clone()),
            article: Set(doc.article.clone()),
            count: Set(doc.count),
            amount: Set(doc.amount),
            cost_of_production: Set(doc.cost_of_production),
            nomenclature_ref: Set(doc.nomenclature_ref.clone()),
            connection_id: Set(doc.connection_id.clone()),
            fetched_at: Set(fetched_at_str),
            is_deleted: Set(false),
            is_posted: Set(false),
            created_at: Set(Some(Utc::now())),
            updated_at: Set(Some(Utc::now())),
            version: Set(1),
        };
        Entity::insert(active_model).exec(db).await?;
        Ok(true) // вставлено
    }
}

/// Обновить только nomenclature_ref для документа
pub async fn update_nomenclature_ref(id: &str, nomenclature_ref: Option<String>) -> Result<()> {
    let db = get_connection();
    if let Some(model) = Entity::find_by_id(id).one(db).await? {
        let mut active: ActiveModel = model.into();
        active.nomenclature_ref = Set(nomenclature_ref);
        active.updated_at = Set(Some(Utc::now()));
        Entity::update(active).exec(db).await?;
    }
    Ok(())
}

/// Query параметры для списка
#[derive(Debug, Clone)]
pub struct ProductionOutputListQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub search_query: Option<String>,
    pub sort_by: String,
    pub sort_desc: bool,
    pub limit: usize,
    pub offset: usize,
}

/// Упрощённая строка для списка
#[derive(Debug, Clone)]
pub struct ProductionOutputListRow {
    pub id: String,
    pub document_no: String,
    pub document_date: String,
    pub description: String,
    pub article: String,
    pub count: i64,
    pub amount: f64,
    pub cost_of_production: Option<f64>,
    pub nomenclature_ref: Option<String>,
    pub nomenclature_code: Option<String>,
    pub nomenclature_article: Option<String>,
    pub connection_id: String,
    pub fetched_at: String,
    pub is_posted: bool,
}

/// Результат запроса списка с пагинацией
#[derive(Debug, Clone)]
pub struct ProductionOutputListResult {
    pub items: Vec<ProductionOutputListRow>,
    pub total: usize,
}

/// SQL-based список с пагинацией и сортировкой
pub async fn list_sql(query: ProductionOutputListQuery) -> Result<ProductionOutputListResult> {
    use sea_orm::{ConnectionTrait, Statement};

    let db = get_connection();

    let mut conditions = vec!["p.is_deleted = 0".to_string()];

    if let Some(ref date_from) = query.date_from {
        if !date_from.is_empty() {
            conditions.push(format!("p.document_date >= '{}'", date_from));
        }
    }
    if let Some(ref date_to) = query.date_to {
        if !date_to.is_empty() {
            conditions.push(format!("p.document_date <= '{}'", date_to));
        }
    }
    if let Some(ref search) = query.search_query {
        if !search.is_empty() {
            let escaped = search.replace('\'', "''");
            conditions.push(format!(
                "(p.document_no LIKE '%{0}%' OR p.description LIKE '%{0}%' OR p.article LIKE '%{0}%')",
                escaped
            ));
        }
    }

    let where_clause = conditions.join(" AND ");

    let sort_column = match query.sort_by.as_str() {
        "document_no" => "p.document_no",
        "document_date" => "p.document_date",
        "description" => "p.description",
        "article" => "p.article",
        "count" => "p.count",
        "amount" => "p.amount",
        "cost_of_production" => "p.cost_of_production",
        "nomenclature_article" => "n.article",
        _ => "p.document_date",
    };
    let sort_dir = if query.sort_desc { "DESC" } else { "ASC" };

    let count_sql = format!(
        "SELECT COUNT(*) as cnt FROM a021_production_output p WHERE {}",
        where_clause
    );

    let list_sql = format!(
        "SELECT p.id, p.document_no, p.document_date, p.description, p.article, \
         p.count, p.amount, p.cost_of_production, p.nomenclature_ref, \
         p.connection_id, p.fetched_at, p.is_posted, \
         n.code as nomenclature_code, n.article as nomenclature_article \
         FROM a021_production_output p \
         LEFT JOIN a004_nomenclature n ON n.id = p.nomenclature_ref AND n.is_deleted = 0 \
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
        .map(|row| ProductionOutputListRow {
            id: row.try_get("", "id").unwrap_or_default(),
            document_no: row.try_get("", "document_no").unwrap_or_default(),
            document_date: row.try_get("", "document_date").unwrap_or_default(),
            description: row.try_get("", "description").unwrap_or_default(),
            article: row.try_get("", "article").unwrap_or_default(),
            count: row.try_get("", "count").unwrap_or(0),
            amount: row.try_get("", "amount").unwrap_or(0.0),
            cost_of_production: row.try_get("", "cost_of_production").ok(),
            nomenclature_ref: row.try_get("", "nomenclature_ref").ok(),
            nomenclature_code: row.try_get("", "nomenclature_code").ok(),
            nomenclature_article: row.try_get("", "nomenclature_article").ok(),
            connection_id: row.try_get("", "connection_id").unwrap_or_default(),
            fetched_at: row.try_get("", "fetched_at").unwrap_or_default(),
            is_posted: row.try_get::<bool>("", "is_posted").unwrap_or(false),
        })
        .collect();

    Ok(ProductionOutputListResult { items, total })
}

pub async fn soft_delete(id: Uuid) -> Result<bool> {
    let db = get_connection();
    let existing = Entity::find_by_id(id.to_string()).one(db).await?;
    if let Some(model) = existing {
        let mut active: ActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(Some(Utc::now()));
        Entity::update(active).exec(db).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}
