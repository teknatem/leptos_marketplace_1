use anyhow::Result;
use chrono::Utc;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ColumnTrait, EntityTrait, FromQueryResult, QueryFilter, QueryOrder, QuerySelect, Set};
use serde::{Deserialize, Serialize};

use crate::shared::data::db::get_connection;

/// SeaORM entity for p908_wb_goods_prices
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "p908_wb_goods_prices")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub nm_id: i64,
    pub connection_mp_ref: String,
    #[sea_orm(nullable)]
    pub vendor_code: Option<String>,
    #[sea_orm(nullable)]
    pub discount: Option<i32>,
    pub editable_size_price: i32,
    #[sea_orm(nullable)]
    pub price: Option<f64>,
    #[sea_orm(nullable)]
    pub discounted_price: Option<f64>,
    pub sizes_json: String,
    pub fetched_at: String,
    /// Resolved UUID from a004_nomenclature (base_nomenclature_ref or own id)
    #[sea_orm(nullable)]
    pub ext_nomenklature_ref: Option<String>,
    /// Dealer price from p906_nomenclature_prices
    #[sea_orm(nullable)]
    pub dealer_price_ut: Option<f64>,
    /// Margin: (discounted_price - dealer_price_ut) / dealer_price_ut * 100
    #[sea_orm(nullable)]
    pub margin_pro: Option<f64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// Entry struct for upsert operations
#[derive(Debug, Clone)]
pub struct WbGoodsPriceEntry {
    pub nm_id: i64,
    pub connection_mp_ref: String,
    pub vendor_code: Option<String>,
    pub discount: Option<i32>,
    pub editable_size_price: bool,
    pub price: Option<f64>,
    pub discounted_price: Option<f64>,
    pub sizes_json: String,
    pub ext_nomenklature_ref: Option<String>,
    pub dealer_price_ut: Option<f64>,
    pub margin_pro: Option<f64>,
}

/// Row returned from list_with_filters (includes JOINed names)
#[derive(Debug, Clone, FromQueryResult)]
pub struct WbGoodsPriceRow {
    pub nm_id: i64,
    pub connection_mp_ref: String,
    pub vendor_code: Option<String>,
    pub discount: Option<i32>,
    pub editable_size_price: i32,
    pub price: Option<f64>,
    pub discounted_price: Option<f64>,
    pub sizes_json: String,
    pub fetched_at: String,
    pub ext_nomenklature_ref: Option<String>,
    pub dealer_price_ut: Option<f64>,
    pub margin_pro: Option<f64>,
    pub nomenclature_name: Option<String>,
    pub connection_name: Option<String>,
}

/// Upsert entry using INSERT ... ON CONFLICT(nm_id) DO UPDATE SET ...
pub async fn upsert_entry(entry: &WbGoodsPriceEntry) -> Result<()> {
    let db = get_connection();
    let fetched_at = Utc::now().to_rfc3339();

    let model = ActiveModel {
        nm_id: Set(entry.nm_id),
        connection_mp_ref: Set(entry.connection_mp_ref.clone()),
        vendor_code: Set(entry.vendor_code.clone()),
        discount: Set(entry.discount),
        editable_size_price: Set(if entry.editable_size_price { 1 } else { 0 }),
        price: Set(entry.price),
        discounted_price: Set(entry.discounted_price),
        sizes_json: Set(entry.sizes_json.clone()),
        fetched_at: Set(fetched_at),
        ext_nomenklature_ref: Set(entry.ext_nomenklature_ref.clone()),
        dealer_price_ut: Set(entry.dealer_price_ut),
        margin_pro: Set(entry.margin_pro),
    };

    Entity::insert(model)
        .on_conflict(
            OnConflict::column(Column::NmId)
                .update_columns([
                    Column::ConnectionMpRef,
                    Column::VendorCode,
                    Column::Discount,
                    Column::EditableSizePrice,
                    Column::Price,
                    Column::DiscountedPrice,
                    Column::SizesJson,
                    Column::FetchedAt,
                    Column::ExtNomenklatureRef,
                    Column::DealerPriceUt,
                    Column::MarginPro,
                ])
                .to_owned(),
        )
        .exec(db)
        .await?;

    Ok(())
}

/// List entries with filters, pagination and JOINed names
pub async fn list_with_filters(
    connection_mp_ref: Option<String>,
    vendor_code: Option<String>,
    search: Option<String>,
    sort_by: &str,
    sort_desc: bool,
    limit: i32,
    offset: i32,
) -> Result<(Vec<WbGoodsPriceRow>, i32)> {
    use sea_orm::{ConnectionTrait, Statement};
    let db = get_connection();

    const MAX_LIMIT: i32 = 5000;
    const MAX_TOTAL_COUNT: i32 = 50_000;

    let safe_limit = limit.max(1).min(MAX_LIMIT);
    let safe_offset = offset.max(0);

    // Build WHERE clauses
    let mut where_clauses: Vec<&str> = vec![];
    let mut params: Vec<sea_orm::Value> = vec![];

    if let Some(ref conn) = connection_mp_ref {
        if !conn.is_empty() {
            where_clauses.push("p.connection_mp_ref = ?");
            params.push(conn.clone().into());
        }
    }
    if let Some(ref vc) = vendor_code {
        if !vc.is_empty() {
            where_clauses.push("p.vendor_code LIKE ?");
            params.push(format!("%{}%", vc).into());
        }
    }
    if let Some(ref s) = search {
        if !s.is_empty() {
            if let Ok(nm_id_val) = s.parse::<i64>() {
                where_clauses.push("p.nm_id = ?");
                params.push(nm_id_val.into());
            } else {
                where_clauses.push("p.vendor_code LIKE ?");
                params.push(format!("%{}%", s).into());
            }
        }
    }

    let where_sql = if where_clauses.is_empty() {
        "1=1".to_string()
    } else {
        where_clauses.join(" AND ")
    };

    // Count query
    let count_sql = format!(
        "SELECT COUNT(*) as count
         FROM p908_wb_goods_prices p
         LEFT JOIN a004_nomenclature n ON p.ext_nomenklature_ref = n.id
         LEFT JOIN a006_connection_mp c ON p.connection_mp_ref = c.id
         WHERE {}",
        where_sql
    );

    #[derive(Debug, FromQueryResult)]
    struct CountResult {
        count: i64,
    }

    let count_stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        &count_sql,
        params.iter().cloned(),
    );

    let count_result = CountResult::find_by_statement(count_stmt)
        .one(db)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Failed to get count"))?;

    let total_count = (count_result.count as i32).min(MAX_TOTAL_COUNT);

    // ORDER BY whitelist
    let order_col = match sort_by {
        "vendor_code" => "p.vendor_code",
        "price" => "p.price",
        "discounted_price" => "p.discounted_price",
        "discount" => "p.discount",
        "dealer_price_ut" => "p.dealer_price_ut",
        "margin_pro" => "p.margin_pro",
        "fetched_at" => "p.fetched_at",
        "nomenclature_name" => "n.description",
        "connection_name" => "c.description",
        _ => "p.nm_id",
    };
    let order_dir = if sort_desc { "DESC" } else { "ASC" };

    // Main query with JOINs
    let data_sql = format!(
        "SELECT
            p.nm_id,
            p.connection_mp_ref,
            p.vendor_code,
            p.discount,
            p.editable_size_price,
            p.price,
            p.discounted_price,
            p.sizes_json,
            p.fetched_at,
            p.ext_nomenklature_ref,
            p.dealer_price_ut,
            p.margin_pro,
            n.description AS nomenclature_name,
            c.description AS connection_name
         FROM p908_wb_goods_prices p
         LEFT JOIN a004_nomenclature n ON p.ext_nomenklature_ref = n.id
         LEFT JOIN a006_connection_mp c ON p.connection_mp_ref = c.id
         WHERE {}
         ORDER BY {} {} NULLS LAST
         LIMIT ? OFFSET ?",
        where_sql, order_col, order_dir
    );

    let mut data_params = params.clone();
    data_params.push(safe_limit.into());
    data_params.push(safe_offset.into());

    let data_stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        &data_sql,
        data_params,
    );

    let items = WbGoodsPriceRow::find_by_statement(data_stmt)
        .all(db)
        .await?;

    Ok((items, total_count))
}

/// Get single entry by nm_id
pub async fn get_by_nm_id(nm_id: i64) -> Result<Option<Model>> {
    let db = get_connection();
    Ok(Entity::find_by_id(nm_id).one(db).await?)
}

/// Delete all entries for a given connection
pub async fn delete_by_connection(connection_mp_ref: &str) -> Result<u64> {
    let db = get_connection();
    let result = Entity::delete_many()
        .filter(Column::ConnectionMpRef.eq(connection_mp_ref))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}
