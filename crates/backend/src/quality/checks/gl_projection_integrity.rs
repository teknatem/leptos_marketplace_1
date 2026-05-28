//! ## Проверка: целостность GL ↔ ProjectionLinked-проекции
//!
//! Сканирует ВСЕ записи `sys_general_ledger` с `resource_table` из набора
//! `{p909, p910, p911, p913}` и проверяет три инварианта:
//!
//! | Инвариант | Описание |
//! |-----------|----------|
//! | `orphan_gl` | GL-запись без единой строки детализации в проекции |
//! | `orphan_projection` | Строка проекции с `general_ledger_ref`, не ссылающимся ни на одну GL-запись |
//! | `amount_mismatch` | Сумма строк проекции × `resource_sign` расходится с `gl.amount` более чем на 0.01 |
//!
//! Возвращает до [`VIOLATION_SAMPLE_LIMIT`] примеров по каждому типу нарушений.

use contracts::quality::{CheckMetric, CheckResult, QualityCheckInfo, ViolationItem};
use sea_orm::{ConnectionTrait, Statement};

pub const CHECK_ID: &str = "gl_projection_integrity";

const VIOLATION_SAMPLE_LIMIT: usize = 20;
const AMOUNT_TOLERANCE: f64 = 0.01;

/// Описание одной ProjectionLinked-таблицы.
struct ProjectionTable {
    resource_table: &'static str,
    label: &'static str,
    amount_field: &'static str,
}

const PROJECTION_TABLES: &[ProjectionTable] = &[
    ProjectionTable {
        resource_table: "p909_mp_order_line_turnovers",
        label: "p909 — Обороты строк заказов МП",
        amount_field: "amount",
    },
    ProjectionTable {
        resource_table: "p910_mp_unlinked_turnovers",
        label: "p910 — Непривязанные обороты МП",
        amount_field: "amount",
    },
    ProjectionTable {
        resource_table: "p911_wb_advert_by_items",
        label: "p911 — Рекламные расходы WB по номенклатуре",
        amount_field: "amount",
    },
    ProjectionTable {
        resource_table: "p913_wb_advert_order_attr",
        label: "p913 — Атрибуция рекламных расходов по заказам WB",
        amount_field: "amount",
    },
];

pub fn info() -> QualityCheckInfo {
    QualityCheckInfo {
        code: String::new(),
        id: CHECK_ID.to_string(),
        name: "Целостность GL ↔ проекции (ProjectionLinked)".to_string(),
        description: "Проверяет три инварианта для таблиц p909/p910/p911/p913: \
                      GL без строк детализации (orphan_gl), \
                      строки проекции без GL-записи (orphan_projection), \
                      расхождение сумм (amount_mismatch)."
            .to_string(),
        category: "General Ledger".to_string(),
    }
}

pub async fn run() -> anyhow::Result<CheckResult> {
    let conn = crate::shared::data::db::get_connection();

    let mut metrics: Vec<CheckMetric> = Vec::new();
    let mut violations: Vec<ViolationItem> = Vec::new();
    let mut total_violations: i64 = 0;
    let mut total_population: i64 = 0;

    // Маленький помощник: вернуть COUNT(*) первого столбца `cnt`.
    async fn scalar_count(conn: &sea_orm::DatabaseConnection, sql: String) -> anyhow::Result<i64> {
        let rows = conn
            .query_all(Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                sql,
            ))
            .await?;
        Ok(rows
            .first()
            .and_then(|r| r.try_get::<i64>("", "cnt").ok())
            .unwrap_or(0))
    }

    for pt in PROJECTION_TABLES {
        let tbl = pt.resource_table;
        let amt = pt.amount_field;

        // ---------------------------------------------------------------
        // 1. orphan_gl — GL-записи без детализации
        // ---------------------------------------------------------------
        let orphan_gl_sql = format!(
            r#"
            SELECT gl.id AS gl_id
            FROM sys_general_ledger gl
            LEFT JOIN {tbl} d ON d.general_ledger_ref = gl.id
            WHERE gl.resource_table = '{tbl}'
              AND d.id IS NULL
            ORDER BY gl.entry_date DESC
            LIMIT 1000
            "#
        );

        let orphan_gl_rows = conn
            .query_all(Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                orphan_gl_sql,
            ))
            .await?;

        let orphan_gl_count = orphan_gl_rows.len() as i64;
        let gl_population = scalar_count(
            conn,
            format!(
                "SELECT CAST(COUNT(*) AS INTEGER) AS cnt FROM sys_general_ledger \
                 WHERE resource_table = '{tbl}'"
            ),
        )
        .await?;
        total_violations += orphan_gl_count;
        total_population += gl_population;
        metrics.push(CheckMetric {
            label: format!("{} — GL без детализации (orphan_gl)", pt.label),
            population: gl_population,
            violations: orphan_gl_count,
            unit: "GL-записей".to_string(),
        });

        for row in orphan_gl_rows
            .iter()
            .take(VIOLATION_SAMPLE_LIMIT.saturating_sub(violations.len()))
        {
            let gl_id: String = row.try_get("", "gl_id").unwrap_or_default();
            violations.push(ViolationItem {
                violation_type: "orphan_gl".to_string(),
                gl_id: Some(gl_id),
                projection_id: None,
                projection_table: Some(tbl.to_string()),
                detail: None,
            });
        }

        // ---------------------------------------------------------------
        // 2. orphan_projection — строки проекции без GL-записи
        // ---------------------------------------------------------------
        let orphan_proj_sql = format!(
            r#"
            SELECT d.id AS proj_id, d.general_ledger_ref
            FROM {tbl} d
            WHERE d.general_ledger_ref IS NOT NULL
              AND d.general_ledger_ref != ''
              AND NOT EXISTS (
                  SELECT 1 FROM sys_general_ledger gl WHERE gl.id = d.general_ledger_ref
              )
            ORDER BY d.id DESC
            LIMIT 1000
            "#
        );

        let orphan_proj_rows = conn
            .query_all(Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                orphan_proj_sql,
            ))
            .await?;

        let orphan_proj_count = orphan_proj_rows.len() as i64;
        let proj_population = scalar_count(
            conn,
            format!(
                "SELECT CAST(COUNT(*) AS INTEGER) AS cnt FROM {tbl} \
                 WHERE general_ledger_ref IS NOT NULL AND general_ledger_ref != ''"
            ),
        )
        .await?;
        total_violations += orphan_proj_count;
        total_population += proj_population;
        metrics.push(CheckMetric {
            label: format!("{} — строки без GL-записи (orphan_projection)", pt.label),
            population: proj_population,
            violations: orphan_proj_count,
            unit: "строк".to_string(),
        });

        for row in orphan_proj_rows
            .iter()
            .take(VIOLATION_SAMPLE_LIMIT.saturating_sub(violations.len()))
        {
            let proj_id: String = row.try_get("", "proj_id").unwrap_or_default();
            let gl_ref: String = row.try_get("", "general_ledger_ref").unwrap_or_default();
            violations.push(ViolationItem {
                violation_type: "orphan_projection".to_string(),
                gl_id: None,
                projection_id: Some(proj_id),
                projection_table: Some(tbl.to_string()),
                detail: Some(format!("general_ledger_ref={gl_ref}")),
            });
        }

        // ---------------------------------------------------------------
        // 3. amount_mismatch — сумма строк ≠ gl.amount
        // ---------------------------------------------------------------
        let mismatch_sql = format!(
            r#"
            SELECT
                gl.id         AS gl_id,
                gl.amount     AS gl_amount,
                gl.resource_sign AS resource_sign,
                SUM(d.{amt})  AS proj_sum
            FROM sys_general_ledger gl
            JOIN {tbl} d ON d.general_ledger_ref = gl.id
            WHERE gl.resource_table = '{tbl}'
            GROUP BY gl.id, gl.amount, gl.resource_sign
            HAVING ABS(SUM(d.{amt}) * gl.resource_sign - gl.amount) > {AMOUNT_TOLERANCE}
            ORDER BY ABS(SUM(d.{amt}) * gl.resource_sign - gl.amount) DESC
            LIMIT 1000
            "#
        );

        let mismatch_rows = conn
            .query_all(Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                mismatch_sql,
            ))
            .await?;

        let mismatch_count = mismatch_rows.len() as i64;
        // Популяция сверки сумм — GL-записи этой таблицы, у которых ЕСТЬ строки детализации.
        let mismatch_population = scalar_count(
            conn,
            format!(
                "SELECT CAST(COUNT(DISTINCT gl.id) AS INTEGER) AS cnt \
                 FROM sys_general_ledger gl \
                 JOIN {tbl} d ON d.general_ledger_ref = gl.id \
                 WHERE gl.resource_table = '{tbl}'"
            ),
        )
        .await?;
        total_violations += mismatch_count;
        total_population += mismatch_population;
        metrics.push(CheckMetric {
            label: format!("{} — расхождение суммы (amount_mismatch)", pt.label),
            population: mismatch_population,
            violations: mismatch_count,
            unit: "GL-записей".to_string(),
        });

        for row in mismatch_rows
            .iter()
            .take(VIOLATION_SAMPLE_LIMIT.saturating_sub(violations.len()))
        {
            let gl_id: String = row.try_get("", "gl_id").unwrap_or_default();
            let gl_amount: f64 = row.try_get("", "gl_amount").unwrap_or(0.0);
            let proj_sum: f64 = row.try_get("", "proj_sum").unwrap_or(0.0);
            let resource_sign: i32 = row.try_get("", "resource_sign").unwrap_or(1);
            let signed_sum = proj_sum * f64::from(resource_sign);
            let delta = signed_sum - gl_amount;
            violations.push(ViolationItem {
                violation_type: "amount_mismatch".to_string(),
                gl_id: Some(gl_id),
                projection_id: None,
                projection_table: Some(tbl.to_string()),
                detail: Some(format!(
                    "gl={gl_amount:.2}, proj={signed_sum:.2}, delta={delta:+.2}"
                )),
            });
        }
    }

    Ok(CheckResult {
        check_id: CHECK_ID.to_string(),
        run_at: chrono::Utc::now(),
        population_total: total_population,
        violations_total: total_violations,
        metrics,
        violations,
    })
}
