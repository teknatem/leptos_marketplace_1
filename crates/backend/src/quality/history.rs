use contracts::quality::{CheckDetails, QualityCheckRunSummary};
use sea_orm::{ConnectionTrait, Statement};
use std::collections::HashMap;

fn parse_time(value: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now())
}

pub async fn start_run(
    check_id: &str,
    digest: &str,
    input: &serde_json::Value,
    trigger: &str,
) -> anyhow::Result<(String, chrono::DateTime<chrono::Utc>)> {
    let id = uuid::Uuid::new_v4().to_string();
    let started_at = chrono::Utc::now();
    let conn = crate::shared::data::db::get_connection();
    conn.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Sqlite,
        "INSERT INTO sys_quality_check_runs \
         (id,check_id,definition_digest,input_json,trigger,status,started_at) \
         VALUES (?1,?2,?3,?4,?5,'running',?6)",
        [
            id.clone().into(),
            check_id.into(),
            digest.into(),
            serde_json::to_string(input)?.into(),
            trigger.into(),
            started_at.to_rfc3339().into(),
        ],
    ))
    .await?;
    Ok((id, started_at))
}

pub async fn finish_success(
    id: &str,
    started_at: chrono::DateTime<chrono::Utc>,
    details: &CheckDetails,
) -> anyhow::Result<()> {
    let finished_at = chrono::Utc::now();
    let duration_ms = (finished_at - started_at).num_milliseconds().max(0);
    let conn = crate::shared::data::db::get_connection();
    conn.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Sqlite,
        "UPDATE sys_quality_check_runs SET status='completed',finished_at=?1,duration_ms=?2,\
         population_total=?3,violations_total=?4,details_json=?5,error=NULL WHERE id=?6",
        [
            finished_at.to_rfc3339().into(),
            duration_ms.into(),
            details.result.population_total.into(),
            details.result.violations_total.into(),
            serde_json::to_string(details)?.into(),
            id.into(),
        ],
    ))
    .await?;
    Ok(())
}

pub async fn finish_failure(
    id: &str,
    started_at: chrono::DateTime<chrono::Utc>,
    error: &str,
) -> anyhow::Result<()> {
    let finished_at = chrono::Utc::now();
    let duration_ms = (finished_at - started_at).num_milliseconds().max(0);
    crate::shared::data::db::get_connection()
        .execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            "UPDATE sys_quality_check_runs SET status='failed',finished_at=?1,duration_ms=?2,error=?3 WHERE id=?4",
            [finished_at.to_rfc3339().into(), duration_ms.into(), error.into(), id.into()],
        ))
        .await?;
    Ok(())
}

pub async fn latest_success_details(check_id: &str) -> anyhow::Result<Option<CheckDetails>> {
    latest_success_details_query(check_id, None).await
}

pub async fn latest_success_details_for_digest(
    check_id: &str,
    digest: &str,
) -> anyhow::Result<Option<CheckDetails>> {
    latest_success_details_query(check_id, Some(digest)).await
}

async fn latest_success_details_query(
    check_id: &str,
    digest: Option<&str>,
) -> anyhow::Result<Option<CheckDetails>> {
    let sql = if digest.is_some() {
        "SELECT details_json FROM sys_quality_check_runs \
         WHERE check_id=?1 AND definition_digest=?2 AND status='completed' AND details_json IS NOT NULL \
         ORDER BY started_at DESC LIMIT 1"
    } else {
        "SELECT details_json FROM sys_quality_check_runs \
         WHERE check_id=?1 AND status='completed' AND details_json IS NOT NULL \
         ORDER BY started_at DESC LIMIT 1"
    };
    let values = match digest {
        Some(digest) => vec![check_id.into(), digest.into()],
        None => vec![check_id.into()],
    };
    let rows = crate::shared::data::db::get_connection()
        .query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            sql,
            values,
        ))
        .await?;
    let Some(row) = rows.first() else {
        return Ok(None);
    };
    let raw: String = row.try_get("", "details_json")?;
    Ok(Some(serde_json::from_str(&raw)?))
}

pub async fn list_runs(check_id: &str, limit: i64) -> anyhow::Result<Vec<QualityCheckRunSummary>> {
    let rows = crate::shared::data::db::get_connection()
        .query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT id,check_id,definition_digest,trigger,status,started_at,finished_at,duration_ms,\
                    population_total,violations_total,error \
             FROM sys_quality_check_runs WHERE check_id=?1 ORDER BY started_at DESC LIMIT ?2",
            [check_id.into(), limit.clamp(1, 100).into()],
        ))
        .await?;
    rows.into_iter()
        .map(|row| {
            let started_at: String = row.try_get("", "started_at")?;
            let finished_at: Option<String> = row.try_get("", "finished_at").ok().flatten();
            Ok(QualityCheckRunSummary {
                id: row.try_get("", "id")?,
                check_id: row.try_get("", "check_id")?,
                definition_digest: row.try_get("", "definition_digest")?,
                trigger: row.try_get("", "trigger")?,
                status: row.try_get("", "status")?,
                started_at: parse_time(&started_at),
                finished_at: finished_at.as_deref().map(parse_time),
                duration_ms: row.try_get("", "duration_ms").ok().flatten(),
                population_total: row.try_get("", "population_total").ok().flatten(),
                violations_total: row.try_get("", "violations_total").ok().flatten(),
                error: row.try_get("", "error").ok().flatten(),
            })
        })
        .collect()
}

/// Latest run for every `(check_id, digest)` in one query. The overview page
/// must not do one history query per card.
pub async fn latest_runs_by_definition(
) -> anyhow::Result<HashMap<(String, String), QualityCheckRunSummary>> {
    let rows = crate::shared::data::db::get_connection()
        .query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT id,check_id,definition_digest,trigger,status,started_at,finished_at,duration_ms,
                    population_total,violations_total,error
             FROM (
               SELECT r.*,ROW_NUMBER() OVER (
                 PARTITION BY check_id,definition_digest ORDER BY started_at DESC,id DESC
               ) AS row_no
               FROM sys_quality_check_runs r
             ) WHERE row_no=1"
                .to_string(),
        ))
        .await?;
    let mut result = HashMap::with_capacity(rows.len());
    for row in rows {
        let started_at: String = row.try_get("", "started_at")?;
        let finished_at: Option<String> = row.try_get("", "finished_at").ok().flatten();
        let run = QualityCheckRunSummary {
            id: row.try_get("", "id")?,
            check_id: row.try_get("", "check_id")?,
            definition_digest: row.try_get("", "definition_digest")?,
            trigger: row.try_get("", "trigger")?,
            status: row.try_get("", "status")?,
            started_at: parse_time(&started_at),
            finished_at: finished_at.as_deref().map(parse_time),
            duration_ms: row.try_get("", "duration_ms").ok().flatten(),
            population_total: row.try_get("", "population_total").ok().flatten(),
            violations_total: row.try_get("", "violations_total").ok().flatten(),
            error: row.try_get("", "error").ok().flatten(),
        };
        result.insert((run.check_id.clone(), run.definition_digest.clone()), run);
    }
    Ok(result)
}
