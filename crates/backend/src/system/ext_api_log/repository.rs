use contracts::system::ext_api_log::{ExtApiLogRow, ExtApiSummaryRow};
use sea_orm::entity::prelude::*;
use sea_orm::{FromQueryResult, QueryOrder, QuerySelect, Set, Statement};

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sys_ext_api_log")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// UTC ISO8601 (RFC3339 с суффиксом `Z`).
    pub ts: String,
    pub method: String,
    pub route: String,
    pub path: String,
    pub query: Option<String>,
    pub status: i32,
    pub duration_ms: i64,
    pub bytes_out: i64,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub client_id: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

fn row_from_model(m: Model) -> ExtApiLogRow {
    ExtApiLogRow {
        id: m.id,
        ts: m.ts,
        method: m.method,
        route: m.route,
        path: m.path,
        query: m.query,
        status: m.status,
        duration_ms: m.duration_ms,
        bytes_out: m.bytes_out,
        client_ip: m.client_ip,
        user_agent: m.user_agent,
        client_id: m.client_id,
    }
}

pub async fn insert(m: Model) -> Result<(), DbErr> {
    ActiveModel {
        id: Set(m.id),
        ts: Set(m.ts),
        method: Set(m.method),
        route: Set(m.route),
        path: Set(m.path),
        query: Set(m.query),
        status: Set(m.status),
        duration_ms: Set(m.duration_ms),
        bytes_out: Set(m.bytes_out),
        client_ip: Set(m.client_ip),
        user_agent: Set(m.user_agent),
        client_id: Set(m.client_id),
    }
    .insert(conn())
    .await?;
    Ok(())
}

pub async fn list_recent(limit: u64) -> Result<Vec<ExtApiLogRow>, DbErr> {
    let models = Entity::find()
        .order_by_desc(Column::Ts)
        .limit(limit)
        .all(conn())
        .await?;
    Ok(models.into_iter().map(row_from_model).collect())
}

/// Событие для бакетизации: запрос — точечное событие, интервал не нужен.
#[derive(Debug, Clone, FromQueryResult)]
pub struct ExtApiEventRow {
    pub ts: String,
    pub status: i32,
    pub duration_ms: i64,
    pub bytes_out: i64,
}

pub async fn query_period(
    date_from_utc: &str,
    date_to_utc: &str,
) -> Result<Vec<ExtApiEventRow>, DbErr> {
    let stmt = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Sqlite,
        "SELECT ts, status, duration_ms, bytes_out \
         FROM sys_ext_api_log \
         WHERE datetime(ts) >= datetime(?) AND datetime(ts) < datetime(?) \
         ORDER BY ts",
        vec![date_from_utc.into(), date_to_utc.into()],
    );
    let rows = conn().query_all(stmt).await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| ExtApiEventRow::from_query_result(&r, "").ok())
        .collect())
}

#[derive(Debug, Clone, FromQueryResult)]
struct SummaryRow {
    key: String,
    req_count: i64,
    bytes_out: i64,
    error_count: i64,
    avg_ms: f64,
}

impl From<SummaryRow> for ExtApiSummaryRow {
    fn from(r: SummaryRow) -> Self {
        Self {
            key: r.key,
            req_count: r.req_count,
            bytes_out: r.bytes_out,
            error_count: r.error_count,
            avg_ms: r.avg_ms,
        }
    }
}

async fn summary_grouped(
    key_expr: &str,
    date_from_utc: &str,
    date_to_utc: &str,
) -> Result<Vec<ExtApiSummaryRow>, DbErr> {
    let sql = format!(
        "SELECT {key_expr} AS key, \
                COUNT(*) AS req_count, \
                CAST(SUM(bytes_out) AS INTEGER) AS bytes_out, \
                CAST(SUM(CASE WHEN status >= 400 THEN 1 ELSE 0 END) AS INTEGER) AS error_count, \
                CAST(AVG(duration_ms) AS REAL) AS avg_ms \
         FROM sys_ext_api_log \
         WHERE datetime(ts) >= datetime(?) AND datetime(ts) < datetime(?) \
         GROUP BY key \
         ORDER BY req_count DESC"
    );
    let stmt = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Sqlite,
        sql,
        vec![date_from_utc.into(), date_to_utc.into()],
    );
    let rows = conn().query_all(stmt).await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| SummaryRow::from_query_result(&r, "").ok())
        .map(Into::into)
        .collect())
}

pub async fn summary_by_route(
    date_from_utc: &str,
    date_to_utc: &str,
) -> Result<Vec<ExtApiSummaryRow>, DbErr> {
    summary_grouped("route", date_from_utc, date_to_utc).await
}

/// Потребитель: пока ключ один на всех, поэтому опознаём по UA/IP.
/// `client_id` заполнится, когда появится многоключевость.
pub async fn summary_by_client(
    date_from_utc: &str,
    date_to_utc: &str,
) -> Result<Vec<ExtApiSummaryRow>, DbErr> {
    summary_grouped(
        "COALESCE(client_id, user_agent, client_ip, 'unknown')",
        date_from_utc,
        date_to_utc,
    )
    .await
}

/// Удаляет строки старше `days` суток. Возвращает количество удалённых.
pub async fn prune_older_than(days: i64) -> Result<u64, DbErr> {
    let res = conn()
        .execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            "DELETE FROM sys_ext_api_log WHERE datetime(ts) < datetime('now', ?)",
            vec![format!("-{days} days").into()],
        ))
        .await?;
    Ok(res.rows_affected())
}
