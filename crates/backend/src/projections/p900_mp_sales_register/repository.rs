use anyhow::Result;
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set};
use serde::{Deserialize, Serialize};

use crate::shared::data::db::get_connection;

/// Модель Sales Register entry
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "p900_sales_register")]
pub struct Model {
    // NK (Natural Key): (marketplace, document_no, line_id)
    #[sea_orm(primary_key, auto_increment = false)]
    pub marketplace: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub document_no: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub line_id: String,

    // Metadata
    #[sea_orm(nullable)]
    pub scheme: Option<String>,
    pub document_type: String,
    pub document_version: i32,

    // References to aggregates (UUID)
    pub connection_mp_ref: String,
    pub organization_ref: String,
    #[sea_orm(nullable)]
    pub marketplace_product_ref: Option<String>,
    pub registrator_ref: String,

    // Timestamps and status
    pub event_time_source: String,
    pub sale_date: String,
    #[sea_orm(nullable)]
    pub source_updated_at: Option<String>,
    pub status_source: String,
    pub status_norm: String,

    // Product identification
    #[sea_orm(nullable)]
    pub seller_sku: Option<String>,
    pub mp_item_id: String,
    #[sea_orm(nullable)]
    pub barcode: Option<String>,
    #[sea_orm(nullable)]
    pub title: Option<String>,

    // Quantities and money
    pub qty: f64,
    #[sea_orm(nullable)]
    pub price_list: Option<f64>,
    #[sea_orm(nullable)]
    pub discount_total: Option<f64>,
    #[sea_orm(nullable)]
    pub price_effective: Option<f64>,
    #[sea_orm(nullable)]
    pub amount_line: Option<f64>,
    #[sea_orm(nullable)]
    pub currency_code: Option<String>,

    // Technical fields
    pub loaded_at_utc: String,
    pub payload_version: i32,
    #[sea_orm(nullable)]
    pub extra: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

/// Структура для передачи данных в upsert
#[derive(Debug, Clone)]
pub struct SalesRegisterEntry {
    // NK
    pub marketplace: String,
    pub document_no: String,
    pub line_id: String,

    // Metadata
    pub scheme: Option<String>,
    pub document_type: String,
    pub document_version: i32,

    // References to aggregates (UUID as String)
    pub connection_mp_ref: String,
    pub organization_ref: String,
    pub marketplace_product_ref: Option<String>,
    pub registrator_ref: String,

    // Timestamps and status
    pub event_time_source: DateTime<Utc>,
    pub sale_date: chrono::NaiveDate,
    pub source_updated_at: Option<DateTime<Utc>>,
    pub status_source: String,
    pub status_norm: String,

    // Product identification
    pub seller_sku: Option<String>,
    pub mp_item_id: String,
    pub barcode: Option<String>,
    pub title: Option<String>,

    // Quantities and money
    pub qty: f64,
    pub price_list: Option<f64>,
    pub discount_total: Option<f64>,
    pub price_effective: Option<f64>,
    pub amount_line: Option<f64>,
    pub currency_code: Option<String>,

    // Technical
    pub payload_version: i32,
    pub extra: Option<String>,
}

/// Upsert записи в sales_register по NK (marketplace, document_no, line_id)
pub async fn upsert_entry(entry: &SalesRegisterEntry) -> Result<()> {
    // Проверяем, существует ли запись
    let existing = Entity::find()
        .filter(Column::Marketplace.eq(&entry.marketplace))
        .filter(Column::DocumentNo.eq(&entry.document_no))
        .filter(Column::LineId.eq(&entry.line_id))
        .one(conn())
        .await?;

    let now = Utc::now();
    let event_time_str = entry.event_time_source.to_rfc3339();
    let sale_date_str = entry.sale_date.format("%Y-%m-%d").to_string();
    let source_updated_str = entry.source_updated_at.map(|dt| dt.to_rfc3339());

    if existing.is_some() {
        // Обновляем существующую запись
        let active = ActiveModel {
            marketplace: Set(entry.marketplace.clone()),
            document_no: Set(entry.document_no.clone()),
            line_id: Set(entry.line_id.clone()),
            scheme: Set(entry.scheme.clone()),
            document_type: Set(entry.document_type.clone()),
            document_version: Set(entry.document_version),
            connection_mp_ref: Set(entry.connection_mp_ref.clone()),
            organization_ref: Set(entry.organization_ref.clone()),
            marketplace_product_ref: Set(entry.marketplace_product_ref.clone()),
            registrator_ref: Set(entry.registrator_ref.clone()),
            event_time_source: Set(event_time_str),
            sale_date: Set(sale_date_str),
            source_updated_at: Set(source_updated_str),
            status_source: Set(entry.status_source.clone()),
            status_norm: Set(entry.status_norm.clone()),
            seller_sku: Set(entry.seller_sku.clone()),
            mp_item_id: Set(entry.mp_item_id.clone()),
            barcode: Set(entry.barcode.clone()),
            title: Set(entry.title.clone()),
            qty: Set(entry.qty),
            price_list: Set(entry.price_list),
            discount_total: Set(entry.discount_total),
            price_effective: Set(entry.price_effective),
            amount_line: Set(entry.amount_line),
            currency_code: Set(entry.currency_code.clone()),
            loaded_at_utc: Set(now.to_rfc3339()),
            payload_version: Set(entry.payload_version),
            extra: Set(entry.extra.clone()),
        };
        active.update(conn()).await?;
    } else {
        // Вставляем новую запись
        let active = ActiveModel {
            marketplace: Set(entry.marketplace.clone()),
            document_no: Set(entry.document_no.clone()),
            line_id: Set(entry.line_id.clone()),
            scheme: Set(entry.scheme.clone()),
            document_type: Set(entry.document_type.clone()),
            document_version: Set(entry.document_version),
            connection_mp_ref: Set(entry.connection_mp_ref.clone()),
            organization_ref: Set(entry.organization_ref.clone()),
            marketplace_product_ref: Set(entry.marketplace_product_ref.clone()),
            registrator_ref: Set(entry.registrator_ref.clone()),
            event_time_source: Set(event_time_str.clone()),
            sale_date: Set(sale_date_str.clone()),
            source_updated_at: Set(source_updated_str),
            status_source: Set(entry.status_source.clone()),
            status_norm: Set(entry.status_norm.clone()),
            seller_sku: Set(entry.seller_sku.clone()),
            mp_item_id: Set(entry.mp_item_id.clone()),
            barcode: Set(entry.barcode.clone()),
            title: Set(entry.title.clone()),
            qty: Set(entry.qty),
            price_list: Set(entry.price_list),
            discount_total: Set(entry.discount_total),
            price_effective: Set(entry.price_effective),
            amount_line: Set(entry.amount_line),
            currency_code: Set(entry.currency_code.clone()),
            loaded_at_utc: Set(now.to_rfc3339()),
            payload_version: Set(entry.payload_version),
            extra: Set(entry.extra.clone()),
        };
        active.insert(conn()).await?;
    }

    Ok(())
}

/// Получить список продаж (с опциональной фильтрацией)
pub async fn list_sales(limit: Option<u64>) -> Result<Vec<Model>> {
    let mut query = Entity::find();

    if let Some(lim) = limit {
        query = query.limit(lim);
    }

    let items = query.all(conn()).await?;
    Ok(items)
}

/// Получить записи по marketplace
pub async fn get_by_marketplace(marketplace: &str, limit: Option<u64>) -> Result<Vec<Model>> {
    let mut query = Entity::find().filter(Column::Marketplace.eq(marketplace));

    if let Some(lim) = limit {
        query = query.limit(lim);
    }

    let items = query.all(conn()).await?;
    Ok(items)
}
