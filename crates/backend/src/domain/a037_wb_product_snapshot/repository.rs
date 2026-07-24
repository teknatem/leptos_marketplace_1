use anyhow::Result;
use chrono::Utc;
use contracts::domain::a037_wb_product_snapshot::aggregate::{
    WbProductSnapshot, WbProductSnapshotHeader, WbProductSnapshotId, WbProductSnapshotLine,
    WbProductSnapshotSourceMeta,
};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use sea_orm::entity::prelude::*;
use sea_orm::{
    ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set, Statement,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a037_wb_product_snapshot")]
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
    pub total_stock_wb: i64,
    pub total_stock_mp: i64,
    pub total_balance_sum: f64,
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

impl From<Model> for WbProductSnapshot {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: false,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());
        let header: WbProductSnapshotHeader =
            serde_json::from_str(&m.header_json).unwrap_or(WbProductSnapshotHeader {
                document_no: m.document_no.clone(),
                snapshot_date: m.document_date.clone(),
                connection_id: m.connection_id.clone(),
                organization_id: m.organization_id.clone(),
                marketplace_id: m.marketplace_id.clone(),
            });
        let totals = serde_json::from_str(&m.totals_json).unwrap_or_default();
        let lines = serde_json::from_str(&m.lines_json).unwrap_or_default();
        let source_meta =
            serde_json::from_str(&m.source_meta_json).unwrap_or(WbProductSnapshotSourceMeta {
                source: "wb_product_snapshot".to_string(),
                fetched_at: m.fetched_at.clone(),
            });

        WbProductSnapshot {
            base: BaseAggregate::with_metadata(
                WbProductSnapshotId::new(uuid),
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
    documents: &[WbProductSnapshot],
) -> Result<usize> {
    let db = get_connection();
    let started_at = std::time::Instant::now();
    tracing::info!(
        "a037_wb_product_snapshot replace_for_period: connection={}, period={}..{}, documents={}",
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

    txn.commit().await?;
    tracing::info!(
        "a037_wb_product_snapshot replace_for_period: committed connection={}, inserted={}, elapsed_ms={}",
        connection_id,
        documents.len(),
        started_at.elapsed().as_millis()
    );
    Ok(documents.len())
}

async fn insert_with_conn<C: ConnectionTrait>(db: &C, document: &WbProductSnapshot) -> Result<()> {
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
        total_stock_wb: Set(document.totals.total_stock_wb),
        total_stock_mp: Set(document.totals.total_stock_mp),
        total_balance_sum: Set(document.totals.total_balance_sum),
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

pub async fn get_by_id(id: Uuid) -> Result<Option<WbProductSnapshot>> {
    let db = get_connection();
    let model = Entity::find_by_id(id.to_string()).one(db).await?;
    Ok(model.map(Into::into))
}

/// Предыдущий по дате снимок того же кабинета (строго раньше `date`).
/// Используется для сравнения рейтингов/оценок с прошлым получением данных.
pub async fn previous_before(connection_id: &str, date: &str) -> Result<Option<WbProductSnapshot>> {
    let db = get_connection();
    let model = Entity::find()
        .filter(Column::ConnectionId.eq(connection_id))
        .filter(Column::IsDeleted.eq(false))
        .filter(Column::DocumentDate.lt(date))
        .order_by_desc(Column::DocumentDate)
        .one(db)
        .await?;
    Ok(model.map(Into::into))
}

/// Последний снимок кабинета на дату `date` включительно (max `document_date <= date`).
/// Если `date == None` — просто самый свежий снимок. Возвращает None, если снимков нет.
/// «≤», а не точное совпадение: WB не отдаёт историю задним числом, дни могут пропадать.
pub async fn latest_on_or_before(
    connection_id: &str,
    date: Option<&str>,
) -> Result<Option<WbProductSnapshot>> {
    let db = get_connection();
    let mut q = Entity::find()
        .filter(Column::ConnectionId.eq(connection_id))
        .filter(Column::IsDeleted.eq(false));
    if let Some(d) = date.filter(|s| !s.is_empty()) {
        q = q.filter(Column::DocumentDate.lte(d));
    }
    let model = q.order_by_desc(Column::DocumentDate).one(db).await?;
    Ok(model.map(Into::into))
}

/// Различные `connection_id`, для которых есть хотя бы один снимок (не удалённый).
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

/// Загружает карту id → description для справочной таблицы (a006/a002).
async fn load_name_map<C: ConnectionTrait>(db: &C, table: &str) -> HashMap<String, String> {
    let sql = format!("SELECT id, description FROM {table}");
    let rows = match db
        .query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            sql,
        ))
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("load_name_map({table}) failed: {e}");
            return HashMap::new();
        }
    };
    rows.into_iter()
        .filter_map(|row| {
            let id: String = row.try_get("", "id").ok()?;
            let desc: String = row.try_get("", "description").unwrap_or_default();
            Some((id, desc))
        })
        .collect()
}

/// Плоская строка остатков на уровне товара (`nm_id`) для внешней выгрузки (Power BI).
#[derive(Debug, Clone)]
pub struct StockRow {
    /// Фактическая дата снимка, из которого взят остаток (`<=` запрошенной date).
    pub snapshot_date: String,
    pub connection_id: String,
    pub connection_name: Option<String>,
    pub organization_name: Option<String>,
    pub nm_id: i64,
    pub vendor_code: String,
    pub brand_name: String,
    pub subject_id: i64,
    pub subject_name: String,
    pub title: String,
    /// Остаток на складах WB, шт.
    pub stock_wb: i64,
    /// Остаток на складах продавца, шт.
    pub stock_mp: i64,
    /// Сумма остатков.
    pub stock_balance_sum: f64,
}

pub struct StockRowsResult {
    pub rows: Vec<StockRow>,
    pub total: usize,
}

/// Остатки на уровне товара: для каждого кабинета берётся снимок с `latest_on_or_before`,
/// его строки разворачиваются в `StockRow`. При `connection_id == None` — все кабинеты,
/// у которых есть снимки. Пагинация применяется к развёрнутым строкам.
pub async fn stock_rows(
    connection_id: Option<&str>,
    date: Option<&str>,
    limit: usize,
    offset: usize,
) -> Result<StockRowsResult> {
    let db = get_connection();

    let connections: Vec<String> = match connection_id.filter(|c| !c.is_empty()) {
        Some(c) => vec![c.to_string()],
        None => distinct_connection_ids().await?,
    };

    let conn_names = load_name_map(db, "a006_connection_mp").await;
    let org_names = load_name_map(db, "a002_organization").await;

    let mut rows: Vec<StockRow> = Vec::new();
    for cid in &connections {
        let Some(snap) = latest_on_or_before(cid, date).await? else {
            continue;
        };
        let snapshot_date = snap.header.snapshot_date.clone();
        let connection_name = conn_names.get(cid).cloned();
        let organization_name = org_names.get(&snap.header.organization_id).cloned();
        for line in &snap.lines {
            rows.push(StockRow {
                snapshot_date: snapshot_date.clone(),
                connection_id: cid.clone(),
                connection_name: connection_name.clone(),
                organization_name: organization_name.clone(),
                nm_id: line.nm_id,
                vendor_code: line.vendor_code.clone(),
                brand_name: line.brand_name.clone(),
                subject_id: line.subject_id,
                subject_name: line.subject_name.clone(),
                title: line.title.clone(),
                stock_wb: line.state.stock_wb,
                stock_mp: line.state.stock_mp,
                stock_balance_sum: line.state.stock_balance_sum,
            });
        }
    }

    rows.sort_by(|a, b| {
        a.connection_id
            .cmp(&b.connection_id)
            .then_with(|| a.nm_id.cmp(&b.nm_id))
    });

    let total = rows.len();
    let page = rows.into_iter().skip(offset).take(limit).collect();

    Ok(StockRowsResult { rows: page, total })
}

#[derive(Debug, Clone)]
pub struct WbProductSnapshotListQuery {
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
pub struct WbProductSnapshotListRow {
    pub id: String,
    pub document_no: String,
    pub document_date: String,
    pub lines_count: i32,
    pub total_stock_wb: i64,
    pub total_stock_mp: i64,
    pub total_balance_sum: f64,
    pub connection_id: String,
    pub connection_name: Option<String>,
    pub organization_name: Option<String>,
    pub fetched_at: String,
}

#[derive(Debug, Clone)]
pub struct WbProductSnapshotListResult {
    pub items: Vec<WbProductSnapshotListRow>,
    pub total: usize,
}

pub async fn list_sql(query: WbProductSnapshotListQuery) -> Result<WbProductSnapshotListResult> {
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
        "total_stock_wb" => "d.total_stock_wb",
        "total_stock_mp" => "d.total_stock_mp",
        "total_balance_sum" => "d.total_balance_sum",
        "connection_name" => "c.description",
        "organization_name" => "o.description",
        "fetched_at" => "d.fetched_at",
        _ => "d.document_date",
    };
    let sort_dir = if query.sort_desc { "DESC" } else { "ASC" };

    let count_sql = format!(
        "SELECT COUNT(*) as cnt
         FROM a037_wb_product_snapshot d
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
            d.lines_count,
            d.total_stock_wb,
            d.total_stock_mp,
            d.total_balance_sum,
            d.connection_id,
            c.description as connection_name,
            o.description as organization_name,
            d.fetched_at
         FROM a037_wb_product_snapshot d
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
        .map(|row| WbProductSnapshotListRow {
            id: row.try_get("", "id").unwrap_or_default(),
            document_no: row.try_get("", "document_no").unwrap_or_default(),
            document_date: row.try_get("", "document_date").unwrap_or_default(),
            lines_count: row.try_get("", "lines_count").unwrap_or(0),
            total_stock_wb: row.try_get("", "total_stock_wb").unwrap_or(0),
            total_stock_mp: row.try_get("", "total_stock_mp").unwrap_or(0),
            total_balance_sum: row.try_get("", "total_balance_sum").unwrap_or(0.0),
            connection_id: row.try_get("", "connection_id").unwrap_or_default(),
            connection_name: row.try_get("", "connection_name").ok(),
            organization_name: row.try_get("", "organization_name").ok(),
            fetched_at: row.try_get("", "fetched_at").unwrap_or_default(),
        })
        .collect();

    Ok(WbProductSnapshotListResult { items, total })
}

/// Одна точка временного ряда по товару (для графика динамики).
#[derive(Debug, Clone)]
pub struct WbProductSnapshotSeriesPoint {
    pub date: String,
    pub stock_wb: i64,
    pub stock_mp: i64,
    pub stock_balance_sum: f64,
    pub product_rating: f64,
    pub feedback_rating: f64,
}

/// Динамика одного товара (nm_id) по дням: сканирует документы периода и достаёт
/// строку нужного nm_id из lines_json каждого дня.
pub async fn series_for_nm(
    connection_id: &str,
    nm_id: i64,
    date_from: &str,
    date_to: &str,
) -> Result<Vec<WbProductSnapshotSeriesPoint>> {
    let db = get_connection();
    let models = Entity::find()
        .filter(Column::ConnectionId.eq(connection_id))
        .filter(Column::IsDeleted.eq(false))
        .filter(Column::DocumentDate.gte(date_from))
        .filter(Column::DocumentDate.lte(date_to))
        .all(db)
        .await?;

    let mut points: Vec<WbProductSnapshotSeriesPoint> = Vec::new();
    for m in models {
        let lines: Vec<WbProductSnapshotLine> =
            serde_json::from_str(&m.lines_json).unwrap_or_default();
        if let Some(line) = lines.into_iter().find(|l| l.nm_id == nm_id) {
            points.push(WbProductSnapshotSeriesPoint {
                date: m.document_date.clone(),
                stock_wb: line.state.stock_wb,
                stock_mp: line.state.stock_mp,
                stock_balance_sum: line.state.stock_balance_sum,
                product_rating: line.state.product_rating,
                feedback_rating: line.state.feedback_rating,
            });
        }
    }
    points.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(points)
}
