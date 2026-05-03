use anyhow::Result;
use chrono::Utc;
use contracts::domain::a026_wb_advert_daily::aggregate::{
    WbAdvertDaily, WbAdvertDailyHeader, WbAdvertDailyId, WbAdvertDailyMetrics,
    WbAdvertDailySourceMeta,
};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use sea_orm::entity::prelude::*;
use sea_orm::{
    ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set, Statement, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::shared::data::db::get_connection;

fn projection_registrator_ref(document_id: &str) -> String {
    format!("a026:{document_id}")
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a026_wb_advert_daily")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub advert_id: i64,
    pub document_no: String,
    pub document_date: String,
    pub connection_id: String,
    pub organization_id: String,
    pub marketplace_id: String,
    pub lines_count: i32,
    pub total_views: i64,
    pub total_clicks: i64,
    pub total_orders: i64,
    pub total_sum: f64,
    pub total_sum_price: f64,
    pub header_json: String,
    pub totals_json: String,
    pub unattributed_totals_json: String,
    pub lines_json: String,
    pub source_meta_json: String,
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

impl From<Model> for WbAdvertDaily {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());
        let mut header: WbAdvertDailyHeader =
            serde_json::from_str(&m.header_json).unwrap_or(WbAdvertDailyHeader {
                document_no: m.document_no.clone(),
                document_date: m.document_date.clone(),
                advert_id: m.advert_id,
                connection_id: m.connection_id.clone(),
                organization_id: m.organization_id.clone(),
                marketplace_id: m.marketplace_id.clone(),
            });
        header.advert_id = m.advert_id;
        let totals = serde_json::from_str(&m.totals_json).unwrap_or_default();
        let unattributed_totals =
            serde_json::from_str(&m.unattributed_totals_json).unwrap_or_default();
        let lines = serde_json::from_str(&m.lines_json).unwrap_or_default();
        let source_meta =
            serde_json::from_str(&m.source_meta_json).unwrap_or(WbAdvertDailySourceMeta {
                source: "wb_advert_stats".to_string(),
                fetched_at: m.fetched_at.clone(),
            });

        WbAdvertDaily {
            base: BaseAggregate::with_metadata(
                WbAdvertDailyId::new(uuid),
                m.code,
                m.description,
                m.comment,
                metadata,
            ),
            header,
            totals,
            unattributed_totals,
            lines,
            source_meta,
            is_posted: m.is_posted,
        }
    }
}

pub async fn replace_for_period(
    connection_id: &str,
    date_from: &str,
    date_to: &str,
    documents: &[WbAdvertDaily],
) -> Result<usize> {
    replace_for_period_scoped(connection_id, date_from, date_to, None, documents).await
}

pub async fn replace_for_period_advert_ids(
    connection_id: &str,
    date_from: &str,
    date_to: &str,
    advert_ids: &[i64],
    documents: &[WbAdvertDaily],
) -> Result<usize> {
    replace_for_period_scoped(
        connection_id,
        date_from,
        date_to,
        Some(advert_ids),
        documents,
    )
    .await
}

async fn replace_for_period_scoped(
    connection_id: &str,
    date_from: &str,
    date_to: &str,
    advert_ids: Option<&[i64]>,
    documents: &[WbAdvertDaily],
) -> Result<usize> {
    let db = get_connection();
    let started_at = std::time::Instant::now();
    let advert_scope: Option<Vec<i64>> = advert_ids.map(|ids| {
        ids.iter()
            .copied()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect()
    });
    tracing::info!(
        "a026_wb_advert_daily replace_for_period: acquiring transaction connection={}, period={}..{}, advert_scope={:?}, documents={}",
        connection_id,
        date_from,
        date_to,
        advert_scope,
        documents.len()
    );
    let txn = db.begin().await?;
    tracing::info!(
        "a026_wb_advert_daily replace_for_period: transaction started connection={}, elapsed_ms={}",
        connection_id,
        started_at.elapsed().as_millis()
    );

    let mut existing_query = Entity::find()
        .filter(Column::ConnectionId.eq(connection_id))
        .filter(Column::DocumentDate.gte(date_from))
        .filter(Column::DocumentDate.lte(date_to));
    if let Some(scope) = advert_scope.as_ref() {
        existing_query = existing_query.filter(Column::AdvertId.is_in(scope.clone()));
    }

    let existing_ids: Vec<String> = existing_query
        .all(&txn)
        .await?
        .into_iter()
        .map(|item| item.id)
        .collect();

    tracing::info!(
        "a026_wb_advert_daily replace_for_period: found existing documents connection={}, count={}",
        connection_id,
        existing_ids.len()
    );

    for id in &existing_ids {
        let registrator_ref = id.clone();
        let projection_ref = projection_registrator_ref(id);
        crate::projections::general_ledger::repository::delete_by_registrator_with_conn(
            &txn,
            "a026_wb_advert_daily",
            &registrator_ref,
        )
        .await?;
        crate::projections::p911_wb_advert_by_items::repository::delete_by_registrator_ref_with_conn(
            &txn,
            &projection_ref,
        )
        .await?;
    }

    tracing::info!(
        "a026_wb_advert_daily replace_for_period: GL and p911 cleanup completed connection={}, elapsed_ms={}",
        connection_id,
        started_at.elapsed().as_millis()
    );

    let mut delete_query = Entity::delete_many()
        .filter(Column::ConnectionId.eq(connection_id))
        .filter(Column::DocumentDate.gte(date_from))
        .filter(Column::DocumentDate.lte(date_to));
    if let Some(scope) = advert_scope.as_ref() {
        delete_query = delete_query.filter(Column::AdvertId.is_in(scope.clone()));
    }
    delete_query.exec(&txn).await?;

    tracing::info!(
        "a026_wb_advert_daily replace_for_period: source rows deleted connection={}, elapsed_ms={}",
        connection_id,
        started_at.elapsed().as_millis()
    );

    for document in documents {
        insert_with_conn(&txn, document).await?;
    }

    txn.commit().await?;
    tracing::info!(
        "a026_wb_advert_daily replace_for_period: committed connection={}, inserted={}, elapsed_ms={}",
        connection_id,
        documents.len(),
        started_at.elapsed().as_millis()
    );
    Ok(documents.len())
}

async fn insert_with_conn<C: ConnectionTrait>(db: &C, document: &WbAdvertDaily) -> Result<()> {
    let header_json = serde_json::to_string(&document.header)?;
    let totals_json = serde_json::to_string(&document.totals)?;
    let unattributed_totals_json = serde_json::to_string(&document.unattributed_totals)?;
    let lines_json = serde_json::to_string(&document.lines)?;
    let source_meta_json = serde_json::to_string(&document.source_meta)?;

    let active_model = ActiveModel {
        id: Set(document.base.id.value().to_string()),
        code: Set(document.base.code.clone()),
        description: Set(document.base.description.clone()),
        comment: Set(document.base.comment.clone()),
        advert_id: Set(document.header.advert_id),
        document_no: Set(document.header.document_no.clone()),
        document_date: Set(document.header.document_date.clone()),
        connection_id: Set(document.header.connection_id.clone()),
        organization_id: Set(document.header.organization_id.clone()),
        marketplace_id: Set(document.header.marketplace_id.clone()),
        lines_count: Set(document.lines.len() as i32),
        total_views: Set(document.totals.views),
        total_clicks: Set(document.totals.clicks),
        total_orders: Set(document.totals.orders),
        total_sum: Set(document.totals.sum),
        total_sum_price: Set(document.totals.sum_price),
        header_json: Set(header_json),
        totals_json: Set(totals_json),
        unattributed_totals_json: Set(unattributed_totals_json),
        lines_json: Set(lines_json),
        source_meta_json: Set(source_meta_json),
        fetched_at: Set(document.source_meta.fetched_at.clone()),
        is_deleted: Set(false),
        is_posted: Set(document.is_posted),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        version: Set(1),
    };

    Entity::insert(active_model).exec(db).await?;
    Ok(())
}

pub async fn upsert_document(document: &WbAdvertDaily) -> Result<()> {
    let db = get_connection();
    let existing = Entity::find_by_id(document.base.id.value().to_string())
        .one(db)
        .await?;

    let header_json = serde_json::to_string(&document.header)?;
    let totals_json = serde_json::to_string(&document.totals)?;
    let unattributed_totals_json = serde_json::to_string(&document.unattributed_totals)?;
    let lines_json = serde_json::to_string(&document.lines)?;
    let source_meta_json = serde_json::to_string(&document.source_meta)?;

    let created_at = existing
        .as_ref()
        .and_then(|item| item.created_at)
        .or(Some(document.base.metadata.created_at));

    let active_model = ActiveModel {
        id: Set(document.base.id.value().to_string()),
        code: Set(document.base.code.clone()),
        description: Set(document.base.description.clone()),
        comment: Set(document.base.comment.clone()),
        advert_id: Set(document.header.advert_id),
        document_no: Set(document.header.document_no.clone()),
        document_date: Set(document.header.document_date.clone()),
        connection_id: Set(document.header.connection_id.clone()),
        organization_id: Set(document.header.organization_id.clone()),
        marketplace_id: Set(document.header.marketplace_id.clone()),
        lines_count: Set(document.lines.len() as i32),
        total_views: Set(document.totals.views),
        total_clicks: Set(document.totals.clicks),
        total_orders: Set(document.totals.orders),
        total_sum: Set(document.totals.sum),
        total_sum_price: Set(document.totals.sum_price),
        header_json: Set(header_json),
        totals_json: Set(totals_json),
        unattributed_totals_json: Set(unattributed_totals_json),
        lines_json: Set(lines_json),
        source_meta_json: Set(source_meta_json),
        fetched_at: Set(document.source_meta.fetched_at.clone()),
        is_deleted: Set(document.base.metadata.is_deleted),
        is_posted: Set(document.base.metadata.is_posted || document.is_posted),
        created_at: Set(created_at),
        updated_at: Set(Some(Utc::now())),
        version: Set(document.base.metadata.version),
    };

    if existing.is_some() {
        active_model.update(db).await?;
    } else {
        active_model.insert(db).await?;
    }

    Ok(())
}

pub async fn get_by_id(id: Uuid) -> Result<Option<WbAdvertDaily>> {
    let db = get_connection();
    let model = Entity::find_by_id(id.to_string()).one(db).await?;
    Ok(model.map(Into::into))
}

pub async fn list_ids_by_period(
    date_from: &str,
    date_to: &str,
    only_posted: bool,
) -> Result<Vec<String>> {
    let db = get_connection();
    let mut query = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .filter(Column::DocumentDate.gte(date_from))
        .filter(Column::DocumentDate.lte(date_to));

    if only_posted {
        query = query.filter(Column::IsPosted.eq(true));
    }

    let items = query.all(db).await?;

    Ok(items.into_iter().map(|item| item.id).collect())
}

#[derive(Debug, Clone)]
pub struct WbAdvertDailyListQuery {
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
pub struct WbAdvertDailyListRow {
    pub id: String,
    pub document_no: String,
    pub document_date: String,
    pub advert_id: i64,
    pub lines_count: i32,
    pub total_views: i64,
    pub total_clicks: i64,
    pub total_orders: i64,
    pub total_sum: f64,
    pub total_sum_price: f64,
    pub connection_id: String,
    pub connection_name: Option<String>,
    pub organization_name: Option<String>,
    pub fetched_at: String,
    pub is_posted: bool,
}

#[derive(Debug, Clone)]
pub struct WbAdvertDailyListResult {
    pub items: Vec<WbAdvertDailyListRow>,
    pub total: usize,
}

pub async fn list_sql(query: WbAdvertDailyListQuery) -> Result<WbAdvertDailyListResult> {
    let db = get_connection();

    let mut conditions = vec!["d.is_deleted = 0".to_string()];

    if let Some(ref date_from) = query.date_from {
        if !date_from.is_empty() {
            conditions.push(format!("d.document_date >= '{}'", date_from));
        }
    }
    if let Some(ref date_to) = query.date_to {
        if !date_to.is_empty() {
            conditions.push(format!("d.document_date <= '{}'", date_to));
        }
    }
    if let Some(ref connection_id) = query.connection_id {
        if !connection_id.is_empty() {
            conditions.push(format!("d.connection_id = '{}'", connection_id));
        }
    }
    if let Some(ref search) = query.search_query {
        if !search.is_empty() {
            let escaped = search.replace('\'', "''");
            conditions.push(format!(
                "(d.document_no LIKE '%{0}%' OR CAST(d.advert_id AS TEXT) LIKE '%{0}%' OR c.description LIKE '%{0}%' OR o.description LIKE '%{0}%')",
                escaped
            ));
        }
    }

    let where_clause = conditions.join(" AND ");
    let sort_column = match query.sort_by.as_str() {
        "document_no" => "d.document_no",
        "advert_id" => "d.advert_id",
        "document_date" => "d.document_date",
        "lines_count" => "d.lines_count",
        "total_views" => "d.total_views",
        "total_clicks" => "d.total_clicks",
        "total_orders" => "d.total_orders",
        "total_sum" => "d.total_sum",
        "total_sum_price" => "d.total_sum_price",
        "connection_name" => "c.description",
        "organization_name" => "o.description",
        "fetched_at" => "d.fetched_at",
        _ => "d.document_date",
    };
    let sort_dir = if query.sort_desc { "DESC" } else { "ASC" };

    let count_sql = format!(
        "SELECT COUNT(*) as cnt
         FROM a026_wb_advert_daily d
         LEFT JOIN a006_connection_mp c ON c.id = d.connection_id
         LEFT JOIN a002_organization o ON o.id = d.organization_id
         WHERE {}",
        where_clause
    );

    let list_sql = format!(
        "SELECT
            d.id,
            d.document_no,
            d.document_date,
            d.advert_id,
            d.lines_count,
            d.total_views,
            d.total_clicks,
            d.total_orders,
            d.total_sum,
            d.total_sum_price,
            d.connection_id,
            c.description as connection_name,
            o.description as organization_name,
            d.fetched_at,
            d.is_posted
         FROM a026_wb_advert_daily d
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
        .map(|row| WbAdvertDailyListRow {
            id: row.try_get("", "id").unwrap_or_default(),
            document_no: row.try_get("", "document_no").unwrap_or_default(),
            document_date: row.try_get("", "document_date").unwrap_or_default(),
            advert_id: row.try_get("", "advert_id").unwrap_or(0),
            lines_count: row.try_get("", "lines_count").unwrap_or(0),
            total_views: row.try_get("", "total_views").unwrap_or(0),
            total_clicks: row.try_get("", "total_clicks").unwrap_or(0),
            total_orders: row.try_get("", "total_orders").unwrap_or(0),
            total_sum: row.try_get("", "total_sum").unwrap_or(0.0),
            total_sum_price: row.try_get("", "total_sum_price").unwrap_or(0.0),
            connection_id: row.try_get("", "connection_id").unwrap_or_default(),
            connection_name: row.try_get("", "connection_name").ok(),
            organization_name: row.try_get("", "organization_name").ok(),
            fetched_at: row.try_get("", "fetched_at").unwrap_or_default(),
            is_posted: row.try_get::<bool>("", "is_posted").unwrap_or(false),
        })
        .collect();

    Ok(WbAdvertDailyListResult { items, total })
}

/// Максимум документов в одном CSV-отчёте (без тихой обрезки).
pub const A026_REPORT_MAX_DOCUMENTS: usize = 2000;

/// Верхняя граница строк позиций в отчёте (после фильтра по позиции), если фильтр пустой — по SUM(lines_count).
pub const A026_REPORT_MAX_LINE_ROWS: i64 = 200_000;

#[derive(Debug, Clone, Default)]
pub struct WbAdvertDailyReportQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub connection_id: Option<String>,
    pub search_query: Option<String>,
}

fn build_report_where_clause(query: &WbAdvertDailyReportQuery) -> String {
    let mut conditions = vec!["d.is_deleted = 0".to_string()];

    if let Some(ref date_from) = query.date_from {
        if !date_from.is_empty() {
            conditions.push(format!("d.document_date >= '{}'", date_from));
        }
    }
    if let Some(ref date_to) = query.date_to {
        if !date_to.is_empty() {
            conditions.push(format!("d.document_date <= '{}'", date_to));
        }
    }
    if let Some(ref connection_id) = query.connection_id {
        if !connection_id.is_empty() {
            conditions.push(format!("d.connection_id = '{}'", connection_id));
        }
    }
    if let Some(ref search) = query.search_query {
        if !search.is_empty() {
            let escaped = search.replace('\'', "''");
            conditions.push(format!(
                "(d.document_no LIKE '%{0}%' OR CAST(d.advert_id AS TEXT) LIKE '%{0}%' OR c.description LIKE '%{0}%' OR o.description LIKE '%{0}%')",
                escaped
            ));
        }
    }

    conditions.join(" AND ")
}

/// Число документов и сумма `lines_count` по тем же фильтрам, что у списка.
pub async fn report_preflight(query: &WbAdvertDailyReportQuery) -> Result<(usize, i64)> {
    let db = get_connection();
    let where_clause = build_report_where_clause(query);
    let sql = format!(
        "SELECT COUNT(*) as cnt, COALESCE(SUM(d.lines_count), 0) as line_sum
         FROM a026_wb_advert_daily d
         LEFT JOIN a006_connection_mp c ON c.id = d.connection_id
         LEFT JOIN a002_organization o ON o.id = d.organization_id
         WHERE {}",
        where_clause
    );

    let row = db
        .query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            sql,
        ))
        .await?;

    let total = row
        .as_ref()
        .and_then(|r| r.try_get::<i64>("", "cnt").ok())
        .unwrap_or(0) as usize;
    let line_sum = row
        .as_ref()
        .and_then(|r| r.try_get::<i64>("", "line_sum").ok())
        .unwrap_or(0);

    Ok((total, line_sum))
}

/// Все документы по фильтру отчёта, порядок: дата документа, номер (без пагинации).
pub async fn list_documents_for_report(
    query: &WbAdvertDailyReportQuery,
) -> Result<Vec<WbAdvertDaily>> {
    let db = get_connection();
    let where_clause = build_report_where_clause(query);
    let ids_sql = format!(
        "SELECT d.id
         FROM a026_wb_advert_daily d
         LEFT JOIN a006_connection_mp c ON c.id = d.connection_id
         LEFT JOIN a002_organization o ON o.id = d.organization_id
         WHERE {}
         ORDER BY d.document_date ASC, d.document_no ASC",
        where_clause
    );

    let rows = db
        .query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            ids_sql,
        ))
        .await?;

    let ids: Vec<String> = rows
        .into_iter()
        .filter_map(|row| row.try_get("", "id").ok())
        .collect();

    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let models = Entity::find()
        .filter(Column::Id.is_in(ids.clone()))
        .all(db)
        .await?;

    let mut by_id: HashMap<String, Model> = models.into_iter().map(|m| (m.id.clone(), m)).collect();
    let mut ordered = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(m) = by_id.remove(&id) {
            ordered.push(WbAdvertDaily::from(m));
        }
    }

    Ok(ordered)
}

pub async fn list_by_advert_id(connection_id: &str, advert_id: i64) -> Result<Vec<WbAdvertDaily>> {
    let db = get_connection();
    let models = Entity::find()
        .filter(Column::ConnectionId.eq(connection_id))
        .filter(Column::AdvertId.eq(advert_id))
        .filter(Column::IsDeleted.eq(false))
        .order_by_asc(Column::DocumentDate)
        .all(db)
        .await?;
    Ok(models.into_iter().map(Into::into).collect())
}

pub fn subtract_metrics(
    totals: &WbAdvertDailyMetrics,
    attributed: &WbAdvertDailyMetrics,
) -> WbAdvertDailyMetrics {
    WbAdvertDailyMetrics {
        views: (totals.views - attributed.views).max(0),
        clicks: (totals.clicks - attributed.clicks).max(0),
        atbs: (totals.atbs - attributed.atbs).max(0),
        orders: (totals.orders - attributed.orders).max(0),
        shks: (totals.shks - attributed.shks).max(0),
        canceled: (totals.canceled - attributed.canceled).max(0),
        sum: (totals.sum - attributed.sum).max(0.0),
        sum_price: (totals.sum_price - attributed.sum_price).max(0.0),
        ctr: 0.0,
        cpc: 0.0,
        cr: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::projection_registrator_ref;

    #[test]
    fn projection_registrator_ref_matches_posting_format() {
        assert_eq!(
            projection_registrator_ref("3f8f68d0-c0d3-4a4e-8b71-dafda55c91c2"),
            "a026:3f8f68d0-c0d3-4a4e-8b71-dafda55c91c2"
        );
    }
}
