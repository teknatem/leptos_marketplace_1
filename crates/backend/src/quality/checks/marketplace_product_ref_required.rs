//! Checks that marketplace_product_ref is filled in active marketplace tables.

use contracts::quality::{CheckMetric, CheckResult, QualityCheckInfo, ViolationItem};
use sea_orm::{ConnectionTrait, Statement};

pub const CHECK_ID: &str = "marketplace_product_ref_required";

const VIOLATION_SAMPLE_LIMIT: usize = 20;

struct TableCheck {
    table: &'static str,
    label: &'static str,
    id_expr: &'static str,
    detail_expr: &'static str,
    order_expr: &'static str,
}

const TABLES: &[TableCheck] = &[
    TableCheck {
        table: "a012_wb_sales",
        label: "a012 - WB sales",
        id_expr: "id",
        detail_expr: "'sale_date=' || COALESCE(sale_date, '') || ', document_no=' || COALESCE(document_no, '') || ', sale_id=' || COALESCE(sale_id, '')",
        order_expr: "sale_date DESC, id",
    },
    TableCheck {
        table: "a013_ym_order_items",
        label: "a013 - Yandex Market order items",
        id_expr: "id",
        detail_expr: "'order_id=' || COALESCE(order_id, '') || ', line_id=' || COALESCE(line_id, '') || ', offer_id=' || COALESCE(offer_id, '')",
        order_expr: "order_id DESC, id",
    },
    TableCheck {
        table: "a015_wb_orders",
        label: "a015 - WB orders",
        id_expr: "id",
        detail_expr: "'document_date=' || COALESCE(document_date, '') || ', document_no=' || COALESCE(document_no, '') || ', g_number=' || COALESCE(g_number, '')",
        order_expr: "document_date DESC, id",
    },
    TableCheck {
        table: "p900_sales_register",
        label: "p900 - marketplace sales register",
        id_expr: "NULL",
        detail_expr: "'sale_date=' || COALESCE(sale_date, '') || ', registrator_ref=' || COALESCE(registrator_ref, '') || ', line_id=' || COALESCE(line_id, '')",
        order_expr: "sale_date DESC, registrator_ref",
    },
    TableCheck {
        table: "p904_sales_data",
        label: "p904 - sales data",
        id_expr: "id",
        detail_expr: "'date=' || COALESCE(date, '') || ', registrator_ref=' || COALESCE(registrator_ref, '') || ', article=' || COALESCE(article, '')",
        order_expr: "date DESC, id",
    },
    TableCheck {
        table: "p909_mp_order_line_turnovers",
        label: "p909 - order line turnovers",
        id_expr: "id",
        detail_expr: "'entry_date=' || COALESCE(entry_date, '') || ', registrator_ref=' || COALESCE(registrator_ref, '') || ', order_key=' || COALESCE(order_key, '')",
        order_expr: "entry_date DESC, id",
    },
    TableCheck {
        table: "p911_wb_advert_by_items",
        label: "p911 - WB advert by items",
        id_expr: "id",
        detail_expr: "'entry_date=' || COALESCE(entry_date, '') || ', registrator_ref=' || COALESCE(registrator_ref, '') || ', campaign=' || COALESCE(wb_advert_campaign_code, '')",
        order_expr: "entry_date DESC, id",
    },
];

pub fn info() -> QualityCheckInfo {
    QualityCheckInfo {
        code: String::new(),
        id: CHECK_ID.to_string(),
        name: "Заполненность товара маркетплейса".to_string(),
        description:
            "Проверяет, что marketplace_product_ref заполнен в активных таблицах маркетплейсов."
                .to_string(),
        category: "Маркетплейсы".to_string(),
    }
}

pub async fn run() -> anyhow::Result<CheckResult> {
    let conn = crate::shared::data::db::get_connection();

    let mut metrics = Vec::with_capacity(TABLES.len());
    let mut violations = Vec::new();

    for table in TABLES {
        let count_sql = format!(
            r#"
            SELECT
                CAST(COUNT(*) AS INTEGER) AS population,
                CAST(SUM(CASE WHEN marketplace_product_ref IS NULL OR TRIM(marketplace_product_ref) = '' THEN 1 ELSE 0 END) AS INTEGER) AS violations
            FROM {table_name}
            "#,
            table_name = table.table
        );
        let count_rows = conn
            .query_all(Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                count_sql,
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

        metrics.push(CheckMetric {
            label: table.label.to_string(),
            population,
            violations: missing_count,
            unit: "строк".to_string(),
        });

        let remaining = VIOLATION_SAMPLE_LIMIT.saturating_sub(violations.len());
        if missing_count == 0 || remaining == 0 {
            continue;
        }

        let sample_sql = format!(
            r#"
            SELECT
                {id_expr} AS projection_id,
                {detail_expr} AS detail
            FROM {table_name}
            WHERE marketplace_product_ref IS NULL OR TRIM(marketplace_product_ref) = ''
            ORDER BY {order_expr}
            LIMIT {limit}
            "#,
            id_expr = table.id_expr,
            detail_expr = table.detail_expr,
            table_name = table.table,
            order_expr = table.order_expr,
            limit = remaining
        );
        let sample_rows = conn
            .query_all(Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                sample_sql,
            ))
            .await?;

        for row in sample_rows {
            let projection_id: Option<String> = row.try_get("", "projection_id").ok().flatten();
            let detail: Option<String> = row.try_get("", "detail").ok();
            violations.push(ViolationItem {
                violation_type: "missing_marketplace_product_ref".to_string(),
                gl_id: None,
                projection_id,
                projection_table: Some(table.table.to_string()),
                detail,
            });
        }
    }

    let population_total = metrics.iter().map(|metric| metric.population).sum();
    let violations_total = metrics.iter().map(|metric| metric.violations).sum();

    Ok(CheckResult {
        check_id: CHECK_ID.to_string(),
        run_at: chrono::Utc::now(),
        population_total,
        violations_total,
        metrics,
        violations,
    })
}
