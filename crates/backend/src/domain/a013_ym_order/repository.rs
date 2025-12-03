use anyhow::Result;
use chrono::Utc;
use contracts::domain::a013_ym_order::aggregate::{
    YmOrder, YmOrderId, YmOrderHeader, YmOrderLine, YmOrderState, YmOrderSourceMeta,
};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a013_ym_order")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub document_no: String,
    pub header_json: String,
    pub lines_json: String,
    pub state_json: String,
    pub source_meta_json: String,
    pub is_deleted: bool,
    pub is_posted: bool,
    pub is_error: bool,
    // Денормализованные поля для быстрых запросов списка
    #[sea_orm(nullable)]
    pub status_changed_at: Option<String>,
    #[sea_orm(nullable)]
    pub creation_date: Option<String>,
    #[sea_orm(nullable)]
    pub delivery_date: Option<String>,
    #[sea_orm(nullable)]
    pub campaign_id: Option<String>,
    #[sea_orm(nullable)]
    pub status_norm: Option<String>,
    #[sea_orm(nullable)]
    pub total_qty: Option<f64>,
    #[sea_orm(nullable)]
    pub total_amount: Option<f64>,
    #[sea_orm(nullable)]
    pub total_amount_api: Option<f64>,
    #[sea_orm(nullable)]
    pub lines_count: Option<i32>,
    #[sea_orm(nullable)]
    pub delivery_total: Option<f64>,
    #[sea_orm(nullable)]
    pub subsidies_total: Option<f64>,
    #[sea_orm(nullable)]
    pub organization_id: Option<String>,
    #[sea_orm(nullable)]
    pub connection_id: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// ============================================================================
// Items table entity (табличная часть)
// ============================================================================

pub mod items {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "a013_ym_order_items")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub order_id: String,
        pub line_id: String,
        pub shop_sku: String,
        pub offer_id: String,
        pub name: String,
        pub qty: f64,
        pub price_list: Option<f64>,
        pub discount_total: Option<f64>,
        pub price_effective: Option<f64>,
        pub amount_line: Option<f64>,
        pub price_plan: Option<f64>,
        pub marketplace_product_ref: Option<String>,
        pub nomenclature_ref: Option<String>,
        pub currency_code: Option<String>,
        pub buyer_price: Option<f64>,
        pub subsidies_json: Option<String>,
        pub status: Option<String>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}

    impl From<Model> for YmOrderLine {
        fn from(m: Model) -> Self {
            YmOrderLine {
                line_id: m.line_id,
                shop_sku: m.shop_sku,
                offer_id: m.offer_id,
                name: m.name,
                qty: m.qty,
                price_list: m.price_list,
                discount_total: m.discount_total,
                price_effective: m.price_effective,
                amount_line: m.amount_line,
                currency_code: m.currency_code,
                buyer_price: m.buyer_price,
                subsidies_json: m.subsidies_json,
                status: m.status,
                price_plan: m.price_plan,
                marketplace_product_ref: m.marketplace_product_ref,
                nomenclature_ref: m.nomenclature_ref,
            }
        }
    }
}

impl From<Model> for YmOrder {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());

        let header: YmOrderHeader = serde_json::from_str(&m.header_json).unwrap_or_else(|_| {
            panic!("Failed to deserialize header_json for document_no: {}", m.document_no)
        });
        let lines: Vec<YmOrderLine> = serde_json::from_str(&m.lines_json).unwrap_or_else(|_| {
            panic!("Failed to deserialize lines_json for document_no: {}", m.document_no)
        });
        let state: YmOrderState = serde_json::from_str(&m.state_json).unwrap_or_else(|_| {
            panic!("Failed to deserialize state_json for document_no: {}", m.document_no)
        });
        let source_meta: YmOrderSourceMeta =
            serde_json::from_str(&m.source_meta_json).unwrap_or_else(|_| {
                panic!("Failed to deserialize source_meta_json for document_no: {}", m.document_no)
            });

        YmOrder {
            base: BaseAggregate::with_metadata(
                YmOrderId(uuid),
                m.code,
                m.description,
                m.comment.clone(),
                metadata,
            ),
            header,
            lines,
            state,
            source_meta,
            is_posted: m.is_posted,
            is_error: m.is_error,
        }
    }
}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

pub async fn list_all() -> Result<Vec<YmOrder>> {
    let items: Vec<YmOrder> = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .order_by_desc(Column::UpdatedAt)
        .limit(1000)
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(items)
}

pub async fn get_by_id(id: Uuid) -> Result<Option<YmOrder>> {
    let result = Entity::find_by_id(id.to_string()).one(conn()).await?;
    Ok(result.map(Into::into))
}

pub async fn get_by_document_no(document_no: &str) -> Result<Option<YmOrder>> {
    let result = Entity::find()
        .filter(Column::DocumentNo.eq(document_no))
        .one(conn())
        .await?;
    Ok(result.map(Into::into))
}

/// Вычисление денормализованных полей из агрегата
fn calculate_denormalized_fields(aggregate: &YmOrder) -> DenormalizedFields {
    // Рассчитываем итоги по строкам
    let mut total_qty = 0.0;
    let mut total_amount = 0.0;
    let mut subsidies_total = 0.0;
    
    for line in &aggregate.lines {
        total_qty += line.qty;
        if let Some(amount) = line.amount_line {
            total_amount += amount;
        }
        // Parse line subsidies
        if let Some(ref subsidies_json) = line.subsidies_json {
            if let Ok(subsidies) = serde_json::from_str::<Vec<serde_json::Value>>(subsidies_json) {
                for sub in subsidies {
                    if let Some(amount) = sub.get("amount").and_then(|a| a.as_f64()) {
                        subsidies_total += amount;
                    }
                }
            }
        }
    }

    // Parse header subsidies
    if let Some(ref header_subsidies_json) = aggregate.header.subsidies_json {
        if let Ok(subsidies) = serde_json::from_str::<Vec<serde_json::Value>>(header_subsidies_json) {
            for sub in subsidies {
                if let Some(amount) = sub.get("amount").and_then(|a| a.as_f64()) {
                    subsidies_total += amount;
                }
            }
        }
    }

    DenormalizedFields {
        status_changed_at: aggregate.state.status_changed_at.map(|dt| dt.to_rfc3339()),
        creation_date: aggregate.state.creation_date.map(|dt| dt.to_rfc3339()),
        delivery_date: aggregate.state.delivery_date.map(|dt| dt.to_rfc3339()),
        campaign_id: Some(aggregate.header.campaign_id.clone()),
        status_norm: Some(aggregate.state.status_norm.clone()),
        total_qty: Some(total_qty),
        total_amount: Some(total_amount),
        total_amount_api: aggregate.header.total_amount,
        lines_count: Some(aggregate.lines.len() as i32),
        delivery_total: aggregate.header.delivery_total,
        subsidies_total: Some(subsidies_total),
        organization_id: Some(aggregate.header.organization_id.clone()),
        connection_id: Some(aggregate.header.connection_id.clone()),
    }
}

struct DenormalizedFields {
    status_changed_at: Option<String>,
    creation_date: Option<String>,
    delivery_date: Option<String>,
    campaign_id: Option<String>,
    status_norm: Option<String>,
    total_qty: Option<f64>,
    total_amount: Option<f64>,
    total_amount_api: Option<f64>,
    lines_count: Option<i32>,
    delivery_total: Option<f64>,
    subsidies_total: Option<f64>,
    organization_id: Option<String>,
    connection_id: Option<String>,
}

pub async fn upsert_document(aggregate: &YmOrder) -> Result<Uuid> {
    let uuid = aggregate.base.id.value();
    let existing = get_by_document_no(&aggregate.header.document_no).await?;
    
    let header_json = serde_json::to_string(&aggregate.header)?;
    let lines_json = serde_json::to_string(&aggregate.lines)?;
    let state_json = serde_json::to_string(&aggregate.state)?;
    let source_meta_json = serde_json::to_string(&aggregate.source_meta)?;

    // Вычисляем денормализованные поля
    let denorm = calculate_denormalized_fields(aggregate);

    let result_uuid = if let Some(existing_doc) = existing {
        let existing_uuid = existing_doc.base.id.value();
        let active = ActiveModel {
            id: Set(existing_uuid.to_string()),
            code: Set(aggregate.base.code.clone()),
            description: Set(aggregate.base.description.clone()),
            comment: Set(aggregate.base.comment.clone()),
            document_no: Set(aggregate.header.document_no.clone()),
            header_json: Set(header_json),
            lines_json: Set(lines_json),
            state_json: Set(state_json),
            source_meta_json: Set(source_meta_json),
            is_deleted: Set(aggregate.base.metadata.is_deleted),
            is_posted: Set(aggregate.is_posted),
            is_error: Set(aggregate.is_error),
            // Денормализованные поля
            status_changed_at: Set(denorm.status_changed_at),
            creation_date: Set(denorm.creation_date),
            delivery_date: Set(denorm.delivery_date),
            campaign_id: Set(denorm.campaign_id),
            status_norm: Set(denorm.status_norm),
            total_qty: Set(denorm.total_qty),
            total_amount: Set(denorm.total_amount),
            total_amount_api: Set(denorm.total_amount_api),
            lines_count: Set(denorm.lines_count),
            delivery_total: Set(denorm.delivery_total),
            subsidies_total: Set(denorm.subsidies_total),
            organization_id: Set(denorm.organization_id),
            connection_id: Set(denorm.connection_id),
            updated_at: Set(Some(aggregate.base.metadata.updated_at)),
            version: Set(aggregate.base.metadata.version + 1),
            created_at: sea_orm::ActiveValue::NotSet,
        };
        active.update(conn()).await?;
        existing_uuid
    } else {
        let active = ActiveModel {
            id: Set(uuid.to_string()),
            code: Set(aggregate.base.code.clone()),
            description: Set(aggregate.base.description.clone()),
            comment: Set(aggregate.base.comment.clone()),
            document_no: Set(aggregate.header.document_no.clone()),
            header_json: Set(header_json),
            lines_json: Set(lines_json),
            state_json: Set(state_json),
            source_meta_json: Set(source_meta_json),
            is_deleted: Set(aggregate.base.metadata.is_deleted),
            is_posted: Set(aggregate.is_posted),
            is_error: Set(aggregate.is_error),
            // Денормализованные поля
            status_changed_at: Set(denorm.status_changed_at),
            creation_date: Set(denorm.creation_date),
            delivery_date: Set(denorm.delivery_date),
            campaign_id: Set(denorm.campaign_id),
            status_norm: Set(denorm.status_norm),
            total_qty: Set(denorm.total_qty),
            total_amount: Set(denorm.total_amount),
            total_amount_api: Set(denorm.total_amount_api),
            lines_count: Set(denorm.lines_count),
            delivery_total: Set(denorm.delivery_total),
            subsidies_total: Set(denorm.subsidies_total),
            organization_id: Set(denorm.organization_id),
            connection_id: Set(denorm.connection_id),
            created_at: Set(Some(aggregate.base.metadata.created_at)),
            updated_at: Set(Some(aggregate.base.metadata.updated_at)),
            version: Set(aggregate.base.metadata.version),
        };
        active.insert(conn()).await?;
        uuid
    };

    // Сохраняем табличную часть (items)
    save_items(&result_uuid.to_string(), &aggregate.lines).await?;

    Ok(result_uuid)
}

pub async fn soft_delete(id: Uuid) -> Result<bool> {
    use sea_orm::sea_query::Expr;
    let result = Entity::update_many()
        .col_expr(Column::IsDeleted, Expr::value(true))
        .col_expr(Column::UpdatedAt, Expr::value(Utc::now()))
        .filter(Column::Id.eq(id.to_string()))
        .exec(conn())
        .await?;
    Ok(result.rows_affected > 0)
}

// ============================================================================
// Items table operations
// ============================================================================

/// Сохранение строк документа в табличную часть
/// Удаляет существующие строки и вставляет новые
pub async fn save_items(order_id: &str, lines: &[YmOrderLine]) -> Result<()> {
    // Удаляем существующие строки
    delete_items(order_id).await?;

    // Вставляем новые строки
    for line in lines {
        let item_id = Uuid::new_v4().to_string();
        let active = items::ActiveModel {
            id: Set(item_id),
            order_id: Set(order_id.to_string()),
            line_id: Set(line.line_id.clone()),
            shop_sku: Set(line.shop_sku.clone()),
            offer_id: Set(line.offer_id.clone()),
            name: Set(line.name.clone()),
            qty: Set(line.qty),
            price_list: Set(line.price_list),
            discount_total: Set(line.discount_total),
            price_effective: Set(line.price_effective),
            amount_line: Set(line.amount_line),
            price_plan: Set(line.price_plan),
            marketplace_product_ref: Set(line.marketplace_product_ref.clone()),
            nomenclature_ref: Set(line.nomenclature_ref.clone()),
            currency_code: Set(line.currency_code.clone()),
            buyer_price: Set(line.buyer_price),
            subsidies_json: Set(line.subsidies_json.clone()),
            status: Set(line.status.clone()),
        };
        items::Entity::insert(active).exec(conn()).await?;
    }

    Ok(())
}

/// Получение строк документа из табличной части
pub async fn get_items(order_id: &str) -> Result<Vec<YmOrderLine>> {
    let items_models = items::Entity::find()
        .filter(items::Column::OrderId.eq(order_id))
        .all(conn())
        .await?;

    Ok(items_models.into_iter().map(Into::into).collect())
}

/// Удаление строк документа из табличной части
pub async fn delete_items(order_id: &str) -> Result<()> {
    items::Entity::delete_many()
        .filter(items::Column::OrderId.eq(order_id))
        .exec(conn())
        .await?;
    Ok(())
}

/// Получение документа с полными строками из табличной части
/// (вместо строк из lines_json загружает из a013_ym_order_items)
pub async fn get_by_id_with_items(id: Uuid) -> Result<Option<YmOrder>> {
    let result = Entity::find_by_id(id.to_string()).one(conn()).await?;
    
    if let Some(model) = result {
        let mut order: YmOrder = model.into();
        // Заменяем строки из JSON на строки из табличной части
        let items = get_items(&id.to_string()).await?;
        if !items.is_empty() {
            order.lines = items;
        }
        Ok(Some(order))
    } else {
        Ok(None)
    }
}

/// Обновление строк документа (без пересохранения всего документа)
pub async fn update_items(order_id: &str, lines: &[YmOrderLine]) -> Result<()> {
    save_items(order_id, lines).await
}

// ============================================================================
// List operations (денормализованные запросы)
// ============================================================================

/// DTO для списка - использует денормализованные колонки, без парсинга JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmOrderListRow {
    pub id: String,
    pub document_no: String,
    pub status_changed_at: Option<String>,
    pub creation_date: Option<String>,
    pub delivery_date: Option<String>,
    pub campaign_id: Option<String>,
    pub status_norm: Option<String>,
    pub total_qty: Option<f64>,
    pub total_amount: Option<f64>,
    pub total_amount_api: Option<f64>,
    pub lines_count: Option<i32>,
    pub delivery_total: Option<f64>,
    pub subsidies_total: Option<f64>,
    pub is_posted: bool,
    pub is_error: bool,
}

/// Параметры запроса для списка
#[derive(Debug, Clone)]
pub struct YmOrderListQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub organization_id: Option<String>,
    pub search_document_no: Option<String>,
    pub status_norm: Option<String>,
    pub sort_by: String,
    pub sort_desc: bool,
    pub limit: usize,
    pub offset: usize,
}

/// Результат запроса списка с пагинацией
#[derive(Debug, Clone)]
pub struct YmOrderListResult {
    pub items: Vec<YmOrderListRow>,
    pub total: usize,
}

/// Получение списка заказов через SQL (без парсинга JSON)
pub async fn list_sql(query: YmOrderListQuery) -> Result<YmOrderListResult> {
    use sea_orm::{ConnectionTrait, Statement};
    
    let db = conn();

    // Build WHERE clause
    let mut conditions = vec!["is_deleted = 0".to_string()];

    if let Some(ref date_from) = query.date_from {
        if !date_from.is_empty() {
            conditions.push(format!("delivery_date >= '{}'", date_from));
        }
    }
    if let Some(ref date_to) = query.date_to {
        if !date_to.is_empty() {
            // Add time part to include the whole day
            conditions.push(format!("delivery_date <= '{}T23:59:59'", date_to));
        }
    }
    if let Some(ref org_id) = query.organization_id {
        if !org_id.is_empty() {
            conditions.push(format!("organization_id = '{}'", org_id));
        }
    }
    if let Some(ref search_doc_no) = query.search_document_no {
        if !search_doc_no.is_empty() {
            conditions.push(format!(
                "document_no LIKE '%{}%'",
                search_doc_no.replace('\'', "''")
            ));
        }
    }
    if let Some(ref status) = query.status_norm {
        if !status.is_empty() {
            conditions.push(format!("status_norm = '{}'", status));
        }
    }

    let where_clause = conditions.join(" AND ");

    // Count total
    let count_sql = format!(
        "SELECT COUNT(*) as cnt FROM a013_ym_order WHERE {}",
        where_clause
    );
    let count_result = db
        .query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            count_sql,
        ))
        .await?;
    let total = count_result
        .map(|r| r.try_get::<i32>("", "cnt").unwrap_or(0) as usize)
        .unwrap_or(0);

    // Build ORDER BY
    let order_column = match query.sort_by.as_str() {
        "document_no" => "document_no",
        "status_changed_at" => "status_changed_at",
        "creation_date" => "creation_date",
        "delivery_date" => "delivery_date",
        "campaign_id" => "campaign_id",
        "status_norm" => "status_norm",
        "total_qty" => "total_qty",
        "total_amount" => "total_amount",
        "lines_count" => "lines_count",
        "delivery_total" => "delivery_total",
        "subsidies_total" => "subsidies_total",
        _ => "delivery_date",
    };
    let order_dir = if query.sort_desc { "DESC" } else { "ASC" };

    // Query data
    let data_sql = format!(
        r#"SELECT 
            id, document_no, status_changed_at, creation_date, delivery_date,
            campaign_id, status_norm, total_qty, total_amount, total_amount_api,
            lines_count, delivery_total, subsidies_total, is_posted, is_error
        FROM a013_ym_order 
        WHERE {}
        ORDER BY {} {} NULLS LAST
        LIMIT {} OFFSET {}"#,
        where_clause, order_column, order_dir, query.limit, query.offset
    );

    let rows = db
        .query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            data_sql,
        ))
        .await?;

    let items: Vec<YmOrderListRow> = rows
        .into_iter()
        .filter_map(|row| {
            Some(YmOrderListRow {
                id: row.try_get("", "id").ok()?,
                document_no: row.try_get("", "document_no").ok()?,
                status_changed_at: row.try_get("", "status_changed_at").ok(),
                creation_date: row.try_get("", "creation_date").ok(),
                delivery_date: row.try_get("", "delivery_date").ok(),
                campaign_id: row.try_get("", "campaign_id").ok(),
                status_norm: row.try_get("", "status_norm").ok(),
                total_qty: row.try_get("", "total_qty").ok(),
                total_amount: row.try_get("", "total_amount").ok(),
                total_amount_api: row.try_get("", "total_amount_api").ok(),
                lines_count: row.try_get("", "lines_count").ok(),
                delivery_total: row.try_get("", "delivery_total").ok(),
                subsidies_total: row.try_get("", "subsidies_total").ok(),
                is_posted: row
                    .try_get::<i32>("", "is_posted")
                    .map(|v| v != 0)
                    .unwrap_or(false),
                is_error: row
                    .try_get::<i32>("", "is_error")
                    .map(|v| v != 0)
                    .unwrap_or(false),
            })
        })
        .collect();

    Ok(YmOrderListResult { items, total })
}

