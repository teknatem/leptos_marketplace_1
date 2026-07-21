use anyhow::Result;
use chrono::Utc;
use contracts::domain::a040_wb_search_analytics_daily::aggregate::{
    WbSearchAnalyticsDaily, WbSearchAnalyticsDailyHeader, WbSearchAnalyticsDailyId,
    WbSearchAnalyticsDailySourceMeta,
};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use sea_orm::entity::prelude::*;
use sea_orm::{
    ConnectionTrait, EntityTrait, QueryFilter, QuerySelect, Set, Statement, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::projections::p916_mp_sales_funnel_turnovers::{
    builder as funnel_builder, repository as funnel_repo,
};
use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a040_wb_search_analytics_daily")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub document_no: String,
    pub document_date: String,
    pub connection_id: String,
    pub organization_id: String,
    pub marketplace_id: String,
    pub lines_count: i32,
    pub total_impressions: i64,
    pub total_open_card: i64,
    pub total_orders: i64,
    pub header_json: String,
    pub totals_json: String,
    pub lines_json: String,
    pub source_meta_json: String,
    pub fetched_at: String,
    pub is_deleted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for WbSearchAnalyticsDaily {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: false,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());
        let header: WbSearchAnalyticsDailyHeader =
            serde_json::from_str(&m.header_json).unwrap_or(WbSearchAnalyticsDailyHeader {
                document_no: m.document_no.clone(),
                snapshot_date: m.document_date.clone(),
                connection_id: m.connection_id.clone(),
                organization_id: m.organization_id.clone(),
                marketplace_id: m.marketplace_id.clone(),
            });
        let totals = serde_json::from_str(&m.totals_json).unwrap_or_default();
        let lines = serde_json::from_str(&m.lines_json).unwrap_or_default();
        let source_meta = serde_json::from_str(&m.source_meta_json).unwrap_or(
            WbSearchAnalyticsDailySourceMeta {
                source: "wb_search_analytics".to_string(),
                fetched_at: m.fetched_at.clone(),
            },
        );

        WbSearchAnalyticsDaily {
            base: BaseAggregate::with_metadata(
                WbSearchAnalyticsDailyId::new(uuid),
                m.code,
                m.description,
                m.comment,
                metadata,
            ),
            header,
            totals,
            lines,
            source_meta,
        }
    }
}

pub async fn replace_for_period(
    connection_id: &str,
    date_from: &str,
    date_to: &str,
    documents: &[WbSearchAnalyticsDaily],
) -> Result<usize> {
    let db = get_connection();
    let started_at = std::time::Instant::now();
    tracing::info!(
        "a040_wb_search_analytics_daily replace_for_period: connection={}, period={}..{}, documents={}",
        connection_id,
        date_from,
        date_to,
        documents.len()
    );
    let txn = db.begin().await?;

    Entity::delete_many()
        .filter(Column::ConnectionId.eq(connection_id))
        .filter(Column::DocumentDate.gte(date_from))
        .filter(Column::DocumentDate.lte(date_to))
        .exec(&txn)
        .await?;

    for document in documents {
        insert_with_conn(&txn, document).await?;
    }

    // Стадия 1 воронки p916: показы (show_count). Заменяем движения a040 периода целиком.
    funnel_repo::delete_marketing_for_period_with_conn(
        &txn,
        funnel_builder::REG_A040,
        connection_id,
        date_from,
        date_to,
    )
    .await?;
    for document in documents {
        let registrator_ref = document.base.id.value().to_string();
        let rows = funnel_builder::from_wb_search_analytics(document, &registrator_ref);
        for row in &rows {
            funnel_repo::insert_entry_raw_with_conn(&txn, row).await?;
        }
    }

    txn.commit().await?;
    tracing::info!(
        "a040_wb_search_analytics_daily replace_for_period: committed connection={}, inserted={}, elapsed_ms={}",
        connection_id,
        documents.len(),
        started_at.elapsed().as_millis()
    );
    Ok(documents.len())
}

async fn insert_with_conn<C: ConnectionTrait>(
    db: &C,
    document: &WbSearchAnalyticsDaily,
) -> Result<()> {
    let header_json = serde_json::to_string(&document.header)?;
    let totals_json = serde_json::to_string(&document.totals)?;
    let lines_json = serde_json::to_string(&document.lines)?;
    let source_meta_json = serde_json::to_string(&document.source_meta)?;

    let active_model = ActiveModel {
        id: Set(document.base.id.value().to_string()),
        code: Set(document.base.code.clone()),
        description: Set(document.base.description.clone()),
        comment: Set(document.base.comment.clone()),
        document_no: Set(document.header.document_no.clone()),
        document_date: Set(document.header.snapshot_date.clone()),
        connection_id: Set(document.header.connection_id.clone()),
        organization_id: Set(document.header.organization_id.clone()),
        marketplace_id: Set(document.header.marketplace_id.clone()),
        lines_count: Set(document.lines.len() as i32),
        total_impressions: Set(document.totals.total_impressions),
        total_open_card: Set(document.totals.total_open_card),
        total_orders: Set(document.totals.total_orders),
        header_json: Set(header_json),
        totals_json: Set(totals_json),
        lines_json: Set(lines_json),
        source_meta_json: Set(source_meta_json),
        fetched_at: Set(document.source_meta.fetched_at.clone()),
        is_deleted: Set(false),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        version: Set(1),
    };

    Entity::insert(active_model).exec(db).await?;
    Ok(())
}

pub async fn get_by_id(id: Uuid) -> Result<Option<WbSearchAnalyticsDaily>> {
    let db = get_connection();
    let model = Entity::find_by_id(id.to_string()).one(db).await?;
    Ok(model.map(Into::into))
}

/// Различные `connection_id`, для которых есть хотя бы один снимок.
pub async fn distinct_connection_ids() -> Result<Vec<String>> {
    let db = get_connection();
    let ids = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .select_only()
        .column(Column::ConnectionId)
        .distinct()
        .into_tuple::<String>()
        .all(db)
        .await?;
    Ok(ids)
}

#[derive(Debug, Clone)]
pub struct WbSearchAnalyticsListQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub connection_id: Option<String>,
    pub search_query: Option<String>,
    pub sort_by: String,
    pub sort_desc: bool,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone)]
pub struct WbSearchAnalyticsListRow {
    pub id: String,
    pub document_no: String,
    pub document_date: String,
    pub lines_count: i32,
    pub total_impressions: i64,
    pub total_open_card: i64,
    pub total_orders: i64,
    pub connection_id: String,
    pub connection_name: Option<String>,
    pub organization_name: Option<String>,
    pub fetched_at: String,
}

#[derive(Debug, Clone)]
pub struct WbSearchAnalyticsListResult {
    pub items: Vec<WbSearchAnalyticsListRow>,
    pub total: usize,
}

pub async fn list_sql(query: WbSearchAnalyticsListQuery) -> Result<WbSearchAnalyticsListResult> {
    let db = get_connection();

    let mut conditions = vec!["d.is_deleted = 0".to_string()];
    if let Some(ref date_from) = query.date_from {
        if !date_from.is_empty() {
            conditions.push(format!("d.document_date >= '{}'", date_from.replace('\'', "''")));
        }
    }
    if let Some(ref date_to) = query.date_to {
        if !date_to.is_empty() {
            conditions.push(format!("d.document_date <= '{}'", date_to.replace('\'', "''")));
        }
    }
    if let Some(ref connection_id) = query.connection_id {
        if !connection_id.is_empty() {
            conditions.push(format!("d.connection_id = '{}'", connection_id.replace('\'', "''")));
        }
    }
    if let Some(ref search) = query.search_query {
        if !search.is_empty() {
            let escaped = search.replace('\'', "''");
            conditions.push(format!(
                "(d.document_no LIKE '%{0}%' OR c.description LIKE '%{0}%' OR o.description LIKE '%{0}%')",
                escaped
            ));
        }
    }

    let where_clause = conditions.join(" AND ");
    let sort_column = match query.sort_by.as_str() {
        "document_no" => "d.document_no",
        "document_date" => "d.document_date",
        "lines_count" => "d.lines_count",
        "total_impressions" => "d.total_impressions",
        "total_open_card" => "d.total_open_card",
        "total_orders" => "d.total_orders",
        "connection_name" => "c.description",
        "organization_name" => "o.description",
        "fetched_at" => "d.fetched_at",
        _ => "d.document_date",
    };
    let sort_dir = if query.sort_desc { "DESC" } else { "ASC" };

    let count_sql = format!(
        "SELECT COUNT(*) as cnt
         FROM a040_wb_search_analytics_daily d
         LEFT JOIN a006_connection_mp c ON c.id = d.connection_id
         LEFT JOIN a002_organization o ON o.id = d.organization_id
         WHERE {}",
        where_clause
    );

    let list_sql = format!(
        "SELECT
            d.id, d.document_no, d.document_date, d.lines_count,
            d.total_impressions, d.total_open_card, d.total_orders,
            d.connection_id,
            c.description as connection_name,
            o.description as organization_name,
            d.fetched_at
         FROM a040_wb_search_analytics_daily d
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
        .map(|row| WbSearchAnalyticsListRow {
            id: row.try_get("", "id").unwrap_or_default(),
            document_no: row.try_get("", "document_no").unwrap_or_default(),
            document_date: row.try_get("", "document_date").unwrap_or_default(),
            lines_count: row.try_get("", "lines_count").unwrap_or(0),
            total_impressions: row.try_get("", "total_impressions").unwrap_or(0),
            total_open_card: row.try_get("", "total_open_card").unwrap_or(0),
            total_orders: row.try_get("", "total_orders").unwrap_or(0),
            connection_id: row.try_get("", "connection_id").unwrap_or_default(),
            connection_name: row.try_get("", "connection_name").ok(),
            organization_name: row.try_get("", "organization_name").ok(),
            fetched_at: row.try_get("", "fetched_at").unwrap_or_default(),
        })
        .collect();

    Ok(WbSearchAnalyticsListResult { items, total })
}
