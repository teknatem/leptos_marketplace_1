use anyhow::Result;
use sea_orm::{FromQueryResult, Statement};
use serde::{Deserialize, Serialize};

use crate::shared::data::db::get_connection;

/// Raw aggregation result from SQL query
#[derive(Debug, Clone, Serialize, Deserialize, FromQueryResult)]
pub struct RevenueAggregation {
    pub marketplace_type: Option<String>,
    pub organization_ref: Option<String>,
    pub organization_name: Option<String>,
    pub total_revenue: f64,
}

/// Get revenue aggregated by marketplace type and organization for a given month
pub async fn get_revenue_by_marketplace_and_org(
    date_from: &str,
    date_to: &str,
) -> Result<Vec<RevenueAggregation>> {
    let db = get_connection();

    let sql = r#"
        SELECT 
            mp.marketplace_type,
            conn.organization AS organization_ref,
            org.description AS organization_name,
            COALESCE(SUM(p904.customer_in), 0) AS total_revenue
        FROM p904_sales_data p904
        LEFT JOIN a006_connection_mp conn ON p904.connection_mp_ref = conn.id
        LEFT JOIN a005_marketplace mp ON conn.marketplace = mp.id
        LEFT JOIN a002_organization org ON conn.organization = org.id
        WHERE p904.date >= ? AND p904.date <= ?
        GROUP BY mp.marketplace_type, conn.organization
        ORDER BY mp.marketplace_type, org.description
    "#;

    let stmt = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Sqlite,
        sql,
        [date_from.into(), date_to.into()],
    );

    let results = RevenueAggregation::find_by_statement(stmt).all(db).await?;

    Ok(results)
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
pub async fn get_organizations_with_sales(date_from: &str, date_to: &str) -> Result<Vec<(String, String)>> {
    let db = get_connection();

    let sql = r#"
        SELECT DISTINCT 
            conn.organization AS organization_ref,
            org.description AS organization_name
        FROM p904_sales_data p904
        JOIN a006_connection_mp conn ON p904.connection_mp_ref = conn.id
        JOIN a002_organization org ON conn.organization = org.id
        WHERE p904.date >= ? AND p904.date <= ?
            AND conn.organization IS NOT NULL
            AND conn.organization != ''
        ORDER BY org.description
    "#;

    #[derive(Debug, FromQueryResult)]
    struct OrgInfo {
        organization_ref: String,
        organization_name: Option<String>,
    }

    let stmt = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Sqlite,
        sql,
        [date_from.into(), date_to.into()],
    );

    let results = OrgInfo::find_by_statement(stmt).all(db).await?;

    let orgs: Vec<(String, String)> = results
        .into_iter()
        .map(|r| {
            (
                r.organization_ref,
                r.organization_name.unwrap_or_else(|| "Неизвестная организация".to_string()),
            )
        })
        .collect();

    Ok(orgs)
}

