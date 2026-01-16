use anyhow::Result;
use sea_orm::{FromQueryResult, Statement};
use serde::{Deserialize, Serialize};

use crate::shared::data::db::get_connection;

/// Raw aggregation result from SQL query
#[derive(Debug, Clone, Serialize, Deserialize, FromQueryResult)]
pub struct RevenueAggregation {
    pub marketplace_code: Option<String>, // "WB", "OZON", "YM"
    pub organization_name: Option<String>, // Directly from conn.organization
    pub total_revenue: f64,
}

/// Returns aggregation result from SQL query
#[derive(Debug, Clone, Serialize, Deserialize, FromQueryResult)]
pub struct ReturnsAggregation {
    pub marketplace_code: Option<String>, // "WB", "OZON", "YM"
    pub organization_name: Option<String>, // Directly from conn.organization
    pub total_returns: f64,
}

/// Get revenue aggregated by marketplace type and organization for a given month
pub async fn get_revenue_by_marketplace_and_org(
    date_from: &str,
    date_to: &str,
) -> Result<Vec<RevenueAggregation>> {
    let db = get_connection();

    let sql = r#"
        SELECT 
            CASE mp.marketplace_type
                WHEN 'mp-wb' THEN 'WB'
                WHEN 'mp-ozon' THEN 'OZON'
                WHEN 'mp-ym' THEN 'YM'
                WHEN 'mp-kuper' THEN 'KUPER'
                WHEN 'mp-lemana' THEN 'LEMANA'
                ELSE mp.marketplace_type
            END AS marketplace_code,
            conn.organization AS organization_name,
            COALESCE(SUM(p904.customer_in), 0) AS total_revenue
        FROM p904_sales_data p904
        LEFT JOIN a006_connection_mp conn ON p904.connection_mp_ref = conn.id
        LEFT JOIN a005_marketplace mp ON conn.marketplace = mp.id
        WHERE p904.date >= ? AND p904.date <= ?
        GROUP BY mp.marketplace_type, conn.organization
        ORDER BY marketplace_code, organization_name
    "#;

    let stmt = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Sqlite,
        sql,
        [date_from.into(), date_to.into()],
    );

    let results = RevenueAggregation::find_by_statement(stmt).all(db).await?;

    Ok(results)
}

/// Get returns aggregated by marketplace type and organization for a given month
pub async fn get_returns_by_marketplace_and_org(
    date_from: &str,
    date_to: &str,
) -> Result<Vec<ReturnsAggregation>> {
    let db = get_connection();

    let sql = r#"
        SELECT 
            CASE mp.marketplace_type
                WHEN 'mp-wb' THEN 'WB'
                WHEN 'mp-ozon' THEN 'OZON'
                WHEN 'mp-ym' THEN 'YM'
                WHEN 'mp-kuper' THEN 'KUPER'
                WHEN 'mp-lemana' THEN 'LEMANA'
                ELSE mp.marketplace_type
            END AS marketplace_code,
            conn.organization AS organization_name,
            COALESCE(SUM(p904.customer_out), 0) AS total_returns
        FROM p904_sales_data p904
        LEFT JOIN a006_connection_mp conn ON p904.connection_mp_ref = conn.id
        LEFT JOIN a005_marketplace mp ON conn.marketplace = mp.id
        WHERE p904.date >= ? AND p904.date <= ?
        GROUP BY mp.marketplace_type, conn.organization
        ORDER BY marketplace_code, organization_name
    "#;

    let stmt = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Sqlite,
        sql,
        [date_from.into(), date_to.into()],
    );

    let results = ReturnsAggregation::find_by_statement(stmt).all(db).await?;

    Ok(results)
}

/// Get distinct available periods (YYYY-MM) from p904_sales_data
pub async fn get_available_periods() -> Result<Vec<String>> {
    let db = get_connection();

    let sql = r#"
        SELECT DISTINCT SUBSTR(p904.date, 1, 7) AS period
        FROM p904_sales_data p904
        WHERE p904.date IS NOT NULL AND p904.date != ''
        ORDER BY period DESC
    "#;

    #[derive(Debug, FromQueryResult)]
    struct PeriodRow {
        period: Option<String>,
    }

    let stmt = Statement::from_sql_and_values(sea_orm::DatabaseBackend::Sqlite, sql, []);
    let results = PeriodRow::find_by_statement(stmt).all(db).await?;

    Ok(results.into_iter().filter_map(|r| r.period).collect())
}

/// Get list of all marketplace types that have data
pub async fn get_active_marketplaces() -> Result<Vec<String>> {
    let db = get_connection();

    let sql = r#"
        SELECT DISTINCT mp.marketplace_type
        FROM a006_connection_mp conn
        JOIN a005_marketplace mp ON conn.marketplace = mp.id
        WHERE mp.marketplace_type IS NOT NULL
        ORDER BY mp.marketplace_type
    "#;

    #[derive(Debug, FromQueryResult)]
    struct MpType {
        marketplace_type: Option<String>,
    }

    let stmt = Statement::from_sql_and_values(sea_orm::DatabaseBackend::Sqlite, sql, []);
    let results = MpType::find_by_statement(stmt).all(db).await?;

    let types: Vec<String> = results
        .into_iter()
        .filter_map(|r| r.marketplace_type)
        .collect();

    Ok(types)
}

/// Get list of organizations that have sales data
pub async fn get_organizations_with_sales(date_from: &str, date_to: &str) -> Result<Vec<String>> {
    let db = get_connection();

    let sql = r#"
        SELECT DISTINCT 
            conn.organization AS organization_name
        FROM p904_sales_data p904
        JOIN a006_connection_mp conn ON p904.connection_mp_ref = conn.id
        WHERE p904.date >= ? AND p904.date <= ?
            AND conn.organization IS NOT NULL
            AND conn.organization != ''
        ORDER BY conn.organization
    "#;

    #[derive(Debug, FromQueryResult)]
    struct OrgInfo {
        organization_name: Option<String>,
    }

    let stmt = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Sqlite,
        sql,
        [date_from.into(), date_to.into()],
    );

    let results = OrgInfo::find_by_statement(stmt).all(db).await?;

    let orgs: Vec<String> = results
        .into_iter()
        .filter_map(|r| r.organization_name)
        .collect();

    Ok(orgs)
}

