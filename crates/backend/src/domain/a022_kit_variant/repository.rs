use anyhow::Result;
use chrono::Utc;
use contracts::domain::a022_kit_variant::aggregate::{KitVariant, KitVariantId};
use contracts::domain::common::{AggregateId, BaseAggregate, EntityMetadata};
use sea_orm::entity::prelude::*;
use sea_orm::{EntityTrait, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a022_kit_variant")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub owner_ref: Option<String>,
    pub goods_json: Option<String>,
    pub connection_id: String,
    pub fetched_at: String,
    pub is_deleted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for KitVariant {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: false,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());
        let fetched_at = m
            .fetched_at
            .parse::<chrono::DateTime<Utc>>()
            .unwrap_or_else(|_| Utc::now());

        KitVariant {
            base: BaseAggregate::with_metadata(
                KitVariantId::new(uuid),
                m.code,
                m.description,
                m.comment,
                metadata,
            ),
            owner_ref: m.owner_ref,
            goods_json: m.goods_json,
            connection_id: m.connection_id,
            fetched_at,
        }
    }
}

pub async fn get_by_id(id: Uuid) -> Result<Option<KitVariant>> {
    let db = get_connection();
    let model = Entity::find_by_id(id.to_string()).one(db).await?;
    Ok(model.map(|m| m.into()))
}

/// Upsert варианта комплектации по ID (1С UUID является первичным ключом)
pub async fn upsert(item: &KitVariant) -> Result<bool> {
    let db = get_connection();
    let id_str = item.base.id.as_string();
    let fetched_at_str = item.fetched_at.to_rfc3339();

    let existing = Entity::find_by_id(&id_str).one(db).await?;

    if let Some(_) = existing {
        let active_model = ActiveModel {
            id: Set(id_str),
            code: Set(item.base.code.clone()),
            description: Set(item.base.description.clone()),
            comment: Set(item.base.comment.clone()),
            owner_ref: Set(item.owner_ref.clone()),
            goods_json: Set(item.goods_json.clone()),
            connection_id: Set(item.connection_id.clone()),
            fetched_at: Set(fetched_at_str),
            is_deleted: Set(item.base.metadata.is_deleted),
            updated_at: Set(Some(Utc::now())),
            version: Set(item.base.metadata.version + 1),
            created_at: sea_orm::ActiveValue::NotSet,
        };
        Entity::update(active_model).exec(db).await?;
        Ok(false)
    } else {
        let active_model = ActiveModel {
            id: Set(id_str),
            code: Set(item.base.code.clone()),
            description: Set(item.base.description.clone()),
            comment: Set(item.base.comment.clone()),
            owner_ref: Set(item.owner_ref.clone()),
            goods_json: Set(item.goods_json.clone()),
            connection_id: Set(item.connection_id.clone()),
            fetched_at: Set(fetched_at_str),
            is_deleted: Set(false),
            created_at: Set(Some(Utc::now())),
            updated_at: Set(Some(Utc::now())),
            version: Set(1),
        };
        Entity::insert(active_model).exec(db).await?;
        Ok(true)
    }
}

/// Query параметры для списка
#[derive(Debug, Clone)]
pub struct KitVariantListQuery {
    pub search_query: Option<String>,
    pub sort_by: String,
    pub sort_desc: bool,
    pub limit: usize,
    pub offset: usize,
}

/// Строка результата списка
#[derive(Debug, Clone)]
pub struct KitVariantListRow {
    pub id: String,
    pub code: String,
    pub description: String,
    pub owner_ref: Option<String>,
    pub owner_description: Option<String>,
    pub owner_article: Option<String>,
    pub goods_json: Option<String>,
    pub connection_id: String,
    pub fetched_at: String,
}

/// Результат с пагинацией
#[derive(Debug, Clone)]
pub struct KitVariantListResult {
    pub items: Vec<KitVariantListRow>,
    pub total: usize,
}

/// SQL-based список с пагинацией и сортировкой
pub async fn list_sql(query: KitVariantListQuery) -> Result<KitVariantListResult> {
    use sea_orm::{ConnectionTrait, Statement};

    let db = get_connection();

    let mut conditions = vec!["k.is_deleted = 0".to_string()];

    if let Some(ref search) = query.search_query {
        if !search.is_empty() {
            let escaped = search.replace('\'', "''");
            conditions.push(format!(
                "(k.description LIKE '%{0}%' OR k.code LIKE '%{0}%' OR n.article LIKE '%{0}%' OR n.description LIKE '%{0}%')",
                escaped
            ));
        }
    }

    let where_clause = conditions.join(" AND ");

    let sort_column = match query.sort_by.as_str() {
        "code" => "k.code",
        "description" => "k.description",
        "owner_description" => "n.description",
        "owner_article" => "n.article",
        _ => "k.description",
    };
    let sort_dir = if query.sort_desc { "DESC" } else { "ASC" };

    let count_sql = format!(
        "SELECT COUNT(*) as cnt FROM a022_kit_variant k WHERE {}",
        where_clause
    );

    let list_sql = format!(
        "SELECT k.id, k.code, k.description, k.owner_ref, k.goods_json, k.connection_id, k.fetched_at, \
         n.description as owner_description, n.article as owner_article \
         FROM a022_kit_variant k \
         LEFT JOIN a004_nomenclature n ON n.id = k.owner_ref AND n.is_deleted = 0 \
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
        .map(|row| KitVariantListRow {
            id: row.try_get("", "id").unwrap_or_default(),
            code: row.try_get("", "code").unwrap_or_default(),
            description: row.try_get("", "description").unwrap_or_default(),
            owner_ref: row.try_get("", "owner_ref").ok(),
            owner_description: row.try_get("", "owner_description").ok(),
            owner_article: row.try_get("", "owner_article").ok(),
            goods_json: row.try_get("", "goods_json").ok(),
            connection_id: row.try_get("", "connection_id").unwrap_or_default(),
            fetched_at: row.try_get("", "fetched_at").unwrap_or_default(),
        })
        .collect();

    Ok(KitVariantListResult { items, total })
}
