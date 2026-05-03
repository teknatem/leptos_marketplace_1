use crate::shared::data::db::get_connection;
use chrono::Utc;
use contracts::system::tasks::aggregate::ScheduledTaskId;
use contracts::system::tasks::runs::TaskRun;
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, FromQueryResult, QueryFilter, Set, Statement};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sys_task_runs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub task_id: String,
    pub session_id: String,
    pub triggered_by: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    pub duration_ms: Option<i64>,
    pub status: String,
    pub total_processed: Option<i64>,
    pub total_inserted: Option<i64>,
    pub total_updated: Option<i64>,
    pub total_errors: Option<i64>,
    pub log_file_path: Option<String>,
    pub error_message: Option<String>,
    pub http_request_count: Option<i64>,
    pub http_bytes_sent: Option<i64>,
    pub http_bytes_received: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for TaskRun {
    fn from(m: Model) -> Self {
        Self {
            id: m.id,
            task_id: m.task_id,
            task_code: None,
            task_description: None,
            session_id: m.session_id,
            triggered_by: m.triggered_by,
            started_at: m.started_at,
            finished_at: m.finished_at,
            duration_ms: m.duration_ms,
            status: m.status,
            total_processed: m.total_processed,
            total_inserted: m.total_inserted,
            total_updated: m.total_updated,
            total_errors: m.total_errors,
            log_file_path: m.log_file_path,
            error_message: m.error_message,
            http_request_count: m.http_request_count,
            http_bytes_sent: m.http_bytes_sent,
            http_bytes_received: m.http_bytes_received,
        }
    }
}

#[derive(Debug, Clone, FromQueryResult)]
struct TaskRunJoinRow {
    id: String,
    task_id: String,
    session_id: String,
    triggered_by: String,
    started_at: chrono::DateTime<chrono::Utc>,
    finished_at: Option<chrono::DateTime<chrono::Utc>>,
    duration_ms: Option<i64>,
    status: String,
    total_processed: Option<i64>,
    total_inserted: Option<i64>,
    total_updated: Option<i64>,
    total_errors: Option<i64>,
    log_file_path: Option<String>,
    error_message: Option<String>,
    http_request_count: Option<i64>,
    http_bytes_sent: Option<i64>,
    http_bytes_received: Option<i64>,
    task_code: Option<String>,
    task_description: Option<String>,
}

fn join_row_to_task_run(row: TaskRunJoinRow) -> TaskRun {
    TaskRun {
        id: row.id,
        task_id: row.task_id,
        task_code: row.task_code,
        task_description: row.task_description,
        session_id: row.session_id,
        triggered_by: row.triggered_by,
        started_at: row.started_at,
        finished_at: row.finished_at,
        duration_ms: row.duration_ms,
        status: row.status,
        total_processed: row.total_processed,
        total_inserted: row.total_inserted,
        total_updated: row.total_updated,
        total_errors: row.total_errors,
        log_file_path: row.log_file_path,
        error_message: row.error_message,
        http_request_count: row.http_request_count,
        http_bytes_sent: row.http_bytes_sent,
        http_bytes_received: row.http_bytes_received,
    }
}

const RUN_JOIN_SELECT: &str = r#"SELECT
  r.id AS id,
  r.task_id AS task_id,
  r.session_id AS session_id,
  r.triggered_by AS triggered_by,
  r.started_at AS started_at,
  r.finished_at AS finished_at,
  r.duration_ms AS duration_ms,
  r.status AS status,
  r.total_processed AS total_processed,
  r.total_inserted AS total_inserted,
  r.total_updated AS total_updated,
  r.total_errors AS total_errors,
  r.log_file_path AS log_file_path,
  r.error_message AS error_message,
  r.http_request_count AS http_request_count,
  r.http_bytes_sent AS http_bytes_sent,
  r.http_bytes_received AS http_bytes_received,
  t.code AS task_code,
  t.description AS task_description
FROM sys_task_runs r
LEFT JOIN sys_tasks t ON t.id = r.task_id"#;

pub async fn create_run(
    task_id: ScheduledTaskId,
    session_id: String,
    triggered_by: String,
    log_file_path: Option<String>,
) -> Result<(), DbErr> {
    let db = get_connection();
    let active = ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        task_id: Set(task_id.0.to_string()),
        session_id: Set(session_id),
        triggered_by: Set(triggered_by),
        started_at: Set(Utc::now()),
        finished_at: Set(None),
        duration_ms: Set(None),
        status: Set("Running".to_string()),
        total_processed: Set(None),
        total_inserted: Set(None),
        total_updated: Set(None),
        total_errors: Set(None),
        log_file_path: Set(log_file_path),
        error_message: Set(None),
        http_request_count: Set(None),
        http_bytes_sent: Set(None),
        http_bytes_received: Set(None),
    };
    active.insert(db).await?;
    Ok(())
}

pub async fn finish_run(
    session_id: &str,
    status: &str,
    total_processed: Option<i64>,
    total_inserted: Option<i64>,
    total_updated: Option<i64>,
    total_errors: Option<i64>,
    http_request_count: Option<i64>,
    http_bytes_sent: Option<i64>,
    http_bytes_received: Option<i64>,
    error_message: Option<String>,
) -> Result<(), DbErr> {
    let db = get_connection();
    let model = Entity::find()
        .filter(Column::SessionId.eq(session_id))
        .one(db)
        .await?;

    if let Some(m) = model {
        let now = Utc::now();
        let duration_ms = (now - m.started_at).num_milliseconds();
        let mut active: ActiveModel = m.into();
        active.finished_at = Set(Some(now));
        active.duration_ms = Set(Some(duration_ms));
        active.status = Set(status.to_string());
        active.total_processed = Set(total_processed);
        active.total_inserted = Set(total_inserted);
        active.total_updated = Set(total_updated);
        active.total_errors = Set(total_errors);
        active.http_request_count = Set(http_request_count);
        active.http_bytes_sent = Set(http_bytes_sent);
        active.http_bytes_received = Set(http_bytes_received);
        active.error_message = Set(error_message);
        active.update(db).await?;
    }
    Ok(())
}

pub async fn find_by_session_id(session_id: &str) -> Result<Option<Model>, DbErr> {
    let db = get_connection();
    Entity::find()
        .filter(Column::SessionId.eq(session_id))
        .one(db)
        .await
}

pub async fn find_all_running_models() -> Result<Vec<Model>, DbErr> {
    let db = get_connection();
    Entity::find()
        .filter(Column::Status.eq("Running"))
        .all(db)
        .await
}

pub async fn find_running_for_task(task_id: &str) -> Result<Option<Model>, DbErr> {
    let db = get_connection();
    Entity::find()
        .filter(Column::TaskId.eq(task_id))
        .filter(Column::Status.eq("Running"))
        .one(db)
        .await
}

pub async fn find_running_for_task_enriched(task_id: &str) -> Result<Option<TaskRun>, DbErr> {
    let db = get_connection();
    let sql = format!(
        "{} WHERE r.task_id = ? AND r.status = 'Running' ORDER BY r.started_at DESC LIMIT 1",
        RUN_JOIN_SELECT
    );
    let stmt =
        Statement::from_sql_and_values(sea_orm::DatabaseBackend::Sqlite, sql, vec![task_id.into()]);
    let row = db.query_one(stmt).await?;
    Ok(row
        .and_then(|r| TaskRunJoinRow::from_query_result(&r, "").ok())
        .map(join_row_to_task_run))
}

pub async fn list_active_enriched() -> Result<Vec<TaskRun>, DbErr> {
    let db = get_connection();
    let sql = format!(
        "{} WHERE r.status = 'Running' ORDER BY r.started_at DESC",
        RUN_JOIN_SELECT
    );
    let stmt = Statement::from_sql_and_values(sea_orm::DatabaseBackend::Sqlite, sql, vec![]);
    let rows = db.query_all(stmt).await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| TaskRunJoinRow::from_query_result(&r, "").ok())
        .map(join_row_to_task_run)
        .collect())
}

pub async fn list_recent_enriched(limit: u64) -> Result<Vec<TaskRun>, DbErr> {
    let db = get_connection();
    let sql = format!("{} ORDER BY r.started_at DESC LIMIT ?", RUN_JOIN_SELECT);
    let stmt = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Sqlite,
        sql,
        vec![sea_orm::Value::BigUnsigned(Some(limit))],
    );
    let rows = db.query_all(stmt).await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| TaskRunJoinRow::from_query_result(&r, "").ok())
        .map(join_row_to_task_run)
        .collect())
}

pub async fn list_for_task_enriched(task_id: &str, limit: u64) -> Result<Vec<TaskRun>, DbErr> {
    let db = get_connection();
    let sql = format!(
        "{} WHERE r.task_id = ? ORDER BY r.started_at DESC LIMIT ?",
        RUN_JOIN_SELECT
    );
    let stmt = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Sqlite,
        sql,
        vec![task_id.into(), sea_orm::Value::BigUnsigned(Some(limit))],
    );
    let rows = db.query_all(stmt).await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| TaskRunJoinRow::from_query_result(&r, "").ok())
        .map(join_row_to_task_run)
        .collect())
}

pub async fn list_for_task(task_id: &str, limit: u64) -> Result<Vec<TaskRun>, DbErr> {
    list_for_task_enriched(task_id, limit).await
}

pub async fn list_recent(limit: u64) -> Result<Vec<TaskRun>, DbErr> {
    list_recent_enriched(limit).await
}
