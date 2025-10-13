use chrono::{NaiveDate, Utc};
use contracts::domain::a009_ozon_returns::aggregate::{OzonReturns, OzonReturnsId};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a009_ozon_returns")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub connection_id: String,
    pub organization_id: String,
    pub marketplace_id: String,
    pub return_id: String,
    pub return_date: String, // stored as YYYY-MM-DD
    pub return_reason_name: String,
    pub return_type: String,
    pub order_id: String,
    pub order_number: String,
    pub sku: String,
    pub product_name: String,
    pub price: f64,
    pub quantity: i32,
    pub posting_number: String,
    pub clearing_id: Option<String>,
    pub return_clearing_id: Option<String>,
    pub is_deleted: bool,
    pub is_posted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for OzonReturns {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());
        let return_date = NaiveDate::parse_from_str(&m.return_date, "%Y-%m-%d")
            .unwrap_or_else(|_| Utc::now().date_naive());

        OzonReturns {
            base: BaseAggregate::with_metadata(
                OzonReturnsId(uuid),
                m.code,
                m.description,
                m.comment.clone(),
                metadata,
            ),
            connection_id: m.connection_id,
            organization_id: m.organization_id,
            marketplace_id: m.marketplace_id,
            return_id: m.return_id,
            return_date,
            return_reason_name: m.return_reason_name,
            return_type: m.return_type,
            order_id: m.order_id,
            order_number: m.order_number,
            sku: m.sku,
            product_name: m.product_name,
            price: m.price,
            quantity: m.quantity,
            posting_number: m.posting_number,
            clearing_id: m.clearing_id,
            return_clearing_id: m.return_clearing_id,
        }
    }
}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

pub async fn list_all() -> anyhow::Result<Vec<OzonReturns>> {
    let mut items: Vec<OzonReturns> = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    items.sort_by(|a, b| b.return_date.cmp(&a.return_date));
    Ok(items)
}

pub async fn get_by_id(id: Uuid) -> anyhow::Result<Option<OzonReturns>> {
    let result = Entity::find_by_id(id.to_string()).one(conn()).await?;
    Ok(result.map(Into::into))
}

pub async fn insert(aggregate: &OzonReturns) -> anyhow::Result<Uuid> {
    let uuid = aggregate.base.id.value();
    let active = ActiveModel {
        id: Set(uuid.to_string()),
        code: Set(aggregate.base.code.clone()),
        description: Set(aggregate.base.description.clone()),
        comment: Set(aggregate.base.comment.clone()),
        connection_id: Set(aggregate.connection_id.clone()),
        organization_id: Set(aggregate.organization_id.clone()),
        marketplace_id: Set(aggregate.marketplace_id.clone()),
        return_id: Set(aggregate.return_id.clone()),
        return_date: Set(aggregate.return_date.format("%Y-%m-%d").to_string()),
        return_reason_name: Set(aggregate.return_reason_name.clone()),
        return_type: Set(aggregate.return_type.clone()),
        order_id: Set(aggregate.order_id.clone()),
        order_number: Set(aggregate.order_number.clone()),
        sku: Set(aggregate.sku.clone()),
        product_name: Set(aggregate.product_name.clone()),
        price: Set(aggregate.price),
        quantity: Set(aggregate.quantity),
        posting_number: Set(aggregate.posting_number.clone()),
        clearing_id: Set(aggregate.clearing_id.clone()),
        return_clearing_id: Set(aggregate.return_clearing_id.clone()),
        is_deleted: Set(aggregate.base.metadata.is_deleted),
        is_posted: Set(aggregate.base.metadata.is_posted),
        created_at: Set(Some(aggregate.base.metadata.created_at)),
        updated_at: Set(Some(aggregate.base.metadata.updated_at)),
        version: Set(aggregate.base.metadata.version),
    };
    active.insert(conn()).await?;
    Ok(uuid)
}

pub async fn update(aggregate: &OzonReturns) -> anyhow::Result<()> {
    let id = aggregate.base.id.value().to_string();
    let active = ActiveModel {
        id: Set(id),
        code: Set(aggregate.base.code.clone()),
        description: Set(aggregate.base.description.clone()),
        comment: Set(aggregate.base.comment.clone()),
        connection_id: Set(aggregate.connection_id.clone()),
        organization_id: Set(aggregate.organization_id.clone()),
        marketplace_id: Set(aggregate.marketplace_id.clone()),
        return_id: Set(aggregate.return_id.clone()),
        return_date: Set(aggregate.return_date.format("%Y-%m-%d").to_string()),
        return_reason_name: Set(aggregate.return_reason_name.clone()),
        return_type: Set(aggregate.return_type.clone()),
        order_id: Set(aggregate.order_id.clone()),
        order_number: Set(aggregate.order_number.clone()),
        sku: Set(aggregate.sku.clone()),
        product_name: Set(aggregate.product_name.clone()),
        price: Set(aggregate.price),
        quantity: Set(aggregate.quantity),
        posting_number: Set(aggregate.posting_number.clone()),
        clearing_id: Set(aggregate.clearing_id.clone()),
        return_clearing_id: Set(aggregate.return_clearing_id.clone()),
        is_deleted: Set(aggregate.base.metadata.is_deleted),
        is_posted: Set(aggregate.base.metadata.is_posted),
        updated_at: Set(Some(aggregate.base.metadata.updated_at)),
        version: Set(aggregate.base.metadata.version),
        created_at: sea_orm::ActiveValue::NotSet,
    };
    active.update(conn()).await?;
    Ok(())
}

pub async fn soft_delete(id: Uuid) -> anyhow::Result<bool> {
    use sea_orm::sea_query::Expr;
    let result = Entity::update_many()
        .col_expr(Column::IsDeleted, Expr::value(true))
        .col_expr(Column::UpdatedAt, Expr::value(Utc::now()))
        .filter(Column::Id.eq(id.to_string()))
        .exec(conn())
        .await?;
    Ok(result.rows_affected > 0)
}

/// Получить возврат по ключу (connection_id, return_id, sku) для upsert логики
pub async fn get_by_return_key(
    connection_id: &str,
    return_id: &str,
    sku: &str,
) -> anyhow::Result<Option<OzonReturns>> {
    let result = Entity::find()
        .filter(Column::ConnectionId.eq(connection_id))
        .filter(Column::ReturnId.eq(return_id))
        .filter(Column::Sku.eq(sku))
        .filter(Column::IsDeleted.eq(false))
        .one(conn())
        .await?;
    Ok(result.map(Into::into))
}
