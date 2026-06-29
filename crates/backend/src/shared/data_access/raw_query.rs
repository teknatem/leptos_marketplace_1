use super::catalog::{DataSourceKind, DataSourceRef};
use super::sql_guard::{inspect_read_query, wrap_limited_sql};
use super::TabularResult;
use crate::shared::data::db::get_connection;
use sea_orm::{DatabaseBackend, FromQueryResult, Statement, Value as DbValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{Duration, Instant};

const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 200;
const QUERY_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlAccessProfile {
    Analytics,
    KnowledgeBase,
    General,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawQueryRequest {
    pub sql: String,
    #[serde(default)]
    pub params: Vec<Value>,
    #[serde(default)]
    pub limit: Option<usize>,
}

fn is_secret_table(table: &str) -> bool {
    matches!(table, "a001_connection_1c" | "a006_connection_mp")
}

fn is_knowledge_table(table: &str) -> bool {
    table.starts_with("a017_") || table.starts_with("a018_") || table.starts_with("a019_")
}

fn is_analytics_table(table: &str) -> bool {
    if is_secret_table(table) || is_knowledge_table(table) {
        return false;
    }
    (table.starts_with('a')
        && table
            .get(1..4)
            .is_some_and(|part| part.chars().all(|c| c.is_ascii_digit())))
        || table.starts_with('p')
        || table == "sys_general_ledger"
}

fn profile_allows(profile: SqlAccessProfile, table: &str) -> bool {
    if is_secret_table(table) || (table.starts_with("sys_") && table != "sys_general_ledger") {
        return false;
    }
    match profile {
        SqlAccessProfile::Analytics => is_analytics_table(table),
        SqlAccessProfile::KnowledgeBase => is_knowledge_table(table),
        SqlAccessProfile::General => is_analytics_table(table) || is_knowledge_table(table),
    }
}

pub fn enforce_access_profile(profile: SqlAccessProfile, tables: &[String]) -> Result<(), String> {
    let blocked: Vec<&str> = tables
        .iter()
        .map(String::as_str)
        .filter(|table| !profile_allows(profile, table))
        .collect();
    if blocked.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Raw SQL access is not allowed for table(s): {}. Use a safe data schema or a dedicated tool.",
            blocked.join(", ")
        ))
    }
}

fn json_param_to_db(value: Value) -> Result<DbValue, String> {
    match value {
        Value::Null => Ok(DbValue::String(None)),
        Value::Bool(value) => Ok(DbValue::Bool(Some(value))),
        Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                Ok(DbValue::BigInt(Some(value)))
            } else if let Some(value) = value.as_u64() {
                Ok(DbValue::BigUnsigned(Some(value)))
            } else if let Some(value) = value.as_f64() {
                Ok(DbValue::Double(Some(value)))
            } else {
                Err("Unsupported numeric SQL parameter".to_string())
            }
        }
        Value::String(value) => Ok(DbValue::String(Some(Box::new(value)))),
        Value::Array(_) | Value::Object(_) => {
            Err("SQL parameters must be scalar JSON values".to_string())
        }
    }
}

pub async fn execute_raw_query(
    request: RawQueryRequest,
    profile: SqlAccessProfile,
) -> Result<TabularResult, String> {
    let started = Instant::now();
    let query_info = inspect_read_query(&request.sql).map_err(|error| {
        tracing::warn!(blocked_reason = %error, "raw SQL rejected");
        error
    })?;
    enforce_access_profile(profile, &query_info.tables).map_err(|error| {
        tracing::warn!(blocked_reason = %error, "raw SQL rejected");
        error
    })?;

    let limit = request.limit.unwrap_or(DEFAULT_LIMIT);
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err(format!("limit must be between 1 and {MAX_LIMIT}"));
    }
    let params = request
        .params
        .into_iter()
        .map(json_param_to_db)
        .collect::<Result<Vec<_>, _>>()?;
    let sql = wrap_limited_sql(&request.sql, limit + 1, "llm_limited_result");
    let statement = Statement::from_sql_and_values(DatabaseBackend::Sqlite, sql, params);
    let rows_future = Value::find_by_statement(statement).all(get_connection());
    let mut rows = tokio::time::timeout(QUERY_TIMEOUT, rows_future)
        .await
        .map_err(|_| "Raw SQL query timed out after 10 seconds".to_string())?
        .map_err(|error| format!("SQL execution error: {error}"))?;
    let truncated = rows.len() > limit;
    if truncated {
        rows.truncate(limit);
    }
    let columns = rows
        .first()
        .and_then(Value::as_object)
        .map(|row| row.keys().cloned().collect())
        .unwrap_or_default();
    let result = TabularResult {
        source: DataSourceRef {
            kind: DataSourceKind::Raw,
            id: "raw_sql".to_string(),
        },
        row_count: rows.len(),
        rows,
        columns,
        truncated,
    };
    tracing::info!(
        source_kind = "raw",
        source_id = "raw_sql",
        elapsed_ms = started.elapsed().as_millis(),
        row_count = result.row_count,
        truncated = result.truncated,
        "semantic data query completed"
    );
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analytics_profile_blocks_secret_and_system_tables() {
        assert!(enforce_access_profile(
            SqlAccessProfile::Analytics,
            &["a006_connection_mp".to_string()]
        )
        .is_err());
        assert!(enforce_access_profile(
            SqlAccessProfile::Analytics,
            &["sys_task_runs".to_string()]
        )
        .is_err());
        assert!(enforce_access_profile(
            SqlAccessProfile::Analytics,
            &[
                "p904_sales_data".to_string(),
                "sys_general_ledger".to_string()
            ]
        )
        .is_ok());
    }

    #[test]
    fn knowledge_profile_is_narrow() {
        assert!(enforce_access_profile(
            SqlAccessProfile::KnowledgeBase,
            &["a018_llm_chat_message".to_string()]
        )
        .is_ok());
        assert!(enforce_access_profile(
            SqlAccessProfile::KnowledgeBase,
            &["p904_sales_data".to_string()]
        )
        .is_err());
    }
}
