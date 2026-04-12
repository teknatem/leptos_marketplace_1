//! SQL-запросы для GL-отчёта и GL-first drilldown через detail projections.

use anyhow::Result;
use sea_orm::{ConnectionTrait, Statement, Value};

use crate::shared::data::db::get_connection;
use contracts::general_ledger::{GlDrilldownQuery, GlReportQuery};
use contracts::general_ledger::{
    GlDrilldownResponse, GlDrilldownRow, GlReportResponse, GlReportRow,
};

use super::detail_links::descriptor_for_resource_table;
use super::drilldown_dimensions::{dimension_label, is_nomenclature_dimension};
use super::turnover_registry::get_turnover_class;

fn conn() -> &'static sea_orm::DatabaseConnection {
    get_connection()
}

fn string_value(value: impl Into<String>) -> Value {
    Value::String(Some(Box::new(value.into())))
}

fn qualified_column(alias: &str, column: &str) -> String {
    if alias.is_empty() {
        column.to_string()
    } else {
        format!("{alias}.{column}")
    }
}

fn append_sys_gl_account_filter(
    sql: &mut String,
    params: &mut Vec<Value>,
    alias: &str,
    account: &Option<String>,
) {
    if let Some(account) = account.as_ref().filter(|value| !value.trim().is_empty()) {
        let debit_account = qualified_column(alias, "debit_account");
        let credit_account = qualified_column(alias, "credit_account");
        sql.push_str(&format!(
            " AND ({debit_account} = ? OR {credit_account} = ?)"
        ));
        params.push(string_value(account.clone()));
        params.push(string_value(account.clone()));
    }
}

fn append_corr_account_filter(
    sql: &mut String,
    params: &mut Vec<Value>,
    alias: &str,
    query: &GlDrilldownQuery,
) {
    let Some(account) = query
        .account
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    else {
        return;
    };
    let Some(corr_account) = query
        .corr_account
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    else {
        return;
    };

    let debit_account = qualified_column(alias, "debit_account");
    let credit_account = qualified_column(alias, "credit_account");
    sql.push_str(&format!(
        " AND (CASE WHEN {debit_account} = ? THEN {credit_account} ELSE {debit_account} END) = ?"
    ));
    params.push(string_value(account.clone()));
    params.push(string_value(corr_account.clone()));
}

fn build_signed_amount_expr(alias: &str, signed_by_account: bool) -> String {
    let row_signed_amount_expr = build_row_signed_amount_expr(alias, signed_by_account);
    format!("COALESCE(SUM({row_signed_amount_expr}), 0.0)")
}

fn build_row_signed_amount_expr(alias: &str, signed_by_account: bool) -> String {
    let amount = qualified_column(alias, "amount");
    if !signed_by_account {
        return format!("COALESCE({amount}, 0.0)");
    }

    let debit_account = qualified_column(alias, "debit_account");
    let credit_account = qualified_column(alias, "credit_account");
    format!(
        "COALESCE(CASE WHEN {debit_account} = ? THEN {amount} WHEN {credit_account} = ? THEN -{amount} ELSE 0.0 END, 0.0)"
    )
}

fn build_row_sign_factor_expr(alias: &str, signed_by_account: bool) -> String {
    if !signed_by_account {
        return "1.0".to_string();
    }

    let debit_account = qualified_column(alias, "debit_account");
    let credit_account = qualified_column(alias, "credit_account");
    format!(
        "CASE WHEN {debit_account} = ? THEN 1.0 WHEN {credit_account} = ? THEN -1.0 ELSE 0.0 END"
    )
}

fn append_signed_amount_params(params: &mut Vec<Value>, account: &Option<String>) {
    if let Some(account) = account.as_ref().filter(|value| !value.trim().is_empty()) {
        params.push(string_value(account.clone()));
        params.push(string_value(account.clone()));
    }
}

fn append_connection_filter(
    sql: &mut String,
    params: &mut Vec<Value>,
    alias: &str,
    connection_mp_ref: &Option<String>,
    connection_mp_refs: &[String],
) {
    let column = if alias.is_empty() {
        "connection_mp_ref".to_string()
    } else {
        format!("{alias}.connection_mp_ref")
    };

    let refs: Vec<String> = if !connection_mp_refs.is_empty() {
        connection_mp_refs
            .iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect()
    } else {
        connection_mp_ref
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .into_iter()
            .collect()
    };

    if refs.is_empty() {
        return;
    }

    if refs.len() == 1 {
        sql.push_str(&format!(" AND {column} = ?"));
        params.push(string_value(refs[0].clone()));
        return;
    }

    let placeholders: Vec<&str> = refs.iter().map(|_| "?").collect();
    sql.push_str(&format!(" AND {column} IN ({})", placeholders.join(", ")));
    for value in refs {
        params.push(string_value(value));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Сводный отчёт
// ─────────────────────────────────────────────────────────────────────────────

pub async fn get_report(query: &GlReportQuery) -> Result<GlReportResponse> {
    let rows = if let Some(account) = &query.account {
        get_report_with_account(query, account).await?
    } else {
        get_report_without_account(query).await?
    };

    let total_debit: f64 = rows.iter().map(|r| r.debit_amount).sum();
    let total_credit: f64 = rows.iter().map(|r| r.credit_amount).sum();
    let total_balance = total_debit - total_credit;

    Ok(GlReportResponse {
        rows,
        total_debit,
        total_credit,
        total_balance,
    })
}

async fn get_report_with_account(query: &GlReportQuery, account: &str) -> Result<Vec<GlReportRow>> {
    let mut sql = String::from(
        r#"
        SELECT
            turnover_code,
            COALESCE(layer, '') AS layer,
            COALESCE(SUM(CASE WHEN debit_account = ? THEN amount ELSE 0.0 END), 0.0) AS debit_amount,
            COALESCE(SUM(CASE WHEN credit_account = ? THEN amount ELSE 0.0 END), 0.0) AS credit_amount,
            COUNT(*) AS entry_count
        FROM sys_general_ledger
        WHERE entry_date >= ?
          AND entry_date <= ?
          AND (debit_account = ? OR credit_account = ?)
        "#,
    );

    let mut params: Vec<Value> = vec![
        Value::String(Some(Box::new(account.to_string()))),
        Value::String(Some(Box::new(account.to_string()))),
        Value::String(Some(Box::new(query.date_from.clone()))),
        Value::String(Some(Box::new(query.date_to.clone()))),
        Value::String(Some(Box::new(account.to_string()))),
        Value::String(Some(Box::new(account.to_string()))),
    ];

    if let Some(cab) = &query.connection_mp_ref {
        sql.push_str(" AND connection_mp_ref = ?");
        params.push(Value::String(Some(Box::new(cab.clone()))));
    }
    if let Some(layer) = &query.layer {
        sql.push_str(" AND layer = ?");
        params.push(Value::String(Some(Box::new(layer.clone()))));
    }

    sql.push_str(" GROUP BY turnover_code, layer ORDER BY layer, turnover_code");

    execute_report_query(&sql, params).await
}

async fn get_report_without_account(query: &GlReportQuery) -> Result<Vec<GlReportRow>> {
    let mut sql = String::from(
        r#"
        SELECT
            turnover_code,
            COALESCE(layer, '') AS layer,
            COALESCE(SUM(amount), 0.0) AS debit_amount,
            COALESCE(SUM(amount), 0.0) AS credit_amount,
            COUNT(*) AS entry_count
        FROM sys_general_ledger
        WHERE entry_date >= ?
          AND entry_date <= ?
        "#,
    );

    let mut params: Vec<Value> = vec![
        Value::String(Some(Box::new(query.date_from.clone()))),
        Value::String(Some(Box::new(query.date_to.clone()))),
    ];

    if let Some(cab) = &query.connection_mp_ref {
        sql.push_str(" AND connection_mp_ref = ?");
        params.push(Value::String(Some(Box::new(cab.clone()))));
    }
    if let Some(layer) = &query.layer {
        sql.push_str(" AND layer = ?");
        params.push(Value::String(Some(Box::new(layer.clone()))));
    }

    sql.push_str(" GROUP BY turnover_code, layer ORDER BY layer, turnover_code");

    execute_report_query(&sql, params).await
}

async fn execute_report_query(sql: &str, params: Vec<Value>) -> Result<Vec<GlReportRow>> {
    let stmt = Statement::from_sql_and_values(conn().get_database_backend(), sql, params);
    let rows = conn().query_all(stmt).await?;

    let mut result = Vec::with_capacity(rows.len());
    for row in rows {
        let turnover_code: String = row.try_get("", "turnover_code")?;
        let layer: String = row.try_get("", "layer").unwrap_or_default();
        let debit_amount: f64 = row.try_get("", "debit_amount")?;
        let credit_amount: f64 = row.try_get("", "credit_amount")?;
        let entry_count: i64 = row.try_get("", "entry_count")?;

        let turnover_name = get_turnover_class(&turnover_code)
            .map(|tc| tc.name.to_string())
            .unwrap_or_else(|| turnover_code.clone());

        result.push(GlReportRow {
            balance: debit_amount - credit_amount,
            turnover_code,
            turnover_name,
            layer,
            debit_amount,
            credit_amount,
            entry_count,
        });
    }

    // Обогатить нулями обороты, которые есть в реестре но нет в запросе
    // (опционально — не добавляем, чтобы не засорять пустыми строками)
    Ok(result)
}

// ─────────────────────────────────────────────────────────────────────────────
// Drilldown from GL to detail projections
// ─────────────────────────────────────────────────────────────────────────────

pub async fn get_drilldown(query: &GlDrilldownQuery) -> Result<GlDrilldownResponse> {
    let turnover_name = get_turnover_class(&query.turnover_code)
        .map(|tc| tc.name.to_string())
        .unwrap_or_else(|| query.turnover_code.clone());

    let group_by_label = dimension_label(&query.group_by)
        .unwrap_or("Неизвестно")
        .to_string();

    // Детализация по документу-регистратору: запрос из sys_general_ledger напрямую
    if query.group_by == "registrator_ref" {
        return get_drilldown_by_registrator(query, turnover_name, group_by_label).await;
    }

    let rows = if is_common_dimension(&query.group_by) {
        query_sys_gl_common_drilldown(query).await?
    } else if is_nomenclature_dimension(&query.group_by) {
        query_detail_nomenclature_drilldown(query).await?
    } else {
        return Err(anyhow::anyhow!(
            "Unsupported drilldown dimension '{}' for turnover '{}'",
            query.group_by,
            query.turnover_code
        ));
    };

    let total_amount: f64 = rows.iter().map(|r| r.amount).sum();
    let total_count: i64 = rows.iter().map(|r| r.entry_count).sum();

    Ok(GlDrilldownResponse {
        rows,
        group_by_label,
        turnover_code: query.turnover_code.clone(),
        turnover_name,
        total_amount,
        total_count,
    })
}

fn is_common_dimension(dimension_id: &str) -> bool {
    matches!(
        dimension_id,
        "entry_date" | "connection_mp_ref" | "registrator_type" | "layer"
    )
}

/// Drilldown to registrator document level.
/// Queries sys_general_ledger directly and groups by registrator identity.
/// group_key = "{registrator_type}~~{registrator_ref}" where registrator_ref is a pure id.
/// group_label = readable short label.
async fn get_drilldown_by_registrator(
    query: &GlDrilldownQuery,
    turnover_name: String,
    group_by_label: String,
) -> Result<GlDrilldownResponse> {
    // group_key = "{registrator_type}~~{registrator_ref}" — тильды-разделитель,
    // нужный для разбора на фронте.
    // group_label = MIN(entry_date) (дата первой проводки по этому документу).
    let amount_expr = build_signed_amount_expr(
        "",
        query
            .account
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty()),
    );
    let mut sql = format!(
        r#"
        SELECT
            COALESCE(registrator_type, '') || '~~' || COALESCE(registrator_ref, '') AS group_key,
            SUBSTR(MIN(entry_date), 1, 10) AS group_label,
            {amount_expr} AS amount,
            COUNT(*) AS entry_count
        FROM sys_general_ledger
        WHERE turnover_code = ?
          AND entry_date >= ?
          AND entry_date <= ?
        "#,
    );

    let mut params: Vec<Value> = Vec::new();
    append_signed_amount_params(&mut params, &query.account);
    params.extend(vec![
        string_value(query.turnover_code.clone()),
        string_value(query.date_from.clone()),
        string_value(query.date_to.clone()),
    ]);

    append_connection_filter(
        &mut sql,
        &mut params,
        "",
        &query.connection_mp_ref,
        &query.connection_mp_refs,
    );
    append_sys_gl_account_filter(&mut sql, &mut params, "", &query.account);
    append_corr_account_filter(&mut sql, &mut params, "", query);
    if let Some(layer) = &query.layer {
        sql.push_str(" AND layer = ?");
        params.push(string_value(layer.clone()));
    }

    sql.push_str(" GROUP BY registrator_type, registrator_ref ORDER BY amount DESC");

    let rows = execute_drilldown_query(&sql, params).await?;
    let total_amount: f64 = rows.iter().map(|r| r.amount).sum();
    let total_count: i64 = rows.iter().map(|r| r.entry_count).sum();

    Ok(GlDrilldownResponse {
        rows,
        group_by_label,
        turnover_code: query.turnover_code.clone(),
        turnover_name,
        total_amount,
        total_count,
    })
}

fn build_sys_gl_dimension_sql(
    alias: &str,
    dimension_id: &str,
) -> (String, String, String, String, &'static str) {
    match dimension_id {
        "entry_date" => (
            format!("SUBSTR({alias}.entry_date, 1, 10)"),
            format!("SUBSTR({alias}.entry_date, 1, 10)"),
            String::new(),
            format!("SUBSTR({alias}.entry_date, 1, 10)"),
            " ORDER BY group_key ASC",
        ),
        "connection_mp_ref" => (
            format!("COALESCE({alias}.connection_mp_ref, '(не указано)')"),
            format!("COALESCE(c.description, {alias}.connection_mp_ref, '(не указано)')"),
            format!("LEFT JOIN a006_connection_mp c ON {alias}.connection_mp_ref = c.id"),
            format!("{alias}.connection_mp_ref"),
            " ORDER BY amount DESC",
        ),
        "registrator_type" => (
            format!("COALESCE({alias}.registrator_type, '(не указано)')"),
            format!("COALESCE({alias}.registrator_type, '(не указано)')"),
            String::new(),
            format!("{alias}.registrator_type"),
            " ORDER BY amount DESC",
        ),
        "layer" => (
            format!("COALESCE({alias}.layer, '(не указано)')"),
            format!("COALESCE({alias}.layer, '(не указано)')"),
            String::new(),
            format!("{alias}.layer"),
            " ORDER BY amount DESC",
        ),
        _ => (
            format!("COALESCE({alias}.registrator_type, '(не указано)')"),
            format!("COALESCE({alias}.registrator_type, '(не указано)')"),
            String::new(),
            format!("{alias}.registrator_type"),
            " ORDER BY amount DESC",
        ),
    }
}

fn build_nomenclature_dimension_sql(
    nomenclature_ref_expr: &str,
    dimension_id: &str,
) -> (String, String, String) {
    if dimension_id == "nomenclature" {
        return (
            format!("COALESCE({nomenclature_ref_expr}, '(без номенклатуры)')"),
            format!("COALESCE(n.description, {nomenclature_ref_expr}, '(без номенклатуры)')"),
            nomenclature_ref_expr.to_string(),
        );
    }

    match dimension_id {
        "dim1_category" => (
            "COALESCE(n.dim1_category, '(не указано)')".to_string(),
            "COALESCE(n.dim1_category, '(не указано)')".to_string(),
            "n.dim1_category".to_string(),
        ),
        "dim2_line" => (
            "COALESCE(n.dim2_line, '(не указано)')".to_string(),
            "COALESCE(n.dim2_line, '(не указано)')".to_string(),
            "n.dim2_line".to_string(),
        ),
        "dim3_model" => (
            "COALESCE(n.dim3_model, '(не указано)')".to_string(),
            "COALESCE(n.dim3_model, '(не указано)')".to_string(),
            "n.dim3_model".to_string(),
        ),
        "dim4_format" => (
            "COALESCE(n.dim4_format, '(не указано)')".to_string(),
            "COALESCE(n.dim4_format, '(не указано)')".to_string(),
            "n.dim4_format".to_string(),
        ),
        "dim5_sink" => (
            "COALESCE(n.dim5_sink, '(не указано)')".to_string(),
            "COALESCE(n.dim5_sink, '(не указано)')".to_string(),
            "n.dim5_sink".to_string(),
        ),
        "dim6_size" => (
            "COALESCE(n.dim6_size, '(не указано)')".to_string(),
            "COALESCE(n.dim6_size, '(не указано)')".to_string(),
            "n.dim6_size".to_string(),
        ),
        _ => (
            format!("COALESCE({nomenclature_ref_expr}, '(без номенклатуры)')"),
            format!("COALESCE(n.description, {nomenclature_ref_expr}, '(без номенклатуры)')"),
            nomenclature_ref_expr.to_string(),
        ),
    }
}

fn append_common_gl_filters(
    sql: &mut String,
    params: &mut Vec<Value>,
    query: &GlDrilldownQuery,
    alias: &str,
) {
    append_connection_filter(
        sql,
        params,
        alias,
        &query.connection_mp_ref,
        &query.connection_mp_refs,
    );
    append_sys_gl_account_filter(sql, params, alias, &query.account);
    append_corr_account_filter(sql, params, alias, query);
    if let Some(layer) = &query.layer {
        let layer_column = qualified_column(alias, "layer");
        sql.push_str(&format!(" AND {layer_column} = ?"));
        params.push(string_value(layer.clone()));
    }
}

async fn query_sys_gl_common_drilldown(query: &GlDrilldownQuery) -> Result<Vec<GlDrilldownRow>> {
    let (select_key, select_label, from_join, group_by_expr, order_by) =
        build_sys_gl_dimension_sql("gl", &query.group_by);
    let amount_expr = build_signed_amount_expr(
        "gl",
        query
            .account
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty()),
    );
    let mut sql = format!(
        r#"
        SELECT
            {select_key} AS group_key,
            {select_label} AS group_label,
            {amount_expr} AS amount,
            COUNT(*) AS entry_count
        FROM sys_general_ledger gl
        {from_join}
        WHERE gl.turnover_code = ?
          AND gl.entry_date >= ?
          AND gl.entry_date <= ?
        "#
    );
    let mut params = Vec::new();
    append_signed_amount_params(&mut params, &query.account);
    params.extend(vec![
        string_value(query.turnover_code.clone()),
        string_value(query.date_from.clone()),
        string_value(query.date_to.clone()),
    ]);
    append_common_gl_filters(&mut sql, &mut params, query, "gl");
    sql.push_str(&format!(" GROUP BY {group_by_expr}{order_by}"));
    execute_drilldown_query(&sql, params).await
}

fn build_matched_gl_cte(query: &GlDrilldownQuery, params: &mut Vec<Value>) -> String {
    let signed_by_account = query
        .account
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty());
    let signed_amount_expr = build_row_signed_amount_expr("gl", signed_by_account);
    let detail_sign_factor_expr = build_row_sign_factor_expr("gl", signed_by_account);
    append_signed_amount_params(params, &query.account);
    append_signed_amount_params(params, &query.account);
    params.push(string_value(query.turnover_code.clone()));
    params.push(string_value(query.date_from.clone()));
    params.push(string_value(query.date_to.clone()));

    let mut sql = format!(
        r#"
        matched_gl AS (
            SELECT
                gl.id,
                gl.resource_table,
                gl.resource_sign,
                gl.registrator_type,
                gl.registrator_ref,
                {signed_amount_expr} AS signed_amount,
                {detail_sign_factor_expr} AS detail_sign_factor
            FROM sys_general_ledger gl
            WHERE gl.turnover_code = ?
              AND gl.entry_date >= ?
              AND gl.entry_date <= ?
        "#,
    );

    append_common_gl_filters(&mut sql, params, query, "gl");
    sql.push_str("\n        )");
    sql
}

fn build_detail_amount_expr(descriptor: &super::detail_links::GlDetailLinkDescriptor) -> String {
    match descriptor.kind {
        super::detail_links::GlDetailLinkKind::ExternalLinked => {
            "COALESCE(SUM(gl_row.signed_amount), 0.0)".to_string()
        }
        super::detail_links::GlDetailLinkKind::ProjectionLinked => {
            "COALESCE(SUM(COALESCE(d.amount, 0.0) * COALESCE(gl.resource_sign, 1) * COALESCE(gl.detail_sign_factor, 1.0)), 0.0)"
                .to_string()
        }
    }
}

fn build_detail_entry_count_expr(
    descriptor: &super::detail_links::GlDetailLinkDescriptor,
) -> &'static str {
    match descriptor.kind {
        super::detail_links::GlDetailLinkKind::ExternalLinked => "COUNT(DISTINCT gl_row.id)",
        super::detail_links::GlDetailLinkKind::ProjectionLinked => "COUNT(DISTINCT gl.id)",
    }
}

fn build_detail_source_sql(table_name: &str, query: &GlDrilldownQuery) -> String {
    let Some(descriptor) = descriptor_for_resource_table(table_name) else {
        return String::new();
    };

    let gl_alias = match descriptor.kind {
        super::detail_links::GlDetailLinkKind::ExternalLinked => "gl_row",
        super::detail_links::GlDetailLinkKind::ProjectionLinked => "gl",
    };
    let (select_key, select_label, group_by_expr) =
        build_nomenclature_dimension_sql(descriptor.nomenclature_ref_expr, &query.group_by);
    let amount_expr = build_detail_amount_expr(descriptor);
    let entry_count_expr = build_detail_entry_count_expr(descriptor);

    descriptor
        .join_variants
        .iter()
        .map(|join_variant| {
            let variant_where_sql = if join_variant.extra_where_sql.is_empty() {
                descriptor.where_sql.to_string()
            } else {
                format!(
                    "{} AND {}",
                    descriptor.where_sql, join_variant.extra_where_sql
                )
            };
            format!(
                r#"
        SELECT
            {select_key} AS group_key,
            {select_label} AS group_label,
            {amount_expr} AS amount,
            {entry_count_expr} AS entry_count
        FROM matched_gl {gl_alias}
        {join_sql}
        LEFT JOIN a004_nomenclature n ON {nomenclature_ref_expr} = n.id
        WHERE {where_sql}
        GROUP BY {group_by_expr}
        "#,
                join_sql = join_variant.join_sql,
                where_sql = variant_where_sql,
                nomenclature_ref_expr = descriptor.nomenclature_ref_expr,
            )
        })
        .collect::<Vec<_>>()
        .join("\n        UNION ALL\n")
}

async fn list_detail_resource_tables(query: &GlDrilldownQuery) -> Result<Vec<String>> {
    let mut sql = String::from(
        r#"
        SELECT DISTINCT gl.resource_table
        FROM sys_general_ledger gl
        WHERE gl.turnover_code = ?
          AND gl.entry_date >= ?
          AND gl.entry_date <= ?
          AND gl.resource_table IS NOT NULL
          AND TRIM(gl.resource_table) <> ''
        "#,
    );
    let mut params = vec![
        string_value(query.turnover_code.clone()),
        string_value(query.date_from.clone()),
        string_value(query.date_to.clone()),
    ];
    append_common_gl_filters(&mut sql, &mut params, query, "gl");
    sql.push_str(" ORDER BY gl.resource_table");

    let stmt = Statement::from_sql_and_values(conn().get_database_backend(), &sql, params);
    let rows = conn().query_all(stmt).await?;
    let mut tables = Vec::with_capacity(rows.len());
    for row in rows {
        let table: String = row.try_get("", "resource_table")?;
        tables.push(table);
    }
    Ok(tables)
}

async fn query_detail_nomenclature_drilldown(
    query: &GlDrilldownQuery,
) -> Result<Vec<GlDrilldownRow>> {
    let resource_tables = list_detail_resource_tables(query).await?;
    if resource_tables.is_empty() {
        return Ok(Vec::new());
    }

    let mut supported_tables = Vec::new();
    let mut unsupported_tables = Vec::new();
    for table in resource_tables {
        if descriptor_for_resource_table(&table).is_some() {
            supported_tables.push(table);
        } else {
            unsupported_tables.push(table);
        }
    }

    if !unsupported_tables.is_empty() {
        return Err(anyhow::anyhow!(
            "Nomenclature drilldown is not implemented for resource_table(s): {}",
            unsupported_tables.join(", ")
        ));
    }

    let mut params: Vec<Value> = Vec::new();
    let matched_gl = build_matched_gl_cte(query, &mut params);
    let sources_sql = supported_tables
        .iter()
        .map(|table_name| build_detail_source_sql(table_name, query))
        .collect::<Vec<_>>()
        .join("\n            UNION ALL\n");

    let sql = format!(
        r#"
        WITH
        {matched_gl}
        SELECT
            group_key,
            group_label,
            COALESCE(SUM(amount), 0.0) AS amount,
            COALESCE(SUM(entry_count), 0) AS entry_count
        FROM (
            {sources_sql}
        ) q
        GROUP BY group_key, group_label
        ORDER BY amount DESC
        "#
    );

    execute_drilldown_query(&sql, params).await
}

async fn execute_drilldown_query(sql: &str, params: Vec<Value>) -> Result<Vec<GlDrilldownRow>> {
    let stmt = Statement::from_sql_and_values(conn().get_database_backend(), sql, params);
    let rows = conn().query_all(stmt).await?;

    let mut result = Vec::with_capacity(rows.len());
    for row in rows {
        let group_key: String = row.try_get("", "group_key").unwrap_or_default();
        let group_label: String = row.try_get("", "group_label").unwrap_or_default();
        let amount: f64 = row.try_get("", "amount").unwrap_or(0.0);
        let entry_count: i64 = row.try_get("", "entry_count").unwrap_or(0);
        result.push(GlDrilldownRow {
            group_key,
            group_label,
            amount,
            entry_count,
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::{
        append_common_gl_filters, append_signed_amount_params, build_detail_source_sql,
        build_matched_gl_cte, build_row_sign_factor_expr, build_signed_amount_expr,
    };
    use contracts::general_ledger::GlDrilldownQuery;
    use sea_orm::Value;

    fn str_value(value: &Value) -> Option<&str> {
        match value {
            Value::String(Some(raw)) => Some(raw.as_ref().as_str()),
            _ => None,
        }
    }

    #[test]
    fn signed_amount_expr_uses_account_side_as_sign() {
        let expr = build_signed_amount_expr("gl", true);
        assert!(expr.contains("CASE WHEN gl.debit_account = ? THEN gl.amount"));
        assert!(expr.contains("WHEN gl.credit_account = ? THEN -gl.amount"));

        let mut params = Vec::new();
        append_signed_amount_params(&mut params, &Some("7609".to_string()));
        assert_eq!(params.len(), 2);
        assert_eq!(str_value(&params[0]), Some("7609"));
        assert_eq!(str_value(&params[1]), Some("7609"));
    }

    #[test]
    fn common_filters_include_corr_account_match_for_account_view_rows() {
        let mut sql = String::new();
        let mut params = Vec::new();
        let query = GlDrilldownQuery {
            turnover_code: "mp_acquiring".to_string(),
            group_by: "registrator_ref".to_string(),
            date_from: "2026-04-01".to_string(),
            date_to: "2026-04-30".to_string(),
            connection_mp_ref: None,
            connection_mp_refs: vec![],
            account: Some("7609".to_string()),
            layer: Some("fact".to_string()),
            corr_account: Some("4403".to_string()),
        };

        append_common_gl_filters(&mut sql, &mut params, &query, "gl");

        assert!(sql.contains("(gl.debit_account = ? OR gl.credit_account = ?)"));
        assert!(sql.contains(
            "(CASE WHEN gl.debit_account = ? THEN gl.credit_account ELSE gl.debit_account END) = ?"
        ));
        assert!(sql.contains("gl.layer = ?"));
        assert_eq!(params.len(), 5);
        assert_eq!(str_value(&params[0]), Some("7609"));
        assert_eq!(str_value(&params[1]), Some("7609"));
        assert_eq!(str_value(&params[2]), Some("7609"));
        assert_eq!(str_value(&params[3]), Some("4403"));
        assert_eq!(str_value(&params[4]), Some("fact"));
    }

    #[test]
    fn matched_gl_cte_keeps_row_level_amounts() {
        let mut params = Vec::new();
        let query = GlDrilldownQuery {
            turnover_code: "mp_penalty".to_string(),
            group_by: "nomenclature".to_string(),
            date_from: "2026-02-01".to_string(),
            date_to: "2026-02-28".to_string(),
            connection_mp_ref: None,
            connection_mp_refs: vec![],
            account: None,
            layer: Some("fact".to_string()),
            corr_account: None,
        };

        let sql = build_matched_gl_cte(&query, &mut params);

        assert!(sql.contains("COALESCE(gl.amount, 0.0) AS signed_amount"));
        assert!(sql.contains("gl.resource_sign"));
        assert!(sql.contains("1.0 AS detail_sign_factor"));
        assert!(!sql.contains("SUM(gl.amount)"));
    }

    #[test]
    fn row_sign_factor_expr_uses_account_side_as_sign() {
        let expr = build_row_sign_factor_expr("gl", true);
        assert!(expr.contains("CASE WHEN gl.debit_account = ? THEN 1.0"));
        assert!(expr.contains("WHEN gl.credit_account = ? THEN -1.0"));
    }

    #[test]
    fn p903_detail_source_sql_uses_union_all_per_identity_path() {
        let query = GlDrilldownQuery {
            turnover_code: "mp_penalty".to_string(),
            group_by: "nomenclature".to_string(),
            date_from: "2026-02-01".to_string(),
            date_to: "2026-02-28".to_string(),
            connection_mp_ref: None,
            connection_mp_refs: vec![],
            account: None,
            layer: Some("fact".to_string()),
            corr_account: None,
        };

        let sql = build_detail_source_sql("p903_wb_finance_report", &query);

        assert!(sql.contains("UNION ALL"));
        assert!(sql.contains("d.id = gl_row.registrator_ref"));
        assert!(sql.contains("d.source_row_ref = gl_row.registrator_ref"));
        assert!(sql.contains("d.rr_dt = SUBSTR(gl_row.registrator_ref, 6, 10)"));
        assert!(sql.contains("d.rrd_id = CAST(SUBSTR(gl_row.registrator_ref, 17) AS INTEGER)"));
        assert!(sql.contains("gl_row.registrator_ref LIKE 'p903:%:%'"));
    }

    #[test]
    fn p911_detail_source_sql_uses_projection_amounts_and_distinct_gl_count() {
        let query = GlDrilldownQuery {
            turnover_code: "advertising_allocated".to_string(),
            group_by: "nomenclature".to_string(),
            date_from: "2026-02-01".to_string(),
            date_to: "2026-02-28".to_string(),
            connection_mp_ref: None,
            connection_mp_refs: vec![],
            account: None,
            layer: Some("oper".to_string()),
            corr_account: None,
        };

        let sql = build_detail_source_sql("p911_wb_advert_by_items", &query);

        assert!(sql.contains("INNER JOIN p911_wb_advert_by_items d ON d.general_ledger_ref = gl.id"));
        assert!(sql.contains("SUM(COALESCE(d.amount, 0.0) * COALESCE(gl.resource_sign, 1) * COALESCE(gl.detail_sign_factor, 1.0))"));
        assert!(sql.contains("COUNT(DISTINCT gl.id) AS entry_count"));
        assert!(!sql.contains("SUM(gl.signed_amount)"));
    }
}
