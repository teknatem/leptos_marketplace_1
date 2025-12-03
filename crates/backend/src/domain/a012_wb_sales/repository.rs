use anyhow::Result;
use chrono::Utc;
use contracts::domain::a012_wb_sales::aggregate::{
    WbSales, WbSalesHeader, WbSalesId, WbSalesLine, WbSalesSourceMeta, WbSalesState,
    WbSalesWarehouse,
};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a012_wb_sales")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub document_no: String,
    #[sea_orm(nullable)]
    pub sale_id: Option<String>,
    // Denormalized fields for fast queries
    #[sea_orm(nullable)]
    pub sale_date: Option<String>,
    #[sea_orm(nullable)]
    pub organization_id: Option<String>,
    #[sea_orm(nullable)]
    pub connection_id: Option<String>,
    #[sea_orm(nullable)]
    pub supplier_article: Option<String>,
    #[sea_orm(nullable)]
    pub nm_id: Option<i64>,
    #[sea_orm(nullable)]
    pub barcode: Option<String>,
    #[sea_orm(nullable)]
    pub product_name: Option<String>,
    #[sea_orm(nullable)]
    pub qty: Option<f64>,
    #[sea_orm(nullable)]
    pub amount_line: Option<f64>,
    #[sea_orm(nullable)]
    pub total_price: Option<f64>,
    #[sea_orm(nullable)]
    pub finished_price: Option<f64>,
    #[sea_orm(nullable)]
    pub event_type: Option<String>,
    // JSON storage
    pub header_json: String,
    pub line_json: String,
    pub state_json: String,
    #[sea_orm(nullable)]
    pub warehouse_json: Option<String>,
    pub source_meta_json: String,
    pub marketplace_product_ref: Option<String>,
    pub nomenclature_ref: Option<String>,
    pub is_deleted: bool,
    pub is_posted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for WbSales {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());

        let header: WbSalesHeader = serde_json::from_str(&m.header_json).unwrap_or_else(|_| {
            panic!(
                "Failed to deserialize header_json for document_no: {}",
                m.document_no
            )
        });
        let line: WbSalesLine = serde_json::from_str(&m.line_json).unwrap_or_else(|_| {
            panic!(
                "Failed to deserialize line_json for document_no: {}",
                m.document_no
            )
        });
        let state: WbSalesState = serde_json::from_str(&m.state_json).unwrap_or_else(|_| {
            panic!(
                "Failed to deserialize state_json for document_no: {}",
                m.document_no
            )
        });
        let warehouse: WbSalesWarehouse = m
            .warehouse_json
            .as_ref()
            .and_then(|json| serde_json::from_str(json).ok())
            .unwrap_or(WbSalesWarehouse {
                warehouse_name: None,
                warehouse_type: None,
            });
        let source_meta: WbSalesSourceMeta = serde_json::from_str(&m.source_meta_json)
            .unwrap_or_else(|_| {
                panic!(
                    "Failed to deserialize source_meta_json for document_no: {}",
                    m.document_no
                )
            });

        WbSales {
            base: BaseAggregate::with_metadata(
                WbSalesId(uuid),
                m.code,
                m.description,
                m.comment.clone(),
                metadata,
            ),
            header,
            line,
            state,
            warehouse,
            source_meta,
            is_posted: m.is_posted,
            marketplace_product_ref: m.marketplace_product_ref,
            nomenclature_ref: m.nomenclature_ref,
        }
    }
}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

pub async fn list_all() -> Result<Vec<WbSales>> {
    let items: Vec<WbSales> = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(items)
}

pub async fn get_by_id(id: Uuid) -> Result<Option<WbSales>> {
    let result = Entity::find_by_id(id.to_string()).one(conn()).await?;
    Ok(result.map(Into::into))
}

pub async fn get_by_document_no(document_no: &str) -> Result<Option<WbSales>> {
    let result = Entity::find()
        .filter(Column::DocumentNo.eq(document_no))
        .one(conn())
        .await?;
    Ok(result.map(Into::into))
}

/// Get by sale_id (saleID from WB API) - used for deduplication
pub async fn get_by_sale_id(sale_id: &str) -> Result<Option<WbSales>> {
    let result = Entity::find()
        .filter(Column::SaleId.eq(sale_id))
        .one(conn())
        .await?;
    Ok(result.map(Into::into))
}

pub async fn upsert_document(aggregate: &WbSales) -> Result<Uuid> {
    let uuid = aggregate.base.id.value();

    // Try to find existing document by sale_id first (if available), then by document_no (srid)
    let existing = if let Some(ref sale_id) = aggregate.header.sale_id {
        get_by_sale_id(sale_id).await?
    } else {
        get_by_document_no(&aggregate.header.document_no).await?
    };

    let header_json = serde_json::to_string(&aggregate.header)?;
    let line_json = serde_json::to_string(&aggregate.line)?;
    let state_json = serde_json::to_string(&aggregate.state)?;
    let warehouse_json = serde_json::to_string(&aggregate.warehouse)?;
    let source_meta_json = serde_json::to_string(&aggregate.source_meta)?;

    // Denormalized fields for fast queries
    let sale_date = Some(aggregate.state.sale_dt.to_rfc3339());
    let organization_id = Some(aggregate.header.organization_id.clone());
    let connection_id = Some(aggregate.header.connection_id.clone());
    let supplier_article = Some(aggregate.line.supplier_article.clone());
    let nm_id = Some(aggregate.line.nm_id);
    let barcode = Some(aggregate.line.barcode.clone());
    let product_name = Some(aggregate.line.name.clone());
    let qty = Some(aggregate.line.qty);
    let amount_line = aggregate.line.amount_line;
    let total_price = aggregate.line.total_price;
    let finished_price = aggregate.line.finished_price;
    let event_type = Some(aggregate.state.event_type.clone());

    if let Some(existing_doc) = existing {
        let existing_uuid = existing_doc.base.id.value();
        let active = ActiveModel {
            id: Set(existing_uuid.to_string()),
            code: Set(aggregate.base.code.clone()),
            description: Set(aggregate.base.description.clone()),
            comment: Set(aggregate.base.comment.clone()),
            document_no: Set(aggregate.header.document_no.clone()),
            sale_id: Set(aggregate.header.sale_id.clone()),
            // Denormalized fields
            sale_date: Set(sale_date),
            organization_id: Set(organization_id),
            connection_id: Set(connection_id),
            supplier_article: Set(supplier_article),
            nm_id: Set(nm_id),
            barcode: Set(barcode),
            product_name: Set(product_name),
            qty: Set(qty),
            amount_line: Set(amount_line),
            total_price: Set(total_price),
            finished_price: Set(finished_price),
            event_type: Set(event_type),
            // JSON fields
            header_json: Set(header_json),
            line_json: Set(line_json),
            state_json: Set(state_json),
            warehouse_json: Set(Some(warehouse_json.clone())),
            source_meta_json: Set(source_meta_json),
            marketplace_product_ref: Set(aggregate.marketplace_product_ref.clone()),
            nomenclature_ref: Set(aggregate.nomenclature_ref.clone()),
            is_deleted: Set(aggregate.base.metadata.is_deleted),
            is_posted: Set(aggregate.is_posted),
            updated_at: Set(Some(aggregate.base.metadata.updated_at)),
            version: Set(aggregate.base.metadata.version + 1),
            created_at: sea_orm::ActiveValue::NotSet,
        };
        active.update(conn()).await?;
        Ok(existing_uuid)
    } else {
        let active = ActiveModel {
            id: Set(uuid.to_string()),
            code: Set(aggregate.base.code.clone()),
            description: Set(aggregate.base.description.clone()),
            comment: Set(aggregate.base.comment.clone()),
            document_no: Set(aggregate.header.document_no.clone()),
            sale_id: Set(aggregate.header.sale_id.clone()),
            // Denormalized fields
            sale_date: Set(sale_date),
            organization_id: Set(organization_id),
            connection_id: Set(connection_id),
            supplier_article: Set(supplier_article),
            nm_id: Set(nm_id),
            barcode: Set(barcode),
            product_name: Set(product_name),
            qty: Set(qty),
            amount_line: Set(amount_line),
            total_price: Set(total_price),
            finished_price: Set(finished_price),
            event_type: Set(event_type),
            // JSON fields
            header_json: Set(header_json),
            line_json: Set(line_json),
            state_json: Set(state_json),
            warehouse_json: Set(Some(warehouse_json)),
            source_meta_json: Set(source_meta_json),
            marketplace_product_ref: Set(aggregate.marketplace_product_ref.clone()),
            nomenclature_ref: Set(aggregate.nomenclature_ref.clone()),
            is_deleted: Set(aggregate.base.metadata.is_deleted),
            is_posted: Set(aggregate.is_posted),
            created_at: Set(Some(aggregate.base.metadata.created_at)),
            updated_at: Set(Some(aggregate.base.metadata.updated_at)),
            version: Set(aggregate.base.metadata.version),
        };
        active.insert(conn()).await?;
        Ok(uuid)
    }
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

pub async fn search_by_document_no(document_no: &str) -> Result<Vec<WbSales>> {
    let items: Vec<WbSales> = Entity::find()
        .filter(Column::DocumentNo.eq(document_no))
        .filter(Column::IsDeleted.eq(false))
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(items)
}

/// DTO for list view - uses denormalized columns, no JSON parsing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSalesListRow {
    pub id: String,
    pub document_no: String,
    pub sale_id: Option<String>,
    pub sale_date: Option<String>,
    pub organization_id: Option<String>,
    pub supplier_article: Option<String>,
    pub product_name: Option<String>,
    pub qty: Option<f64>,
    pub amount_line: Option<f64>,
    pub total_price: Option<f64>,
    pub finished_price: Option<f64>,
    pub event_type: Option<String>,
    pub marketplace_product_ref: Option<String>,
    pub nomenclature_ref: Option<String>,
    pub is_posted: bool,
}

/// Query parameters for list
#[derive(Debug, Clone)]
pub struct WbSalesListQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub organization_id: Option<String>,
    pub search_sale_id: Option<String>,
    pub search_srid: Option<String>,
    pub sort_by: String,
    pub sort_desc: bool,
    pub limit: usize,
    pub offset: usize,
}

/// Result from list query with pagination
#[derive(Debug, Clone)]
pub struct WbSalesListResult {
    pub items: Vec<WbSalesListRow>,
    pub total: usize,
}

/// List WB Sales using direct SQL query (no caching, no JSON parsing)
pub async fn list_sql(query: WbSalesListQuery) -> Result<WbSalesListResult> {
    use sea_orm::{ConnectionTrait, Statement};

    let db = conn();

    // Build WHERE clause
    let mut conditions = vec!["is_deleted = 0".to_string()];

    if let Some(ref date_from) = query.date_from {
        conditions.push(format!("sale_date >= '{}'", date_from));
    }
    if let Some(ref date_to) = query.date_to {
        // Add time part to include the whole day
        conditions.push(format!("sale_date <= '{}T23:59:59'", date_to));
    }
    if let Some(ref org_id) = query.organization_id {
        if !org_id.is_empty() {
            conditions.push(format!("organization_id = '{}'", org_id));
        }
    }
    if let Some(ref search_sale_id) = query.search_sale_id {
        if !search_sale_id.is_empty() {
            conditions.push(format!(
                "sale_id LIKE '%{}%'",
                search_sale_id.replace("'", "''")
            ));
        }
    }
    if let Some(ref search_srid) = query.search_srid {
        if !search_srid.is_empty() {
            conditions.push(format!(
                "document_no LIKE '%{}%'",
                search_srid.replace("'", "''")
            ));
        }
    }

    let where_clause = conditions.join(" AND ");

    // Count total
    let count_sql = format!(
        "SELECT COUNT(*) as cnt FROM a012_wb_sales WHERE {}",
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
        "sale_id" => "sale_id",
        "sale_date" => "sale_date",
        "supplier_article" => "supplier_article",
        "product_name" => "product_name",
        "qty" => "qty",
        "amount_line" => "amount_line",
        "total_price" => "total_price",
        "finished_price" => "finished_price",
        "event_type" => "event_type",
        _ => "sale_date",
    };
    let order_dir = if query.sort_desc { "DESC" } else { "ASC" };

    // Query data
    let data_sql = format!(
        r#"SELECT 
            id, document_no, sale_id, sale_date, organization_id,
            supplier_article, product_name, qty, amount_line, total_price,
            finished_price, event_type, marketplace_product_ref, nomenclature_ref, is_posted
        FROM a012_wb_sales 
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

    let items: Vec<WbSalesListRow> = rows
        .into_iter()
        .filter_map(|row| {
            Some(WbSalesListRow {
                id: row.try_get("", "id").ok()?,
                document_no: row.try_get("", "document_no").ok()?,
                sale_id: row.try_get("", "sale_id").ok(),
                sale_date: row.try_get("", "sale_date").ok(),
                organization_id: row.try_get("", "organization_id").ok(),
                supplier_article: row.try_get("", "supplier_article").ok(),
                product_name: row.try_get("", "product_name").ok(),
                qty: row.try_get("", "qty").ok(),
                amount_line: row.try_get("", "amount_line").ok(),
                total_price: row.try_get("", "total_price").ok(),
                finished_price: row.try_get("", "finished_price").ok(),
                event_type: row.try_get("", "event_type").ok(),
                marketplace_product_ref: row.try_get("", "marketplace_product_ref").ok(),
                nomenclature_ref: row.try_get("", "nomenclature_ref").ok(),
                is_posted: row
                    .try_get::<i32>("", "is_posted")
                    .map(|v| v != 0)
                    .unwrap_or(false),
            })
        })
        .collect();

    Ok(WbSalesListResult { items, total })
}
