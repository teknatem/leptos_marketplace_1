use anyhow::Result;
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set};
use serde::{Deserialize, Serialize};

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "p904_sales_data")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    // Technical fields
    pub registrator_ref: String,
    pub registrator_type: String,

    // Dimensions
    pub date: String,
    pub connection_mp_ref: String,
    pub nomenclature_ref: String,
    pub marketplace_product_ref: String,

    // Sums
    pub customer_in: f64,
    pub customer_out: f64,
    pub coinvest_in: f64,
    pub commission_out: f64,
    pub acquiring_out: f64,
    pub penalty_out: f64,
    pub logistics_out: f64,
    pub seller_out: f64,
    pub price_full: f64,
    pub price_list: f64,
    pub price_return: f64,
    pub commission_percent: f64,
    pub coinvest_persent: f64,
    pub total: f64,
    #[sea_orm(nullable)]
    pub cost: Option<f64>,

    // Info fields
    pub document_no: String,
    pub article: String,
    pub posted_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

pub async fn upsert_entry(entry: &Model) -> Result<()> {
    let active = ActiveModel {
        id: Set(entry.id.clone()),
        registrator_ref: Set(entry.registrator_ref.clone()),
        registrator_type: Set(entry.registrator_type.clone()),
        date: Set(entry.date.clone()),
        connection_mp_ref: Set(entry.connection_mp_ref.clone()),
        nomenclature_ref: Set(entry.nomenclature_ref.clone()),
        marketplace_product_ref: Set(entry.marketplace_product_ref.clone()),
        customer_in: Set(entry.customer_in),
        customer_out: Set(entry.customer_out),
        coinvest_in: Set(entry.coinvest_in),
        commission_out: Set(entry.commission_out),
        acquiring_out: Set(entry.acquiring_out),
        penalty_out: Set(entry.penalty_out),
        logistics_out: Set(entry.logistics_out),
        seller_out: Set(entry.seller_out),
        price_full: Set(entry.price_full),
        price_list: Set(entry.price_list),
        price_return: Set(entry.price_return),
        commission_percent: Set(entry.commission_percent),
        coinvest_persent: Set(entry.coinvest_persent),
        total: Set(entry.total),
        cost: Set(entry.cost),
        document_no: Set(entry.document_no.clone()),
        article: Set(entry.article.clone()),
        posted_at: Set(entry.posted_at.clone()),
    };

    // Using insert with on_conflict would be better, but for now simple insert/update logic
    // Since we generate UUIDs for ID, we can just insert.
    // If we want idempotency based on business keys, we should check first.
    // For now, let's assume we delete by registrator before inserting, so insert is safe.

    Entity::insert(active).exec(conn()).await?;

    Ok(())
}

pub async fn get_by_registrator(registrator_ref: &str) -> Result<Vec<Model>> {
    let items = Entity::find()
        .filter(Column::RegistratorRef.eq(registrator_ref))
        .all(conn())
        .await?;
    Ok(items)
}

pub async fn delete_by_registrator(registrator_ref: &str) -> Result<u64> {
    let result = Entity::delete_many()
        .filter(Column::RegistratorRef.eq(registrator_ref))
        .exec(conn())
        .await?;
    Ok(result.rows_affected)
}

pub async fn list(limit: Option<u64>) -> Result<Vec<Model>> {
    let mut query = Entity::find().order_by_desc(Column::Date);

    if let Some(lim) = limit {
        query = query.limit(lim);
    }

    let items = query.all(conn()).await?;
    Ok(items)
}

/// Enhanced model that includes cabinet name from connection_mp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelWithCabinet {
    #[serde(flatten)]
    pub base: Model,
    pub connection_mp_name: Option<String>,
}

/// List with filters: date range and connection_mp_ref
pub async fn list_with_filters(
    date_from: Option<String>,
    date_to: Option<String>,
    connection_mp_ref: Option<String>,
    limit: Option<u64>,
) -> Result<Vec<ModelWithCabinet>> {
    use sea_orm::{FromQueryResult, Statement};

    // Build SQL query manually for better control
    let mut sql = r#"
        SELECT 
            p904.id,
            p904.registrator_ref,
            p904.registrator_type,
            p904.date,
            p904.connection_mp_ref,
            p904.nomenclature_ref,
            p904.marketplace_product_ref,
            p904.customer_in,
            p904.customer_out,
            p904.coinvest_in,
            p904.commission_out,
            p904.acquiring_out,
            p904.penalty_out,
            p904.logistics_out,
            p904.seller_out,
            p904.price_full,
            p904.price_list,
            p904.price_return,
            p904.commission_percent,
            p904.coinvest_persent,
            p904.total,
            p904.cost,
            p904.document_no,
            p904.article,
            p904.posted_at,
            conn.description as connection_mp_name
        FROM p904_sales_data p904
        LEFT JOIN a006_connection_mp conn ON p904.connection_mp_ref = conn.id
        WHERE 1=1
    "#
    .to_string();

    let mut params: Vec<sea_orm::Value> = vec![];

    // Add date filters
    if let Some(from) = &date_from {
        sql.push_str(&format!(" AND p904.date >= ?"));
        params.push(from.clone().into());
    }
    if let Some(to) = &date_to {
        sql.push_str(&format!(" AND p904.date <= ?"));
        params.push(to.clone().into());
    }

    // Add connection_mp filter
    if let Some(conn_ref) = &connection_mp_ref {
        sql.push_str(&format!(" AND p904.connection_mp_ref = ?"));
        params.push(conn_ref.clone().into());
    }

    // Add ordering and limit
    sql.push_str(" ORDER BY p904.date DESC");
    if let Some(lim) = limit {
        tracing::info!("P904 repository: applying LIMIT {}", lim);
        sql.push_str(&format!(" LIMIT {}", lim));
    } else {
        tracing::warn!("P904 repository: NO LIMIT specified, this could return ALL records");
    }

    // Define a struct for query results
    #[derive(Debug, FromQueryResult)]
    struct QueryResult {
        id: String,
        registrator_ref: String,
        registrator_type: String,
        date: String,
        connection_mp_ref: String,
        nomenclature_ref: String,
        marketplace_product_ref: String,
        customer_in: f64,
        customer_out: f64,
        coinvest_in: f64,
        commission_out: f64,
        acquiring_out: f64,
        penalty_out: f64,
        logistics_out: f64,
        seller_out: f64,
        price_full: f64,
        price_list: f64,
        price_return: f64,
        commission_percent: f64,
        coinvest_persent: f64,
        total: f64,
        cost: Option<f64>,
        document_no: String,
        article: String,
        posted_at: String,
        connection_mp_name: Option<String>,
    }

    let stmt = Statement::from_sql_and_values(sea_orm::DatabaseBackend::Sqlite, &sql, params);

    let results = QueryResult::find_by_statement(stmt).all(conn()).await?;

    let items = results
        .into_iter()
        .map(|r| ModelWithCabinet {
            base: Model {
                id: r.id,
                registrator_ref: r.registrator_ref,
                registrator_type: r.registrator_type,
                date: r.date,
                connection_mp_ref: r.connection_mp_ref,
                nomenclature_ref: r.nomenclature_ref,
                marketplace_product_ref: r.marketplace_product_ref,
                customer_in: r.customer_in,
                customer_out: r.customer_out,
                coinvest_in: r.coinvest_in,
                commission_out: r.commission_out,
                acquiring_out: r.acquiring_out,
                penalty_out: r.penalty_out,
                logistics_out: r.logistics_out,
                seller_out: r.seller_out,
                price_full: r.price_full,
                price_list: r.price_list,
                price_return: r.price_return,
                commission_percent: r.commission_percent,
                coinvest_persent: r.coinvest_persent,
                total: r.total,
                cost: r.cost,
                document_no: r.document_no,
                article: r.article,
                posted_at: r.posted_at,
            },
            connection_mp_name: r.connection_mp_name,
        })
        .collect();

    Ok(items)
}
