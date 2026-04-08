use anyhow::Result;
use chrono::Utc;
use contracts::domain::a027_wb_documents::aggregate::{
    WbDocument, WbDocumentHeader, WbDocumentId, WbDocumentSourceMeta, WbWeeklyReportManualData,
};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use sea_orm::entity::prelude::*;
use sea_orm::{ConnectionTrait, EntityTrait, QueryFilter, Set, Statement};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a027_wb_documents")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub service_name: String,
    pub name: String,
    pub category: String,
    pub creation_time: String,
    pub viewed: bool,
    pub connection_id: String,
    pub organization_id: String,
    pub marketplace_id: String,
    pub is_weekly_report: bool,
    pub report_period_from: Option<String>,
    pub report_period_to: Option<String>,
    pub weekly_report_manual_json: String,
    pub extensions_json: String,
    pub source_meta_json: String,
    pub is_deleted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for WbDocument {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: false,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());
        let extensions = serde_json::from_str(&m.extensions_json).unwrap_or_default();
        let source_meta =
            serde_json::from_str(&m.source_meta_json).unwrap_or(WbDocumentSourceMeta {
                fetched_at: Utc::now().to_rfc3339(),
                locale: "ru".to_string(),
                document_version: 1,
            });
        let weekly_report_data: WbWeeklyReportManualData =
            serde_json::from_str(&m.weekly_report_manual_json).unwrap_or_default();

        WbDocument {
            base: BaseAggregate::with_metadata(
                WbDocumentId::new(uuid),
                m.code,
                m.description,
                m.comment,
                metadata,
            ),
            header: WbDocumentHeader {
                service_name: m.service_name,
                name: m.name,
                category: m.category,
                extensions,
                creation_time: m.creation_time,
                viewed: m.viewed,
                connection_id: m.connection_id,
                organization_id: m.organization_id,
                marketplace_id: m.marketplace_id,
            },
            is_weekly_report: m.is_weekly_report,
            report_period_from: m.report_period_from,
            report_period_to: m.report_period_to,
            weekly_report_data,
            source_meta,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WbDocumentsListQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub connection_id: Option<String>,
    pub weekly_only: bool,
    pub search_query: Option<String>,
    pub sort_by: String,
    pub sort_desc: bool,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone)]
pub struct WbDocumentsListRow {
    pub id: String,
    pub service_name: String,
    pub name: String,
    pub category: String,
    pub creation_time: String,
    pub is_weekly_report: bool,
    pub report_period_from: Option<String>,
    pub report_period_to: Option<String>,
    pub viewed: bool,
    pub extensions: Vec<String>,
    pub connection_id: String,
    pub connection_name: Option<String>,
    pub organization_name: Option<String>,
    pub fetched_at: String,
}

#[derive(Debug, Clone)]
pub struct WbDocumentsListResult {
    pub items: Vec<WbDocumentsListRow>,
    pub total: usize,
}

pub async fn get_by_id(id: Uuid) -> Result<Option<WbDocument>> {
    let db = get_connection();
    let model = Entity::find_by_id(id.to_string()).one(db).await?;
    Ok(model.map(Into::into))
}

pub async fn get_by_connection_and_service_name(
    connection_id: &str,
    service_name: &str,
) -> Result<Option<WbDocument>> {
    let db = get_connection();
    let model = Entity::find()
        .filter(Column::ConnectionId.eq(connection_id))
        .filter(Column::ServiceName.eq(service_name))
        .filter(Column::IsDeleted.eq(false))
        .one(db)
        .await?;
    Ok(model.map(Into::into))
}

async fn upsert_with_conn<C: ConnectionTrait>(db: &C, document: &WbDocument) -> Result<bool> {
    let existing = Entity::find()
        .filter(Column::ConnectionId.eq(document.header.connection_id.clone()))
        .filter(Column::ServiceName.eq(document.header.service_name.clone()))
        .filter(Column::IsDeleted.eq(false))
        .one(db)
        .await?;

    let extensions_json = serde_json::to_string(&document.header.extensions)?;
    let weekly_report_manual_json = serde_json::to_string(&document.weekly_report_data)?;
    let source_meta_json = serde_json::to_string(&document.source_meta)?;

    let (id, created_at, version, is_new) = if let Some(existing) = existing {
        (
            existing.id,
            existing
                .created_at
                .or(Some(document.base.metadata.created_at)),
            existing.version + 1,
            false,
        )
    } else {
        (
            document.base.id.value().to_string(),
            Some(document.base.metadata.created_at),
            document.base.metadata.version.max(1),
            true,
        )
    };

    let active_model = ActiveModel {
        id: Set(id),
        code: Set(document.base.code.clone()),
        description: Set(document.base.description.clone()),
        comment: Set(document.base.comment.clone()),
        service_name: Set(document.header.service_name.clone()),
        name: Set(document.header.name.clone()),
        category: Set(document.header.category.clone()),
        creation_time: Set(document.header.creation_time.clone()),
        viewed: Set(document.header.viewed),
        connection_id: Set(document.header.connection_id.clone()),
        organization_id: Set(document.header.organization_id.clone()),
        marketplace_id: Set(document.header.marketplace_id.clone()),
        is_weekly_report: Set(document.is_weekly_report),
        report_period_from: Set(document.report_period_from.clone()),
        report_period_to: Set(document.report_period_to.clone()),
        weekly_report_manual_json: Set(weekly_report_manual_json),
        extensions_json: Set(extensions_json),
        source_meta_json: Set(source_meta_json),
        is_deleted: Set(document.base.metadata.is_deleted),
        created_at: Set(created_at),
        updated_at: Set(Some(Utc::now())),
        version: Set(version),
    };

    if is_new {
        active_model.insert(db).await?;
    } else {
        active_model.update(db).await?;
    }

    Ok(is_new)
}

pub async fn upsert_by_service_name(document: &WbDocument) -> Result<bool> {
    let db = get_connection();
    upsert_with_conn(db, document).await
}

pub async fn list_sql(query: WbDocumentsListQuery) -> Result<WbDocumentsListResult> {
    let db = get_connection();

    let mut conditions = vec!["d.is_deleted = 0".to_string()];

    if let Some(ref date_from) = query.date_from {
        if !date_from.is_empty() {
            conditions.push(format!("substr(d.creation_time, 1, 10) >= '{}'", date_from));
        }
    }
    if let Some(ref date_to) = query.date_to {
        if !date_to.is_empty() {
            conditions.push(format!("substr(d.creation_time, 1, 10) <= '{}'", date_to));
        }
    }
    if let Some(ref connection_id) = query.connection_id {
        if !connection_id.is_empty() {
            conditions.push(format!("d.connection_id = '{}'", connection_id));
        }
    }
    if query.weekly_only {
        conditions.push("d.is_weekly_report = 1".to_string());
    }
    if let Some(ref search) = query.search_query {
        if !search.is_empty() {
            let escaped = search.replace('\'', "''");
            conditions.push(format!(
                "(d.service_name LIKE '%{0}%' OR d.name LIKE '%{0}%' OR d.category LIKE '%{0}%' OR c.description LIKE '%{0}%' OR o.description LIKE '%{0}%')",
                escaped
            ));
        }
    }

    let where_clause = conditions.join(" AND ");
    let sort_column = match query.sort_by.as_str() {
        "document_date" => "COALESCE(d.report_period_to, substr(d.creation_time, 1, 10))",
        "service_name" => "d.service_name",
        "name" => "d.name",
        "category" => "d.category",
        "creation_time" => "d.creation_time",
        "is_weekly_report" => "d.is_weekly_report",
        "report_period_from" => "d.report_period_from",
        "report_period_to" => "d.report_period_to",
        "viewed" => "d.viewed",
        "connection_name" => "c.description",
        "organization_name" => "o.description",
        "fetched_at" => "json_extract(d.source_meta_json, '$.fetched_at')",
        _ => "d.creation_time",
    };
    let sort_dir = if query.sort_desc { "DESC" } else { "ASC" };

    let count_sql = format!(
        "SELECT COUNT(*) as cnt
         FROM a027_wb_documents d
         LEFT JOIN a006_connection_mp c ON c.id = d.connection_id
         LEFT JOIN a002_organization o ON o.id = d.organization_id
         WHERE {}",
        where_clause
    );

    let list_sql = format!(
        "SELECT
            d.id,
            d.service_name,
            d.name,
            d.category,
            d.creation_time,
            d.is_weekly_report,
            d.report_period_from,
            d.report_period_to,
            d.viewed,
            d.extensions_json,
            d.connection_id,
            c.description as connection_name,
            o.description as organization_name,
            json_extract(d.source_meta_json, '$.fetched_at') as fetched_at
         FROM a027_wb_documents d
         LEFT JOIN a006_connection_mp c ON c.id = d.connection_id
         LEFT JOIN a002_organization o ON o.id = d.organization_id
         WHERE {}
         ORDER BY {} {}
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
        .map(|row| WbDocumentsListRow {
            id: row.try_get("", "id").unwrap_or_default(),
            service_name: row.try_get("", "service_name").unwrap_or_default(),
            name: row.try_get("", "name").unwrap_or_default(),
            category: row.try_get("", "category").unwrap_or_default(),
            creation_time: row.try_get("", "creation_time").unwrap_or_default(),
            is_weekly_report: row.try_get::<bool>("", "is_weekly_report").unwrap_or(false),
            report_period_from: row.try_get("", "report_period_from").ok(),
            report_period_to: row.try_get("", "report_period_to").ok(),
            viewed: row.try_get::<bool>("", "viewed").unwrap_or(false),
            extensions: serde_json::from_str(
                &row.try_get::<String>("", "extensions_json")
                    .unwrap_or_default(),
            )
            .unwrap_or_default(),
            connection_id: row.try_get("", "connection_id").unwrap_or_default(),
            connection_name: row.try_get("", "connection_name").ok(),
            organization_name: row.try_get("", "organization_name").ok(),
            fetched_at: row.try_get("", "fetched_at").unwrap_or_default(),
        })
        .collect();

    Ok(WbDocumentsListResult { items, total })
}
