//! ## Проверка: полнота проведения p907_ym_payment_report → GL
//!
//! Гарантия «100% соответствия проведённых документов»: каждая ненулевая строка
//! отчёта по платежам YM обязана сформировать хотя бы одну GL-проводку. После
//! добавления универсальных оборотов «Прочие доходы/расходы» (catch-all) не
//! существует операции, которая молча выпадает из учёта.
//!
//! | Инвариант | Описание |
//! |-----------|----------|
//! | `missing_gl` | Строка p907 с ненулевой `transaction_sum`, у которой нет ни одной GL-записи (`registrator_type='p907_ym_payment_report'`, `registrator_ref=p907.id`) |
//!
//! Популяция — все «проводимые» строки (ненулевая сумма). Строки с нулевой/пустой
//! суммой проводку не формируют по дизайну и в популяцию не входят.

use contracts::quality::{CheckMetric, CheckResult, QualityCheckInfo, ViolationItem};
use sea_orm::{ConnectionTrait, Statement};

pub const CHECK_ID: &str = "p907_gl_coverage";

const VIOLATION_SAMPLE_LIMIT: usize = 20;

pub fn info() -> QualityCheckInfo {
    QualityCheckInfo {
        code: String::new(),
        id: CHECK_ID.to_string(),
        name: "Полнота проведения p907 (YM) → GL".to_string(),
        description: "Проверяет, что каждая ненулевая строка отчёта по платежам Yandex Market \
                      сформировала GL-проводку (missing_gl). Гарантия 100% покрытия после \
                      добавления универсальных оборотов «Прочие доходы/расходы»."
            .to_string(),
        category: "General Ledger".to_string(),
    }
}

pub async fn run() -> anyhow::Result<CheckResult> {
    let conn = crate::shared::data::db::get_connection();

    // Популяция и нарушения одним проходом.
    let count_sql = r#"
        SELECT
            CAST(COUNT(*) AS INTEGER) AS population,
            CAST(SUM(
                CASE WHEN NOT EXISTS (
                    SELECT 1 FROM sys_general_ledger g
                    WHERE g.registrator_type = 'p907_ym_payment_report'
                      AND g.registrator_ref = p.id
                ) THEN 1 ELSE 0 END
            ) AS INTEGER) AS violations
        FROM p907_ym_payment_report p
        WHERE p.transaction_sum IS NOT NULL AND p.transaction_sum <> 0
    "#;
    let count_rows = conn
        .query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            count_sql.to_string(),
        ))
        .await?;
    let (population, missing_count) = count_rows
        .first()
        .map(|row| {
            (
                row.try_get::<i64>("", "population").unwrap_or(0),
                row.try_get::<i64>("", "violations").unwrap_or(0),
            )
        })
        .unwrap_or((0, 0));

    let metrics = vec![CheckMetric {
        label: "p907 — ненулевые строки без GL-проводки (missing_gl)".to_string(),
        population,
        violations: missing_count,
        unit: "строк".to_string(),
    }];

    let mut violations = Vec::new();
    if missing_count > 0 {
        let sample_sql = format!(
            r#"
            SELECT
                p.id AS projection_id,
                'transaction_date=' || COALESCE(p.transaction_date, '')
                    || ', source=' || COALESCE(p.transaction_source, '')
                    || ', sum=' || COALESCE(CAST(p.transaction_sum AS TEXT), '')
                    || ', order_id=' || COALESCE(CAST(p.order_id AS TEXT), '') AS detail
            FROM p907_ym_payment_report p
            WHERE p.transaction_sum IS NOT NULL AND p.transaction_sum <> 0
              AND NOT EXISTS (
                  SELECT 1 FROM sys_general_ledger g
                  WHERE g.registrator_type = 'p907_ym_payment_report'
                    AND g.registrator_ref = p.id
              )
            ORDER BY p.transaction_date DESC, p.id
            LIMIT {limit}
            "#,
            limit = VIOLATION_SAMPLE_LIMIT
        );
        let sample_rows = conn
            .query_all(Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                sample_sql,
            ))
            .await?;
        for row in sample_rows {
            let projection_id: Option<String> = row.try_get("", "projection_id").ok();
            let detail: Option<String> = row.try_get("", "detail").ok();
            violations.push(ViolationItem {
                violation_type: "missing_gl".to_string(),
                gl_id: None,
                projection_id,
                projection_table: Some("p907_ym_payment_report".to_string()),
                detail,
            });
        }
    }

    Ok(CheckResult {
        check_id: CHECK_ID.to_string(),
        run_at: chrono::Utc::now(),
        population_total: population,
        violations_total: missing_count,
        metrics,
        violations,
    })
}
