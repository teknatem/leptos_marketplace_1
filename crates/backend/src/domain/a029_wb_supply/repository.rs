use anyhow::Result;
use chrono::Utc;
use contracts::domain::a029_wb_supply::aggregate::{
    WbSupply, WbSupplyHeader, WbSupplyId, WbSupplyInfo, WbSupplyOrderRow, WbSupplySourceMeta,
};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a029_wb_supply")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub supply_id: String,
    pub supply_name: Option<String>,
    pub is_done: bool,
    pub is_b2b: bool,
    pub created_at_wb: Option<String>,
    pub closed_at_wb: Option<String>,
    pub scan_dt: Option<String>,
    pub cargo_type: Option<i32>,
    pub connection_id: String,
    pub organization_id: String,
    pub marketplace_id: String,
    pub info_json: String,
    pub supply_orders_json: String,
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

impl From<Model> for WbSupply {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());

        let header = WbSupplyHeader {
            supply_id: m.supply_id.clone(),
            connection_id: m.connection_id.clone(),
            organization_id: m.organization_id.clone(),
            marketplace_id: m.marketplace_id.clone(),
        };

        let info: WbSupplyInfo = serde_json::from_str(&m.info_json).unwrap_or_else(|_| {
            panic!(
                "Failed to deserialize info_json for supply_id: {}",
                m.supply_id
            )
        });

        let supply_orders: Vec<WbSupplyOrderRow> =
            serde_json::from_str(&m.supply_orders_json).unwrap_or_default();

        let source_meta: WbSupplySourceMeta = serde_json::from_str(&m.source_meta_json)
            .unwrap_or_else(|_| {
                panic!(
                    "Failed to deserialize source_meta_json for supply_id: {}",
                    m.supply_id
                )
            });

        WbSupply {
            base: BaseAggregate::with_metadata(
                WbSupplyId(uuid),
                m.code,
                m.description,
                m.comment,
                metadata,
            ),
            header,
            info,
            source_meta,
            is_posted: m.is_posted,
            supply_orders,
            document_date: m.created_at_wb,
        }
    }
}

pub async fn get_by_id(id: Uuid) -> Result<Option<WbSupply>> {
    let db = get_connection();
    let id_str = id.to_string();
    let model = Entity::find_by_id(id_str).one(db).await?;
    Ok(model.map(|m| m.into()))
}

pub async fn get_by_supply_id(supply_id: &str) -> Result<Option<WbSupply>> {
    let db = get_connection();
    let model = Entity::find()
        .filter(Column::SupplyId.eq(supply_id))
        .one(db)
        .await?;
    Ok(model.map(|m| m.into()))
}

pub async fn upsert_document(document: &WbSupply) -> Result<Uuid> {
    let db = get_connection();
    let uuid = document.base.id.value();

    let info_json = serde_json::to_string(&document.info)?;
    let supply_orders_json = serde_json::to_string(&document.supply_orders)?;
    let source_meta_json = serde_json::to_string(&document.source_meta)?;

    let existing = get_by_supply_id(&document.header.supply_id).await?;

    if let Some(existing_doc) = existing {
        let existing_uuid = existing_doc.base.id.value();
        let active_model = ActiveModel {
            id: Set(existing_uuid.to_string()),
            code: Set(document.base.code.clone()),
            description: Set(document.base.description.clone()),
            comment: Set(document.base.comment.clone()),
            supply_id: Set(document.header.supply_id.clone()),
            supply_name: Set(document.info.name.clone()),
            is_done: Set(document.info.is_done),
            is_b2b: Set(document.info.is_b2b),
            created_at_wb: Set(document.info.created_at_wb.map(|dt| dt.to_rfc3339())),
            closed_at_wb: Set(document.info.closed_at_wb.map(|dt| dt.to_rfc3339())),
            scan_dt: Set(document.info.scan_dt.map(|dt| dt.to_rfc3339())),
            cargo_type: Set(document.info.cargo_type),
            connection_id: Set(document.header.connection_id.clone()),
            organization_id: Set(document.header.organization_id.clone()),
            marketplace_id: Set(document.header.marketplace_id.clone()),
            info_json: Set(info_json),
            supply_orders_json: Set(supply_orders_json),
            source_meta_json: Set(source_meta_json),
            is_deleted: Set(document.base.metadata.is_deleted),
            is_posted: Set(document.is_posted),
            updated_at: Set(Some(Utc::now())),
            version: Set(existing_doc.base.metadata.version + 1),
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
            supply_id: Set(document.header.supply_id.clone()),
            supply_name: Set(document.info.name.clone()),
            is_done: Set(document.info.is_done),
            is_b2b: Set(document.info.is_b2b),
            created_at_wb: Set(document.info.created_at_wb.map(|dt| dt.to_rfc3339())),
            closed_at_wb: Set(document.info.closed_at_wb.map(|dt| dt.to_rfc3339())),
            scan_dt: Set(document.info.scan_dt.map(|dt| dt.to_rfc3339())),
            cargo_type: Set(document.info.cargo_type),
            connection_id: Set(document.header.connection_id.clone()),
            organization_id: Set(document.header.organization_id.clone()),
            marketplace_id: Set(document.header.marketplace_id.clone()),
            info_json: Set(info_json),
            supply_orders_json: Set(supply_orders_json),
            source_meta_json: Set(source_meta_json),
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

/// Query parameters for paginated list
#[derive(Debug, Clone)]
pub struct WbSupplyListQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub connection_id: Option<String>,
    pub organization_id: Option<String>,
    pub search_query: Option<String>,
    pub sort_by: String,
    pub sort_desc: bool,
    pub limit: usize,
    pub offset: usize,
    pub show_done: bool,
}

/// Simplified row for list queries
#[derive(Debug, Clone)]
pub struct WbSupplyListRow {
    pub id: String,
    pub supply_id: String,
    pub supply_name: Option<String>,
    pub is_deleted: bool,
    pub is_done: bool,
    pub is_b2b: bool,
    pub created_at_wb: Option<String>,
    pub closed_at_wb: Option<String>,
    pub cargo_type: Option<i32>,
    pub connection_id: String,
    pub organization_name: Option<String>,
    pub orders_count: i64,
    pub is_posted: bool,
}

/// Result from list query with pagination
#[derive(Debug, Clone)]
pub struct WbSupplyListResult {
    pub items: Vec<WbSupplyListRow>,
    pub total: usize,
}

/// SQL-based list query with pagination and sorting
pub async fn list_sql(query: WbSupplyListQuery) -> Result<WbSupplyListResult> {
    use sea_orm::{ConnectionTrait, Statement, Value};

    let db = get_connection();

    let mut conditions = vec!["1 = 1".to_string()];
    let mut params: Vec<Value> = Vec::new();

    if let Some(ref date_from) = query.date_from {
        if !date_from.is_empty() {
            conditions.push("s.created_at_wb >= ?".to_string());
            params.push(date_from.clone().into());
        }
    }
    if let Some(ref date_to) = query.date_to {
        if !date_to.is_empty() {
            conditions.push("s.created_at_wb <= ?".to_string());
            params.push(date_to.clone().into());
        }
    }
    if let Some(ref conn_id) = query.connection_id {
        if !conn_id.is_empty() {
            conditions.push("s.connection_id = ?".to_string());
            params.push(conn_id.clone().into());
        }
    }
    if let Some(ref org_id) = query.organization_id {
        if !org_id.is_empty() {
            conditions.push(
                "LOWER(TRIM(REPLACE(COALESCE(s.organization_id, ''), '\"', ''))) = \
                 LOWER(TRIM(REPLACE(?, '\"', '')))"
                    .to_string(),
            );
            params.push(org_id.clone().into());
        }
    }
    if let Some(ref search) = query.search_query {
        if !search.is_empty() {
            let like = format!("%{}%", search);
            conditions.push("(s.supply_id LIKE ? OR s.supply_name LIKE ?)".to_string());
            params.push(like.clone().into());
            params.push(like.into());
        }
    }
    if !query.show_done {
        conditions.push("s.is_done = 0".to_string());
    }

    let where_clause = conditions.join(" AND ");

    let count_sql = format!(
        "SELECT COUNT(*) as cnt FROM a029_wb_supply s WHERE {}",
        where_clause
    );
    let count_result = db
        .query_one(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            &count_sql,
            params.clone(),
        ))
        .await?;
    let total = count_result
        .map(|r| r.try_get::<i32>("", "cnt").unwrap_or(0) as usize)
        .unwrap_or(0);

    let order_column = match query.sort_by.as_str() {
        "supply_id" => "s.supply_id",
        "supply_name" => "s.supply_name",
        "is_done" => "s.is_done",
        "created_at_wb" => "s.created_at_wb",
        "closed_at_wb" => "s.closed_at_wb",
        "organization_name" => "org.description",
        _ => "s.created_at_wb",
    };
    let order_dir = if query.sort_desc { "DESC" } else { "ASC" };

    let data_sql = format!(
        r#"SELECT
            s.id,
            s.supply_id,
            s.supply_name,
            s.is_deleted,
            s.is_done,
            s.is_b2b,
            s.created_at_wb,
            s.closed_at_wb,
            s.cargo_type,
            s.connection_id,
            org.description as organization_name,
            CASE
                WHEN json_array_length(s.supply_orders_json) > 0
                THEN json_array_length(s.supply_orders_json)
                WHEN s.supply_id LIKE 'WB-GI-%'
                THEN (
                    SELECT COUNT(*) FROM a015_wb_orders w
                    WHERE w.is_deleted = 0
                      AND json_extract(w.source_meta_json, '$.income_id') > 0
                      AND json_extract(w.source_meta_json, '$.income_id') =
                          CAST(SUBSTR(s.supply_id, 7) AS INTEGER)
                )
                ELSE 0
            END as orders_count,
            s.is_posted
        FROM a029_wb_supply s
        LEFT JOIN a002_organization org
               ON LOWER(TRIM(REPLACE(COALESCE(org.id, ''), '"', '')))
                = LOWER(TRIM(REPLACE(COALESCE(s.organization_id, ''), '"', '')))
              AND org.is_deleted = 0
        WHERE {}
        ORDER BY {} {} NULLS LAST
        LIMIT ? OFFSET ?"#,
        where_clause, order_column, order_dir
    );

    let mut data_params = params;
    data_params.push((query.limit as i64).into());
    data_params.push((query.offset as i64).into());

    let rows = db
        .query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            &data_sql,
            data_params,
        ))
        .await?;

    let items: Vec<WbSupplyListRow> = rows
        .into_iter()
        .filter_map(|row| {
            Some(WbSupplyListRow {
                id: row.try_get("", "id").ok()?,
                supply_id: row.try_get("", "supply_id").ok()?,
                supply_name: row.try_get("", "supply_name").ok(),
                is_deleted: row
                    .try_get::<i32>("", "is_deleted")
                    .map(|v| v != 0)
                    .unwrap_or(false),
                is_done: row
                    .try_get::<i32>("", "is_done")
                    .map(|v| v != 0)
                    .unwrap_or(false),
                is_b2b: row
                    .try_get::<i32>("", "is_b2b")
                    .map(|v| v != 0)
                    .unwrap_or(false),
                created_at_wb: row.try_get("", "created_at_wb").ok(),
                closed_at_wb: row.try_get("", "closed_at_wb").ok(),
                cargo_type: row.try_get("", "cargo_type").ok(),
                connection_id: row.try_get("", "connection_id").unwrap_or_default(),
                organization_name: row.try_get("", "organization_name").ok(),
                orders_count: row.try_get::<i64>("", "orders_count").unwrap_or(0),
                is_posted: row
                    .try_get::<i32>("", "is_posted")
                    .map(|v| v != 0)
                    .unwrap_or(false),
            })
        })
        .collect();

    Ok(WbSupplyListResult { items, total })
}
