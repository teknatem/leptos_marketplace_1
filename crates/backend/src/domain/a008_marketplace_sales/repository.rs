use chrono::{NaiveDate, Utc};
use contracts::domain::a008_marketplace_sales::aggregate::{MarketplaceSales, MarketplaceSalesId};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a008_marketplace_sales")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub connection_id: String,
    pub organization_id: String,
    pub marketplace_id: String,
    pub accrual_date: String, // stored as YYYY-MM-DD
    pub product_id: String,
    pub quantity: i32,
    pub revenue: f64,
    pub operation_type: String,
    pub is_deleted: bool,
    pub is_posted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for MarketplaceSales {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());
        let accrual_date = NaiveDate::parse_from_str(&m.accrual_date, "%Y-%m-%d").unwrap_or_else(|_| Utc::now().date_naive());

        MarketplaceSales {
            base: BaseAggregate::with_metadata(
                MarketplaceSalesId(uuid),
                m.code,
                m.description,
                m.comment.clone(),
                metadata,
            ),
            connection_id: m.connection_id,
            organization_id: m.organization_id,
            marketplace_id: m.marketplace_id,
            accrual_date,
            product_id: m.product_id,
            quantity: m.quantity,
            revenue: m.revenue,
            operation_type: m.operation_type,
        }
    }
}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

pub async fn list_all() -> anyhow::Result<Vec<MarketplaceSales>> {
    let mut items: Vec<MarketplaceSales> = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    items.sort_by(|a, b| a.accrual_date.cmp(&b.accrual_date));
    Ok(items)
}

pub async fn get_by_id(id: Uuid) -> anyhow::Result<Option<MarketplaceSales>> {
    let result = Entity::find_by_id(id.to_string()).one(conn()).await?;
    Ok(result.map(Into::into))
}

pub async fn insert(aggregate: &MarketplaceSales) -> anyhow::Result<Uuid> {
    let uuid = aggregate.base.id.value();
    let active = ActiveModel {
        id: Set(uuid.to_string()),
        code: Set(aggregate.base.code.clone()),
        description: Set(aggregate.base.description.clone()),
        comment: Set(aggregate.base.comment.clone()),
        connection_id: Set(aggregate.connection_id.clone()),
        organization_id: Set(aggregate.organization_id.clone()),
        marketplace_id: Set(aggregate.marketplace_id.clone()),
        accrual_date: Set(aggregate.accrual_date.format("%Y-%m-%d").to_string()),
        product_id: Set(aggregate.product_id.clone()),
        quantity: Set(aggregate.quantity),
        revenue: Set(aggregate.revenue),
        operation_type: Set(aggregate.operation_type.clone()),
        is_deleted: Set(aggregate.base.metadata.is_deleted),
        is_posted: Set(aggregate.base.metadata.is_posted),
        created_at: Set(Some(aggregate.base.metadata.created_at)),
        updated_at: Set(Some(aggregate.base.metadata.updated_at)),
        version: Set(aggregate.base.metadata.version),
    };
    active.insert(conn()).await?;
    Ok(uuid)
}

pub async fn update(aggregate: &MarketplaceSales) -> anyhow::Result<()> {
    let id = aggregate.base.id.value().to_string();
    let active = ActiveModel {
        id: Set(id),
        code: Set(aggregate.base.code.clone()),
        description: Set(aggregate.base.description.clone()),
        comment: Set(aggregate.base.comment.clone()),
        connection_id: Set(aggregate.connection_id.clone()),
        organization_id: Set(aggregate.organization_id.clone()),
        marketplace_id: Set(aggregate.marketplace_id.clone()),
        accrual_date: Set(aggregate.accrual_date.format("%Y-%m-%d").to_string()),
        product_id: Set(aggregate.product_id.clone()),
        quantity: Set(aggregate.quantity),
        revenue: Set(aggregate.revenue),
        operation_type: Set(aggregate.operation_type.clone()),
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

pub async fn get_by_key(
    connection_id: &str,
    product_id: &str,
    accrual_date: NaiveDate,
    operation_type: &str,
) -> anyhow::Result<Option<MarketplaceSales>> {
    let result = Entity::find()
        .filter(Column::ConnectionId.eq(connection_id))
        .filter(Column::ProductId.eq(product_id))
        .filter(Column::AccrualDate.eq(accrual_date.format("%Y-%m-%d").to_string()))
        .filter(Column::OperationType.eq(operation_type))
        .filter(Column::IsDeleted.eq(false))
        .one(conn())
        .await?;
    Ok(result.map(Into::into))
}


