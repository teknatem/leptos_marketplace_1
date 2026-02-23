use anyhow::Result;
use chrono::Utc;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set};
use serde::{Deserialize, Serialize};

use crate::shared::data::db::get_connection;

/// SeaORM entity for p907_ym_payment_report
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "p907_ym_payment_report")]
pub struct Model {
    /// Internal stable primary key:
    /// - real `transaction_id` from YM when available
    /// - `SYNTH_{...}` synthetic key when YM leaves transaction_id empty
    #[sea_orm(primary_key, auto_increment = false)]
    pub record_key: String,

    // Metadata
    pub connection_mp_ref: String,
    pub organization_ref: String,

    // Business info
    #[sea_orm(nullable)]
    pub business_id: Option<i64>,
    #[sea_orm(nullable)]
    pub partner_id: Option<i64>,
    #[sea_orm(nullable)]
    pub shop_name: Option<String>,
    #[sea_orm(nullable)]
    pub inn: Option<String>,
    #[sea_orm(nullable)]
    pub model: Option<String>,

    // Transaction info
    #[sea_orm(nullable)]
    pub transaction_id: Option<String>, // real YM transaction ID (nullable)
    #[sea_orm(nullable)]
    pub transaction_date: Option<String>,
    #[sea_orm(nullable)]
    pub transaction_type: Option<String>,
    #[sea_orm(nullable)]
    pub transaction_source: Option<String>,
    #[sea_orm(nullable)]
    pub transaction_sum: Option<f64>,
    #[sea_orm(nullable)]
    pub payment_status: Option<String>,

    // Order info
    #[sea_orm(nullable)]
    pub order_id: Option<i64>,
    #[sea_orm(nullable)]
    pub shop_order_id: Option<String>,
    #[sea_orm(nullable)]
    pub order_creation_date: Option<String>,
    #[sea_orm(nullable)]
    pub order_delivery_date: Option<String>,
    #[sea_orm(nullable)]
    pub order_type: Option<String>,

    // Product/service info
    #[sea_orm(nullable)]
    pub shop_sku: Option<String>,
    #[sea_orm(nullable)]
    pub offer_or_service_name: Option<String>,
    #[sea_orm(nullable)]
    pub count: Option<i32>,

    // Bank / Act info
    #[sea_orm(nullable)]
    pub act_id: Option<i64>,
    #[sea_orm(nullable)]
    pub act_date: Option<String>,
    #[sea_orm(nullable)]
    pub bank_order_id: Option<i64>,
    #[sea_orm(nullable)]
    pub bank_order_date: Option<String>,
    #[sea_orm(nullable)]
    pub bank_sum: Option<f64>,

    // Extra
    #[sea_orm(nullable)]
    pub claim_number: Option<String>,
    #[sea_orm(nullable)]
    pub bonus_account_year_month: Option<String>,
    #[sea_orm(nullable)]
    pub comments: Option<String>,

    // Technical
    pub loaded_at_utc: String,
    pub payload_version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// Entry struct for upsert operations
#[derive(Debug, Clone)]
pub struct YmPaymentReportEntry {
    /// Internal stable primary key (real transaction_id or synthetic SYNTH_...)
    pub record_key: String,
    pub connection_mp_ref: String,
    pub organization_ref: String,
    pub business_id: Option<i64>,
    pub partner_id: Option<i64>,
    pub shop_name: Option<String>,
    pub inn: Option<String>,
    pub model: Option<String>,
    /// Real YM transaction ID from CSV — None when YM leaves it empty
    pub transaction_id: Option<String>,
    pub transaction_date: Option<String>,
    pub transaction_type: Option<String>,
    pub transaction_source: Option<String>,
    pub transaction_sum: Option<f64>,
    pub payment_status: Option<String>,
    pub order_id: Option<i64>,
    pub shop_order_id: Option<String>,
    pub order_creation_date: Option<String>,
    pub order_delivery_date: Option<String>,
    pub order_type: Option<String>,
    pub shop_sku: Option<String>,
    pub offer_or_service_name: Option<String>,
    pub count: Option<i32>,
    pub act_id: Option<i64>,
    pub act_date: Option<String>,
    pub bank_order_id: Option<i64>,
    pub bank_order_date: Option<String>,
    pub bank_sum: Option<f64>,
    pub claim_number: Option<String>,
    pub bonus_account_year_month: Option<String>,
    pub comments: Option<String>,
    pub payload_version: i32,
}

/// Upsert entry using INSERT ... ON CONFLICT(record_key) DO UPDATE SET ...
/// No pre-SELECT needed; conflict resolution is handled atomically by SQLite.
pub async fn upsert_entry(entry: &YmPaymentReportEntry) -> Result<()> {
    let db = get_connection();
    let loaded_at_utc = Utc::now().to_rfc3339();

    let model = ActiveModel {
        record_key: Set(entry.record_key.clone()),
        connection_mp_ref: Set(entry.connection_mp_ref.clone()),
        organization_ref: Set(entry.organization_ref.clone()),
        business_id: Set(entry.business_id),
        partner_id: Set(entry.partner_id),
        shop_name: Set(entry.shop_name.clone()),
        inn: Set(entry.inn.clone()),
        model: Set(entry.model.clone()),
        transaction_id: Set(entry.transaction_id.clone()),
        transaction_date: Set(entry.transaction_date.clone()),
        transaction_type: Set(entry.transaction_type.clone()),
        transaction_source: Set(entry.transaction_source.clone()),
        transaction_sum: Set(entry.transaction_sum),
        payment_status: Set(entry.payment_status.clone()),
        order_id: Set(entry.order_id),
        shop_order_id: Set(entry.shop_order_id.clone()),
        order_creation_date: Set(entry.order_creation_date.clone()),
        order_delivery_date: Set(entry.order_delivery_date.clone()),
        order_type: Set(entry.order_type.clone()),
        shop_sku: Set(entry.shop_sku.clone()),
        offer_or_service_name: Set(entry.offer_or_service_name.clone()),
        count: Set(entry.count),
        act_id: Set(entry.act_id),
        act_date: Set(entry.act_date.clone()),
        bank_order_id: Set(entry.bank_order_id),
        bank_order_date: Set(entry.bank_order_date.clone()),
        bank_sum: Set(entry.bank_sum),
        claim_number: Set(entry.claim_number.clone()),
        bonus_account_year_month: Set(entry.bonus_account_year_month.clone()),
        comments: Set(entry.comments.clone()),
        loaded_at_utc: Set(loaded_at_utc),
        payload_version: Set(entry.payload_version),
    };

    Entity::insert(model)
        .on_conflict(
            OnConflict::column(Column::RecordKey)
                .update_columns([
                    Column::ConnectionMpRef,
                    Column::OrganizationRef,
                    Column::BusinessId,
                    Column::PartnerId,
                    Column::ShopName,
                    Column::Inn,
                    Column::Model,
                    Column::TransactionId,
                    Column::TransactionDate,
                    Column::TransactionType,
                    Column::TransactionSource,
                    Column::TransactionSum,
                    Column::PaymentStatus,
                    Column::OrderId,
                    Column::ShopOrderId,
                    Column::OrderCreationDate,
                    Column::OrderDeliveryDate,
                    Column::OrderType,
                    Column::ShopSku,
                    Column::OfferOrServiceName,
                    Column::Count,
                    Column::ActId,
                    Column::ActDate,
                    Column::BankOrderId,
                    Column::BankOrderDate,
                    Column::BankSum,
                    Column::ClaimNumber,
                    Column::BonusAccountYearMonth,
                    Column::Comments,
                    Column::LoadedAtUtc,
                    Column::PayloadVersion,
                ])
                .to_owned(),
        )
        .exec(db)
        .await?;

    Ok(())
}

/// List entries with filters and pagination
pub async fn list_with_filters(
    date_from: &str,
    date_to: &str,
    transaction_type: Option<String>,
    payment_status: Option<String>,
    shop_sku: Option<String>,
    order_id: Option<i64>,
    connection_mp_ref: Option<String>,
    organization_ref: Option<String>,
    sort_by: &str,
    sort_desc: bool,
    limit: i32,
    offset: i32,
) -> Result<(Vec<Model>, i32)> {
    let db = get_connection();
    const MAX_LIMIT: i32 = 1000;
    const MAX_TOTAL_COUNT: i32 = 10_000;
    const COUNT_SCAN_LIMIT: u64 = (MAX_TOTAL_COUNT as u64) + 1;

    let safe_limit = limit.max(1).min(MAX_LIMIT);
    let safe_offset = offset.max(0);

    let apply_filters = |mut q: sea_orm::Select<Entity>| -> sea_orm::Select<Entity> {
        if !date_from.is_empty() {
            q = q.filter(Column::TransactionDate.gte(date_from));
        }
        if !date_to.is_empty() {
            q = q.filter(Column::TransactionDate.lte(date_to));
        }
        if let Some(ref tt) = transaction_type {
            if !tt.is_empty() {
                q = q.filter(Column::TransactionType.eq(tt));
            }
        }
        if let Some(ref ps) = payment_status {
            if !ps.is_empty() {
                q = q.filter(Column::PaymentStatus.eq(ps));
            }
        }
        if let Some(ref sku) = shop_sku {
            if !sku.is_empty() {
                q = q.filter(Column::ShopSku.contains(sku));
            }
        }
        if let Some(oid) = order_id {
            q = q.filter(Column::OrderId.eq(oid));
        }
        if let Some(ref conn) = connection_mp_ref {
            q = q.filter(Column::ConnectionMpRef.eq(conn));
        }
        if let Some(ref org) = organization_ref {
            q = q.filter(Column::OrganizationRef.eq(org));
        }
        q
    };

    let total_count = apply_filters(Entity::find())
        .select_only()
        .column(Column::RecordKey)
        .limit(COUNT_SCAN_LIMIT)
        .into_tuple::<String>()
        .all(db)
        .await?
        .len() as i32;
    let total_count = total_count.min(MAX_TOTAL_COUNT);

    let mut query = apply_filters(Entity::find());

    query = match sort_by {
        "transaction_date" => {
            if sort_desc {
                query.order_by_desc(Column::TransactionDate)
            } else {
                query.order_by_asc(Column::TransactionDate)
            }
        }
        "transaction_sum" => {
            if sort_desc {
                query.order_by_desc(Column::TransactionSum)
            } else {
                query.order_by_asc(Column::TransactionSum)
            }
        }
        "order_id" => {
            if sort_desc {
                query.order_by_desc(Column::OrderId)
            } else {
                query.order_by_asc(Column::OrderId)
            }
        }
        "transaction_type" => {
            if sort_desc {
                query.order_by_desc(Column::TransactionType)
            } else {
                query.order_by_asc(Column::TransactionType)
            }
        }
        "bank_sum" => {
            if sort_desc {
                query.order_by_desc(Column::BankSum)
            } else {
                query.order_by_asc(Column::BankSum)
            }
        }
        _ => {
            if sort_desc {
                query.order_by_desc(Column::TransactionDate)
            } else {
                query.order_by_asc(Column::TransactionDate)
            }
        }
    };

    let items = query
        .limit(safe_limit as u64)
        .offset(safe_offset as u64)
        .all(db)
        .await?;

    Ok((items, total_count))
}

/// Get a single entry by record_key
pub async fn get_by_id(record_key: &str) -> Result<Option<Model>> {
    let db = get_connection();
    let item = Entity::find_by_id(record_key).one(db).await?;
    Ok(item)
}

/// Delete all entries for a connection in a date range (for re-import)
pub async fn delete_by_connection_and_date_range(
    connection_mp_ref: &str,
    date_from: &str,
    date_to: &str,
) -> Result<u64> {
    let db = get_connection();
    let result = Entity::delete_many()
        .filter(Column::ConnectionMpRef.eq(connection_mp_ref))
        .filter(Column::TransactionDate.gte(date_from))
        .filter(Column::TransactionDate.lte(date_to))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}
