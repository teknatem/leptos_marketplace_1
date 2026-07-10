//! Журнал запусков плагинов и агрегированная статистика (наблюдаемость).
//!
//! Запись ведётся из [`super::service::invoke`] на каждый серверный вызов. Чтения —
//! сырым SQL (как в `shared/llm/admin_tools.rs`), классификация «здоровья» — в Rust.

use contracts::plugins::{
    PluginDataMode, PluginHealth, PluginModeStats, PluginRunBrief, PluginRunRecord,
    PluginRunSummary, PluginStats, StageCount,
};
use sea_orm::{DatabaseBackend, FromQueryResult, Statement, Value as DbValue};

fn db() -> &'static sea_orm::DatabaseConnection {
    crate::shared::data::db::get_connection()
}

fn window_arg(days: i64) -> String {
    format!("-{} days", days.max(1))
}

fn json_rows(sql: &str, values: Vec<DbValue>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Sqlite, sql, values)
}

/// Зафиксировать запуск плагина. Fire-and-forget: ошибки записи лог не валят вызов.
#[allow(clippy::too_many_arguments)]
pub async fn record(
    plugin_id: &str,
    code: &str,
    method: &str,
    duration_ms: i64,
    status: &str,
    error_stage: Option<&str>,
    row_count: Option<i64>,
    triggered_by: Option<&str>,
    data_mode: PluginDataMode,
) {
    use sea_orm::ConnectionTrait;

    let started_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let id = uuid::Uuid::new_v4().to_string();
    let stmt = json_rows(
        "INSERT INTO plugin_run \
           (id, plugin_id, code, method, started_at, duration_ms, status, error_stage, row_count, triggered_by, data_mode) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        vec![
            id.into(),
            plugin_id.into(),
            code.into(),
            method.into(),
            started_at.into(),
            duration_ms.into(),
            status.into(),
            error_stage.map(str::to_string).into(),
            row_count.into(),
            triggered_by.map(str::to_string).into(),
            match data_mode { PluginDataMode::Live => "live", PluginDataMode::Snapshot => "snapshot" }.into(),
        ],
    );
    if let Err(error) = db().execute(stmt).await {
        tracing::warn!("Failed to record plugin_run for {plugin_id}: {error}");
    }
}

fn classify(
    total: i64,
    errors: i64,
    timeouts: i64,
    avg_ms: i64,
    last_status: Option<&str>,
) -> PluginHealth {
    if total == 0 {
        return PluginHealth::NoData;
    }
    let rate = errors as f64 / total as f64;
    if rate > 0.20 || timeouts > 0 {
        PluginHealth::Crit
    } else if rate > 0.05 || avg_ms > 2000 || matches!(last_status, Some("error") | Some("timeout"))
    {
        PluginHealth::Warn
    } else {
        PluginHealth::Ok
    }
}

fn i64_at(row: &serde_json::Value, key: &str) -> i64 {
    row.get(key)
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0)
}

fn str_at(row: &serde_json::Value, key: &str) -> Option<String> {
    row.get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

/// Полная статистика по плагину за окно `days` дней (сводка + последние запуски).
pub async fn stats(plugin_id: &str, days: i64, recent_limit: i64) -> anyhow::Result<PluginStats> {
    let summary = summary(plugin_id, days).await?;

    let recent_rows = serde_json::Value::find_by_statement(json_rows(
        "SELECT id, method, started_at, duration_ms, status, error_stage, row_count, data_mode \
         FROM plugin_run WHERE plugin_id = ? ORDER BY started_at DESC LIMIT ?",
        vec![plugin_id.into(), recent_limit.max(1).into()],
    ))
    .all(db())
    .await?;
    let recent = recent_rows
        .into_iter()
        .filter_map(|row| serde_json::from_value::<PluginRunRecord>(row).ok())
        .collect();

    Ok(PluginStats { summary, recent })
}

/// Агрегированная сводка по плагину за окно `days` дней.
pub async fn summary(plugin_id: &str, days: i64) -> anyhow::Result<PluginRunSummary> {
    let window = window_arg(days);

    let agg = serde_json::Value::find_by_statement(json_rows(
        "SELECT \
           COUNT(*) AS total, \
           SUM(CASE WHEN status != 'ok' THEN 1 ELSE 0 END) AS errors, \
           SUM(CASE WHEN status = 'timeout' THEN 1 ELSE 0 END) AS timeouts, \
           CAST(COALESCE(AVG(duration_ms), 0) AS INTEGER) AS avg_ms, \
           COALESCE(MAX(duration_ms), 0) AS max_ms \
         FROM plugin_run \
         WHERE plugin_id = ? AND datetime(started_at) >= datetime('now', ?)",
        vec![plugin_id.into(), window.clone().into()],
    ))
    .all(db())
    .await?;
    let agg = agg.into_iter().next().unwrap_or(serde_json::Value::Null);

    let total = i64_at(&agg, "total");
    let errors = i64_at(&agg, "errors");
    let timeouts = i64_at(&agg, "timeouts");
    let avg_ms = i64_at(&agg, "avg_ms");
    let max_ms = i64_at(&agg, "max_ms");

    let last = serde_json::Value::find_by_statement(json_rows(
        "SELECT started_at AS last_run_at, status AS last_status \
         FROM plugin_run WHERE plugin_id = ? ORDER BY started_at DESC LIMIT 1",
        vec![plugin_id.into()],
    ))
    .all(db())
    .await?;
    let last = last.into_iter().next().unwrap_or(serde_json::Value::Null);
    let last_run_at = str_at(&last, "last_run_at");
    let last_status = str_at(&last, "last_status");

    let stage_rows = serde_json::Value::find_by_statement(json_rows(
        "SELECT COALESCE(error_stage, 'unknown') AS stage, COUNT(*) AS count \
         FROM plugin_run \
         WHERE plugin_id = ? AND status != 'ok' AND datetime(started_at) >= datetime('now', ?) \
         GROUP BY error_stage ORDER BY count DESC",
        vec![plugin_id.into(), window.into()],
    ))
    .all(db())
    .await?;
    let by_stage = stage_rows
        .into_iter()
        .filter_map(|row| serde_json::from_value::<StageCount>(row).ok())
        .collect();

    let mode_rows = serde_json::Value::find_by_statement(json_rows(
        "SELECT data_mode, COUNT(*) AS total, \
           SUM(CASE WHEN status != 'ok' THEN 1 ELSE 0 END) AS errors, \
           CAST(COALESCE(AVG(duration_ms), 0) AS INTEGER) AS avg_ms \
         FROM plugin_run \
         WHERE plugin_id = ? AND datetime(started_at) >= datetime('now', ?) \
         GROUP BY data_mode ORDER BY data_mode",
        vec![plugin_id.into(), window_arg(days).into()],
    ))
    .all(db())
    .await?;
    let by_data_mode = mode_rows
        .into_iter()
        .filter_map(|row| serde_json::from_value::<PluginModeStats>(row).ok())
        .collect();

    let error_rate = if total > 0 {
        errors as f64 / total as f64
    } else {
        0.0
    };
    let health = classify(total, errors, timeouts, avg_ms, last_status.as_deref());

    Ok(PluginRunSummary {
        days,
        total,
        errors,
        timeouts,
        error_rate,
        avg_ms,
        max_ms,
        by_stage,
        by_data_mode,
        last_run_at,
        last_status,
        health,
    })
}

/// Краткие сводки по всем плагинам за окно `days` дней (для реестра).
pub async fn summary_all(days: i64) -> anyhow::Result<Vec<PluginRunBrief>> {
    let window = window_arg(days);
    let rows = serde_json::Value::find_by_statement(json_rows(
        "SELECT r.plugin_id AS plugin_id, \
           COUNT(*) AS runs, \
           SUM(CASE WHEN r.status != 'ok' THEN 1 ELSE 0 END) AS errors, \
           SUM(CASE WHEN r.status = 'timeout' THEN 1 ELSE 0 END) AS timeouts, \
           CAST(COALESCE(AVG(r.duration_ms), 0) AS INTEGER) AS avg_ms, \
           MAX(r.started_at) AS last_run_at, \
           (SELECT status FROM plugin_run r2 WHERE r2.plugin_id = r.plugin_id \
              ORDER BY started_at DESC LIMIT 1) AS last_status \
         FROM plugin_run r \
         WHERE datetime(r.started_at) >= datetime('now', ?) \
         GROUP BY r.plugin_id",
        vec![window.into()],
    ))
    .all(db())
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let runs = i64_at(&row, "runs");
            let errors = i64_at(&row, "errors");
            let timeouts = i64_at(&row, "timeouts");
            let avg_ms = i64_at(&row, "avg_ms");
            let last_status = str_at(&row, "last_status");
            PluginRunBrief {
                plugin_id: str_at(&row, "plugin_id").unwrap_or_default(),
                runs,
                error_rate: if runs > 0 {
                    errors as f64 / runs as f64
                } else {
                    0.0
                },
                last_run_at: str_at(&row, "last_run_at"),
                health: classify(runs, errors, timeouts, avg_ms, last_status.as_deref()),
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_thresholds() {
        assert_eq!(classify(0, 0, 0, 0, None), PluginHealth::NoData);
        assert_eq!(classify(100, 0, 0, 100, Some("ok")), PluginHealth::Ok);
        assert_eq!(classify(100, 10, 0, 100, Some("ok")), PluginHealth::Warn);
        assert_eq!(classify(100, 0, 0, 3000, Some("ok")), PluginHealth::Warn);
        assert_eq!(classify(100, 30, 0, 100, Some("error")), PluginHealth::Crit);
        assert_eq!(
            classify(100, 1, 1, 100, Some("timeout")),
            PluginHealth::Crit
        );
    }
}
