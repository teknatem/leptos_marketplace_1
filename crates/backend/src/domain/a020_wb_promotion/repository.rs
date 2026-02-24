use anyhow::Result;
use chrono::Utc;
use contracts::domain::a020_wb_promotion::aggregate::{
    WbPromotion, WbPromotionData, WbPromotionHeader, WbPromotionId, WbPromotionNomenclature,
    WbPromotionSourceMeta,
};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a020_wb_promotion")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub document_no: String,
    pub promotion_id: i64,
    pub name: String,
    pub promotion_description: Option<String>,
    pub start_date_time: String,
    pub end_date_time: String,
    pub promotion_type: Option<String>,
    pub exception_products_count: Option<i32>,
    pub in_promo_action_total: Option<i32>,
    pub header_json: String,
    pub data_json: String,
    pub nomenclatures_json: String,
    pub source_meta_json: String,
    pub connection_id: String,
    pub organization_id: String,
    pub raw_payload_ref: String,
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

impl From<Model> for WbPromotion {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());

        let header: WbPromotionHeader =
            serde_json::from_str(&m.header_json).unwrap_or_else(|_| WbPromotionHeader {
                document_no: m.document_no.clone(),
                connection_id: m.connection_id.clone(),
                organization_id: m.organization_id.clone(),
                marketplace_id: String::new(),
            });

        let data: WbPromotionData =
            serde_json::from_str(&m.data_json).unwrap_or_else(|_| WbPromotionData {
                promotion_id: m.promotion_id,
                name: m.name.clone(),
                description: m.promotion_description.clone(),
                advantages: vec![],
                start_date_time: m.start_date_time.clone(),
                end_date_time: m.end_date_time.clone(),
                promotion_type: m.promotion_type.clone(),
                exception_products_count: m.exception_products_count,
                in_promo_action_total: m.in_promo_action_total,
                in_promo_action_leftovers: None,
                not_in_promo_action_leftovers: None,
                not_in_promo_action_total: None,
                participation_percentage: None,
                ranging: vec![],
            });

        let nomenclatures: Vec<WbPromotionNomenclature> =
            serde_json::from_str(&m.nomenclatures_json).unwrap_or_default();

        let source_meta: WbPromotionSourceMeta =
            serde_json::from_str(&m.source_meta_json).unwrap_or_else(|_| WbPromotionSourceMeta {
                raw_payload_ref: m.raw_payload_ref.clone(),
                fetched_at: m.fetched_at.clone(),
            });

        WbPromotion {
            base: BaseAggregate::with_metadata(
                WbPromotionId(uuid),
                m.code,
                m.description,
                m.comment,
                metadata,
            ),
            header,
            data,
            nomenclatures,
            source_meta,
            is_posted: m.is_posted,
        }
    }
}

pub async fn get_by_id(id: Uuid) -> Result<Option<WbPromotion>> {
    let db = get_connection();
    let id_str = id.to_string();
    let model = Entity::find_by_id(id_str).one(db).await?;
    Ok(model.map(|m| m.into()))
}

pub async fn get_by_promotion_id_and_connection(
    promotion_id: i64,
    connection_id: &str,
) -> Result<Option<WbPromotion>> {
    let db = get_connection();
    let model = Entity::find()
        .filter(Column::PromotionId.eq(promotion_id))
        .filter(Column::ConnectionId.eq(connection_id))
        .filter(Column::IsDeleted.eq(false))
        .one(db)
        .await?;
    Ok(model.map(|m| m.into()))
}

pub async fn upsert_document(document: &WbPromotion) -> Result<Uuid> {
    let db = get_connection();
    let uuid = document.base.id.value();

    let header_json = serde_json::to_string(&document.header)?;
    let data_json = serde_json::to_string(&document.data)?;
    let nomenclatures_json = serde_json::to_string(&document.nomenclatures)?;
    let source_meta_json = serde_json::to_string(&document.source_meta)?;

    let existing = get_by_promotion_id_and_connection(
        document.data.promotion_id,
        &document.header.connection_id,
    )
    .await?;

    if let Some(existing_doc) = existing {
        let existing_uuid = existing_doc.base.id.value();
        let active_model = ActiveModel {
            id: Set(existing_uuid.to_string()),
            code: Set(document.base.code.clone()),
            description: Set(document.base.description.clone()),
            comment: Set(document.base.comment.clone()),
            document_no: Set(document.header.document_no.clone()),
            promotion_id: Set(document.data.promotion_id),
            name: Set(document.data.name.clone()),
            promotion_description: Set(document.data.description.clone()),
            start_date_time: Set(document.data.start_date_time.clone()),
            end_date_time: Set(document.data.end_date_time.clone()),
            promotion_type: Set(document.data.promotion_type.clone()),
            exception_products_count: Set(document.data.exception_products_count),
            in_promo_action_total: Set(document.data.in_promo_action_total),
            header_json: Set(header_json),
            data_json: Set(data_json),
            nomenclatures_json: Set(nomenclatures_json),
            source_meta_json: Set(source_meta_json),
            connection_id: Set(document.header.connection_id.clone()),
            organization_id: Set(document.header.organization_id.clone()),
            raw_payload_ref: Set(document.source_meta.raw_payload_ref.clone()),
            fetched_at: Set(document.source_meta.fetched_at.clone()),
            is_deleted: Set(document.base.metadata.is_deleted),
            is_posted: Set(document.is_posted),
            updated_at: Set(Some(Utc::now())),
            version: Set(document.base.metadata.version + 1),
            created_at: sea_orm::ActiveValue::NotSet,
        };
        Entity::update(active_model).exec(db).await?;
        Ok(existing_uuid)
    } else {
        let active_model = ActiveModel {
            id: Set(uuid.to_string()),
            code: Set(document.base.code.clone()),
            description: Set(document.base.description.clone()),
            comment: Set(document.base.comment.clone()),
            document_no: Set(document.header.document_no.clone()),
            promotion_id: Set(document.data.promotion_id),
            name: Set(document.data.name.clone()),
            promotion_description: Set(document.data.description.clone()),
            start_date_time: Set(document.data.start_date_time.clone()),
            end_date_time: Set(document.data.end_date_time.clone()),
            promotion_type: Set(document.data.promotion_type.clone()),
            exception_products_count: Set(document.data.exception_products_count),
            in_promo_action_total: Set(document.data.in_promo_action_total),
            header_json: Set(header_json),
            data_json: Set(data_json),
            nomenclatures_json: Set(nomenclatures_json),
            source_meta_json: Set(source_meta_json),
            connection_id: Set(document.header.connection_id.clone()),
            organization_id: Set(document.header.organization_id.clone()),
            raw_payload_ref: Set(document.source_meta.raw_payload_ref.clone()),
            fetched_at: Set(document.source_meta.fetched_at.clone()),
            is_deleted: Set(false),
            is_posted: Set(document.is_posted),
            created_at: Set(Some(Utc::now())),
            updated_at: Set(Some(Utc::now())),
            version: Set(1),
        };
        Entity::insert(active_model).exec(db).await?;
        Ok(uuid)
    }
}

pub async fn set_posted(id: Uuid, is_posted: bool) -> Result<()> {
    let db = get_connection();
    let id_str = id.to_string();
    let existing = Entity::find_by_id(&id_str).one(db).await?;
    if let Some(model) = existing {
        let mut active_model: ActiveModel = model.into();
        active_model.is_posted = Set(is_posted);
        active_model.updated_at = Set(Some(Utc::now()));
        Entity::update(active_model).exec(db).await?;
    }
    Ok(())
}

pub async fn soft_delete(id: Uuid) -> Result<bool> {
    let db = get_connection();
    let id_str = id.to_string();
    let existing = Entity::find_by_id(&id_str).one(db).await?;
    if let Some(model) = existing {
        let mut active_model: ActiveModel = model.into();
        active_model.is_deleted = Set(true);
        active_model.updated_at = Set(Some(Utc::now()));
        Entity::update(active_model).exec(db).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[derive(Debug, Clone)]
pub struct WbPromotionListQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub connection_id: Option<String>,
    pub search_query: Option<String>,
    pub sort_by: String,
    pub sort_desc: bool,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct WbPromotionListRow {
    pub id: String,
    pub document_no: String,
    pub promotion_id: i64,
    pub name: String,
    pub promotion_type: Option<String>,
    pub start_date_time: String,
    pub end_date_time: String,
    pub in_promo_action_total: Option<i32>,
    pub nomenclatures_count: i64,
    pub is_posted: bool,
    pub connection_id: String,
    pub organization_id: String,
    pub organization_name: Option<String>,
}

pub async fn list_sql(query: WbPromotionListQuery) -> Result<(Vec<WbPromotionListRow>, usize)> {
    let db = get_connection();

    let mut conditions = vec!["p.is_deleted = 0".to_string()];

    if let Some(ref date_from) = query.date_from {
        if !date_from.is_empty() {
            conditions.push(format!("p.end_date_time >= '{}'", date_from));
        }
    }
    if let Some(ref date_to) = query.date_to {
        if !date_to.is_empty() {
            conditions.push(format!("p.start_date_time <= '{}'", date_to));
        }
    }
    if let Some(ref conn_id) = query.connection_id {
        if !conn_id.is_empty() {
            conditions.push(format!("p.connection_id = '{}'", conn_id));
        }
    }
    if let Some(ref q) = query.search_query {
        if !q.is_empty() {
            let escaped = q.replace('\'', "''");
            conditions.push(format!(
                "(p.name LIKE '%{}%' OR p.promotion_type LIKE '%{}%')",
                escaped, escaped
            ));
        }
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sort_col = match query.sort_by.as_str() {
        "name" => "p.name",
        "promotion_type" => "p.promotion_type",
        "start_date_time" => "p.start_date_time",
        "end_date_time" => "p.end_date_time",
        "in_promo_action_total" => "p.in_promo_action_total",
        _ => "p.start_date_time",
    };
    let sort_dir = if query.sort_desc { "DESC" } else { "ASC" };

    let count_sql = format!(
        "SELECT COUNT(*) as cnt FROM a020_wb_promotion p {}",
        where_clause
    );

    let data_sql = format!(
        r#"SELECT
            p.id,
            p.document_no,
            p.promotion_id,
            p.name,
            p.promotion_type,
            p.start_date_time,
            p.end_date_time,
            p.in_promo_action_total,
            (SELECT json_array_length(p2.nomenclatures_json) FROM a020_wb_promotion p2 WHERE p2.id = p.id) as nomenclatures_count,
            p.is_posted,
            p.connection_id,
            p.organization_id,
            o.description as organization_name
        FROM a020_wb_promotion p
        LEFT JOIN a002_organization o ON p.organization_id = o.id
        {}
        ORDER BY {} {}
        LIMIT {} OFFSET {}"#,
        where_clause, sort_col, sort_dir, query.limit, query.offset
    );

    let total = {
        let result = sea_orm::ConnectionTrait::query_one(
            db,
            sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                count_sql.clone(),
            ),
        )
        .await?;
        result
            .and_then(|row| row.try_get::<i64>("", "cnt").ok())
            .unwrap_or(0) as usize
    };

    let rows = sea_orm::ConnectionTrait::query_all(
        db,
        sea_orm::Statement::from_string(sea_orm::DatabaseBackend::Sqlite, data_sql.clone()),
    )
    .await?;

    let items = rows
        .into_iter()
        .map(|row| {
            let nomenclatures_count: i64 = row.try_get("", "nomenclatures_count").unwrap_or(0);
            WbPromotionListRow {
                id: row.try_get("", "id").unwrap_or_default(),
                document_no: row.try_get("", "document_no").unwrap_or_default(),
                promotion_id: row.try_get("", "promotion_id").unwrap_or(0),
                name: row.try_get("", "name").unwrap_or_default(),
                promotion_type: row.try_get("", "promotion_type").ok(),
                start_date_time: row.try_get("", "start_date_time").unwrap_or_default(),
                end_date_time: row.try_get("", "end_date_time").unwrap_or_default(),
                in_promo_action_total: row.try_get("", "in_promo_action_total").ok(),
                nomenclatures_count,
                is_posted: row.try_get::<bool>("", "is_posted").unwrap_or(false),
                connection_id: row.try_get("", "connection_id").unwrap_or_default(),
                organization_id: row.try_get("", "organization_id").unwrap_or_default(),
                organization_name: row.try_get("", "organization_name").ok(),
            }
        })
        .collect();

    Ok((items, total))
}
