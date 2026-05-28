//! ## Проверка: целостность GL ↔ p903_wb_finance_report (ExternalLinked)
//!
//! В отличие от ProjectionLinked-таблиц (p909/p910/p911/p913), p903 связана с GL
//! через `gl.registrator_ref → p903.id` (или через `source_row_ref` / `rr_dt+rrd_id`
//! для устаревших форматов регистратора).
//!
//! Проверяет два инварианта:
//!
//! | Инвариант | Описание |
//! |-----------|----------|
//! | `orphan_gl` | GL-запись с `resource_table = p903`, не имеющая соответствующей строки p903 ни по одному из трёх вариантов `registrator_ref` |
//! | `amount_mismatch` | GL-запись, где сумма p903-строки × `resource_sign` расходится с `gl.amount` более чем на 0.01 |

use contracts::quality::{CheckMetric, CheckResult, QualityCheckInfo, ViolationItem};
use sea_orm::{ConnectionTrait, Statement};

pub const CHECK_ID: &str = "p903_gl_integrity";

const VIOLATION_SAMPLE_LIMIT: usize = 20;
const AMOUNT_TOLERANCE: f64 = 0.01;

pub fn info() -> QualityCheckInfo {
    QualityCheckInfo {
        code: String::new(),
        id: CHECK_ID.to_string(),
        name: "Целостность GL ↔ p903 (ExternalLinked)".to_string(),
        description: "Проверяет GL-записи, привязанные к p903_wb_finance_report: \
                      GL без соответствующей строки финотчёта (orphan_gl) \
                      и расхождение сумм (amount_mismatch)."
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

    async fn scalar_count(conn: &sea_orm::DatabaseConnection, sql: &str) -> anyhow::Result<i64> {
        let rows = conn
            .query_all(Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                sql.to_string(),
            ))
            .await?;
        Ok(rows
            .first()
            .and_then(|r| r.try_get::<i64>("", "cnt").ok())
            .unwrap_or(0))
    }

    // -----------------------------------------------------------------------
    // 1. orphan_gl — GL-записи, не resolving ни в одну строку p903.
    //
    // Три варианта формата registrator_ref (из detail_links.rs):
    //   a) plain id: d.id = gl.registrator_ref
    //   b) prefix "p903:<source_row_ref>": d.source_row_ref = gl.registrator_ref
    //   c) compound "p903:<rr_dt>:<rrd_id>": d.rr_dt = ... AND d.rrd_id = ...
    //
    // Используем CASE WHEN + LEFT JOIN для покрытия всех форматов.
    // -----------------------------------------------------------------------
    let orphan_gl_sql = r#"
        SELECT gl.id AS gl_id, gl.registrator_ref
        FROM sys_general_ledger gl
        WHERE gl.resource_table = 'p903_wb_finance_report'
          AND NOT EXISTS (
              -- variant a: direct id match
              SELECT 1 FROM p903_wb_finance_report d
              WHERE d.id = gl.registrator_ref
                AND gl.registrator_ref NOT LIKE 'p903:%'
          )
          AND NOT EXISTS (
              -- variant b: source_row_ref match ("p903:<ref>" without second colon)
              SELECT 1 FROM p903_wb_finance_report d
              WHERE d.source_row_ref = gl.registrator_ref
                AND gl.registrator_ref LIKE 'p903:%'
                AND gl.registrator_ref NOT LIKE 'p903:%:%'
          )
          AND NOT EXISTS (
              -- variant c: compound rr_dt + rrd_id ("p903:<date>:<id>")
              SELECT 1 FROM p903_wb_finance_report d
              WHERE d.rr_dt = SUBSTR(gl.registrator_ref, 6, 10)
                AND d.rrd_id = CAST(SUBSTR(gl.registrator_ref, 17) AS INTEGER)
                AND gl.registrator_ref LIKE 'p903:%:%'
          )
        ORDER BY gl.entry_date DESC
        LIMIT 1000
    "#;

    let orphan_gl_rows = conn
        .query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            orphan_gl_sql.to_string(),
        ))
        .await?;

    let orphan_gl_count = orphan_gl_rows.len() as i64;
    let gl_population = scalar_count(
        conn,
        "SELECT CAST(COUNT(*) AS INTEGER) AS cnt FROM sys_general_ledger \
         WHERE resource_table = 'p903_wb_finance_report'",
    )
    .await?;
    total_violations += orphan_gl_count;
    total_population += gl_population;
    metrics.push(CheckMetric {
        label: "p903 — GL без строки финотчёта (orphan_gl)".to_string(),
        population: gl_population,
        violations: orphan_gl_count,
        unit: "GL-записей".to_string(),
    });

    for row in orphan_gl_rows
        .iter()
        .take(VIOLATION_SAMPLE_LIMIT.saturating_sub(violations.len()))
    {
        let gl_id: String = row.try_get("", "gl_id").unwrap_or_default();
        let reg_ref: String = row.try_get("", "registrator_ref").unwrap_or_default();
        violations.push(ViolationItem {
            violation_type: "orphan_gl".to_string(),
            gl_id: Some(gl_id),
            projection_id: None,
            projection_table: Some("p903_wb_finance_report".to_string()),
            detail: Some(format!("registrator_ref={reg_ref}")),
        });
    }

    // -----------------------------------------------------------------------
    // 2. amount_mismatch — GL.amount ≠ p903.resource_field × resource_sign.
    //
    // resource_field на GL определяет, какое поле p903 является источником.
    // Для упрощения используем самый распространённый вариант через variant a/b/c,
    // опираясь на поле, явно указанное в GL (resource_field). SQLite не поддерживает
    // динамические имена колонок в простом SQL, поэтому собираем список
    // наиболее употребительных полей через CASE WHEN.
    // -----------------------------------------------------------------------
    let amount_mismatch_sql = format!(
        r#"
        SELECT
            gl.id            AS gl_id,
            gl.amount        AS gl_amount,
            gl.resource_sign AS resource_sign,
            gl.resource_field,
            gl.registrator_ref,
            CASE gl.resource_field
                WHEN 'ppvz_vw'              THEN d.ppvz_vw
                WHEN 'ppvz_vw_nds'          THEN d.ppvz_vw_nds
                WHEN 'delivery_rub'         THEN d.delivery_rub
                WHEN 'delivery_amount'      THEN d.delivery_amount
                WHEN 'penalty'              THEN d.penalty
                WHEN 'additional_payment'   THEN d.additional_payment
                WHEN 'acquiring_fee'        THEN d.acquiring_fee
                WHEN 'rebill_logistic_cost' THEN d.rebill_logistic_cost
                WHEN 'retail_amount'        THEN d.retail_amount
                WHEN 'ppvz_sales_commission' THEN d.ppvz_sales_commission
                ELSE NULL
            END AS p903_field_value
        FROM sys_general_ledger gl
        INNER JOIN p903_wb_finance_report d ON d.id = gl.registrator_ref
        WHERE gl.resource_table = 'p903_wb_finance_report'
          AND gl.registrator_ref NOT LIKE 'p903:%'
          AND CASE gl.resource_field
                WHEN 'ppvz_vw'              THEN d.ppvz_vw
                WHEN 'ppvz_vw_nds'          THEN d.ppvz_vw_nds
                WHEN 'delivery_rub'         THEN d.delivery_rub
                WHEN 'delivery_amount'      THEN d.delivery_amount
                WHEN 'penalty'              THEN d.penalty
                WHEN 'additional_payment'   THEN d.additional_payment
                WHEN 'acquiring_fee'        THEN d.acquiring_fee
                WHEN 'rebill_logistic_cost' THEN d.rebill_logistic_cost
                WHEN 'retail_amount'        THEN d.retail_amount
                WHEN 'ppvz_sales_commission' THEN d.ppvz_sales_commission
                ELSE NULL
              END IS NOT NULL
          AND ABS(
                CASE gl.resource_field
                    WHEN 'ppvz_vw'              THEN d.ppvz_vw
                    WHEN 'ppvz_vw_nds'          THEN d.ppvz_vw_nds
                    WHEN 'delivery_rub'         THEN d.delivery_rub
                    WHEN 'delivery_amount'      THEN d.delivery_amount
                    WHEN 'penalty'              THEN d.penalty
                    WHEN 'additional_payment'   THEN d.additional_payment
                    WHEN 'acquiring_fee'        THEN d.acquiring_fee
                    WHEN 'rebill_logistic_cost' THEN d.rebill_logistic_cost
                    WHEN 'retail_amount'        THEN d.retail_amount
                    WHEN 'ppvz_sales_commission' THEN d.ppvz_sales_commission
                    ELSE NULL
                END * gl.resource_sign - gl.amount
              ) > {AMOUNT_TOLERANCE}
        ORDER BY ABS(
                CASE gl.resource_field
                    WHEN 'ppvz_vw'              THEN d.ppvz_vw
                    WHEN 'ppvz_vw_nds'          THEN d.ppvz_vw_nds
                    WHEN 'delivery_rub'         THEN d.delivery_rub
                    WHEN 'delivery_amount'      THEN d.delivery_amount
                    WHEN 'penalty'              THEN d.penalty
                    WHEN 'additional_payment'   THEN d.additional_payment
                    WHEN 'acquiring_fee'        THEN d.acquiring_fee
                    WHEN 'rebill_logistic_cost' THEN d.rebill_logistic_cost
                    WHEN 'retail_amount'        THEN d.retail_amount
                    WHEN 'ppvz_sales_commission' THEN d.ppvz_sales_commission
                    ELSE NULL
                END * gl.resource_sign - gl.amount
              ) DESC
        LIMIT 1000
        "#
    );

    let mismatch_rows = conn
        .query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            amount_mismatch_sql,
        ))
        .await?;

    let mismatch_count = mismatch_rows.len() as i64;
    // Популяция сверки — GL-записи p903, прямо разрешимые в строку финотчёта (variant a).
    let mismatch_population = scalar_count(
        conn,
        "SELECT CAST(COUNT(DISTINCT gl.id) AS INTEGER) AS cnt \
         FROM sys_general_ledger gl \
         INNER JOIN p903_wb_finance_report d ON d.id = gl.registrator_ref \
         WHERE gl.resource_table = 'p903_wb_finance_report' \
           AND gl.registrator_ref NOT LIKE 'p903:%'",
    )
    .await?;
    total_violations += mismatch_count;
    total_population += mismatch_population;
    metrics.push(CheckMetric {
        label: "p903 — расхождение суммы GL vs p903-строка (amount_mismatch)".to_string(),
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
        let field_val: f64 = row.try_get("", "p903_field_value").unwrap_or(0.0);
        let resource_sign: i32 = row.try_get("", "resource_sign").unwrap_or(1);
        let resource_field: String = row.try_get("", "resource_field").unwrap_or_default();
        let signed = field_val * f64::from(resource_sign);
        let delta = signed - gl_amount;
        violations.push(ViolationItem {
            violation_type: "amount_mismatch".to_string(),
            gl_id: Some(gl_id),
            projection_id: None,
            projection_table: Some("p903_wb_finance_report".to_string()),
            detail: Some(format!(
                "field={resource_field}, gl={gl_amount:.2}, p903={signed:.2}, delta={delta:+.2}"
            )),
        });
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
