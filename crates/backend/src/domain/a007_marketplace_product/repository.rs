use chrono::Utc;
use contracts::domain::a007_marketplace_product::aggregate::{
    MarketplaceProduct, MarketplaceProductId,
};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a007_marketplace_product")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub marketplace_ref: String,
    pub connection_mp_ref: String,
    pub marketplace_sku: String,
    pub barcode: Option<String>,
    pub article: String,
    pub brand: Option<String>,
    pub category_id: Option<String>,
    pub category_name: Option<String>,
    pub last_update: Option<chrono::DateTime<chrono::Utc>>,
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

impl From<Model> for MarketplaceProduct {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());

        MarketplaceProduct {
            base: BaseAggregate::with_metadata(
                MarketplaceProductId(uuid),
                m.code,
                m.description,
                m.comment.clone(),
                metadata,
            ),
            marketplace_ref: m.marketplace_ref,
            connection_mp_ref: m.connection_mp_ref,
            marketplace_sku: m.marketplace_sku,
            barcode: m.barcode,
            article: m.article,
            brand: m.brand,
            category_id: m.category_id,
            category_name: m.category_name,
            last_update: m.last_update,
            nomenclature_ref: m.nomenclature_ref,
        }
    }
}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

pub async fn list_all() -> anyhow::Result<Vec<MarketplaceProduct>> {
    let mut items: Vec<MarketplaceProduct> = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    items.sort_by(|a, b| {
        a.base
            .description
            .to_lowercase()
            .cmp(&b.base.description.to_lowercase())
    });
    Ok(items)
}

pub async fn get_by_id(id: Uuid) -> anyhow::Result<Option<MarketplaceProduct>> {
    let result = Entity::find_by_id(id.to_string()).one(conn()).await?;
    Ok(result.map(Into::into))
}

pub async fn insert(aggregate: &MarketplaceProduct) -> anyhow::Result<Uuid> {
    let uuid = aggregate.base.id.value();
    let active = ActiveModel {
        id: Set(uuid.to_string()),
        code: Set(aggregate.base.code.clone()),
        description: Set(aggregate.base.description.clone()),
        comment: Set(aggregate.base.comment.clone()),
        marketplace_ref: Set(aggregate.marketplace_ref.clone()),
        connection_mp_ref: Set(aggregate.connection_mp_ref.clone()),
        marketplace_sku: Set(aggregate.marketplace_sku.clone()),
        barcode: Set(aggregate.barcode.clone()),
        article: Set(aggregate.article.clone()),
        brand: Set(aggregate.brand.clone()),
        category_id: Set(aggregate.category_id.clone()),
        category_name: Set(aggregate.category_name.clone()),
        last_update: Set(aggregate.last_update),
        nomenclature_ref: Set(aggregate.nomenclature_ref.clone()),
        is_deleted: Set(aggregate.base.metadata.is_deleted),
        is_posted: Set(aggregate.base.metadata.is_posted),
        created_at: Set(Some(aggregate.base.metadata.created_at)),
        updated_at: Set(Some(aggregate.base.metadata.updated_at)),
        version: Set(aggregate.base.metadata.version),
    };
    active.insert(conn()).await?;
    Ok(uuid)
}

pub async fn update(aggregate: &MarketplaceProduct) -> anyhow::Result<()> {
    let id = aggregate.base.id.value().to_string();
    let active = ActiveModel {
        id: Set(id),
        code: Set(aggregate.base.code.clone()),
        description: Set(aggregate.base.description.clone()),
        comment: Set(aggregate.base.comment.clone()),
        marketplace_ref: Set(aggregate.marketplace_ref.clone()),
        connection_mp_ref: Set(aggregate.connection_mp_ref.clone()),
        marketplace_sku: Set(aggregate.marketplace_sku.clone()),
        barcode: Set(aggregate.barcode.clone()),
        article: Set(aggregate.article.clone()),
        brand: Set(aggregate.brand.clone()),
        category_id: Set(aggregate.category_id.clone()),
        category_name: Set(aggregate.category_name.clone()),
        last_update: Set(aggregate.last_update),
        nomenclature_ref: Set(aggregate.nomenclature_ref.clone()),
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

pub async fn get_by_connection_and_sku(
    connection_mp_ref: &str,
    sku: &str,
) -> anyhow::Result<Option<MarketplaceProduct>> {
    let result = Entity::find()
        .filter(Column::ConnectionMpRef.eq(connection_mp_ref))
        .filter(Column::MarketplaceSku.eq(sku))
        .filter(Column::IsDeleted.eq(false))
        .one(conn())
        .await?;
    Ok(result.map(Into::into))
}

pub async fn get_by_barcode(barcode: &str) -> anyhow::Result<Vec<MarketplaceProduct>> {
    let items: Vec<MarketplaceProduct> = Entity::find()
        .filter(Column::Barcode.eq(barcode))
        .filter(Column::IsDeleted.eq(false))
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(items)
}

pub async fn list_by_marketplace_ref(
    marketplace_ref: &str,
) -> anyhow::Result<Vec<MarketplaceProduct>> {
    let items: Vec<MarketplaceProduct> = Entity::find()
        .filter(Column::MarketplaceRef.eq(marketplace_ref))
        .filter(Column::IsDeleted.eq(false))
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(items)
}

pub async fn get_by_nomenclature_ref(
    nomenclature_ref: &str,
) -> anyhow::Result<Vec<MarketplaceProduct>> {
    let items: Vec<MarketplaceProduct> = Entity::find()
        .filter(Column::NomenclatureRef.eq(nomenclature_ref))
        .filter(Column::IsDeleted.eq(false))
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(items)
}
