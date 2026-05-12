use anyhow::Result;
use chrono::Utc;
use contracts::domain::a032_wb_returns_claims::aggregate::{WbReturnsClaims, WbReturnsClaimsId};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a032_wb_returns_claims")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub connection_id: String,
    pub organization_id: String,
    pub marketplace_id: String,
    pub claim_id: String,
    pub claim_type: Option<i32>,
    pub status: Option<i32>,
    pub status_ex: Option<i32>,
    pub nm_id: i64,
    pub imt_name: Option<String>,
    pub user_comment: Option<String>,
    pub wb_comment: Option<String>,
    pub dt: String,
    pub order_dt: Option<String>,
    pub dt_update: Option<String>,
    pub delivery_dt: Option<String>,
    pub price: Option<f64>,
    pub currency_code: Option<String>,
    pub srid: Option<String>,
    pub origin_id_info: Option<String>,
    pub actions: Option<String>,
    pub is_archive: bool,
    pub is_deleted: bool,
    pub is_posted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for WbReturnsClaims {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());

        let parse_dt = |s: &str| {
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now())
        };
        let parse_dt_opt = |s: Option<String>| {
            s.and_then(|v| {
                chrono::DateTime::parse_from_rfc3339(&v)
                    .ok()
                    .map(|d| d.with_timezone(&Utc))
            })
        };

        WbReturnsClaims {
            base: BaseAggregate::with_metadata(
                WbReturnsClaimsId(uuid),
                m.code,
                m.description,
                m.comment,
                metadata,
            ),
            connection_id: m.connection_id,
            organization_id: m.organization_id,
            marketplace_id: m.marketplace_id,
            claim_id: m.claim_id,
            claim_type: m.claim_type,
            status: m.status,
            status_ex: m.status_ex,
            nm_id: m.nm_id,
            imt_name: m.imt_name,
            user_comment: m.user_comment,
            wb_comment: m.wb_comment,
            dt: parse_dt(&m.dt),
            order_dt: parse_dt_opt(m.order_dt),
            dt_update: parse_dt_opt(m.dt_update),
            delivery_dt: parse_dt_opt(m.delivery_dt),
            price: m.price,
            currency_code: m.currency_code,
            srid: m.srid,
            origin_id_info: m.origin_id_info,
            actions: m.actions,
            is_archive: m.is_archive,
        }
    }
}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

pub async fn get_by_id(id: Uuid) -> Result<Option<WbReturnsClaims>> {
    let result = Entity::find_by_id(id.to_string()).one(conn()).await?;
    Ok(result.map(Into::into))
}

pub async fn get_by_claim_key(
    connection_id: &str,
    claim_id: &str,
) -> Result<Option<WbReturnsClaims>> {
    let result = Entity::find()
        .filter(Column::ConnectionId.eq(connection_id))
        .filter(Column::ClaimId.eq(claim_id))
        .filter(Column::IsDeleted.eq(false))
        .one(conn())
        .await?;
    Ok(result.map(Into::into))
}

pub async fn list_all() -> Result<Vec<WbReturnsClaims>> {
    let mut items: Vec<WbReturnsClaims> = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    items.sort_by(|a, b| b.dt.cmp(&a.dt));
    Ok(items)
}

pub async fn list_by_connection(connection_id: &str) -> Result<Vec<WbReturnsClaims>> {
    let mut items: Vec<WbReturnsClaims> = Entity::find()
        .filter(Column::ConnectionId.eq(connection_id))
        .filter(Column::IsDeleted.eq(false))
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    items.sort_by(|a, b| b.dt.cmp(&a.dt));
    Ok(items)
}

pub async fn insert(agg: &WbReturnsClaims) -> Result<Uuid> {
    let uuid = agg.base.id.value();
    let active = ActiveModel {
        id: Set(uuid.to_string()),
        code: Set(agg.base.code.clone()),
        description: Set(agg.base.description.clone()),
        comment: Set(agg.base.comment.clone()),
        connection_id: Set(agg.connection_id.clone()),
        organization_id: Set(agg.organization_id.clone()),
        marketplace_id: Set(agg.marketplace_id.clone()),
        claim_id: Set(agg.claim_id.clone()),
        claim_type: Set(agg.claim_type),
        status: Set(agg.status),
        status_ex: Set(agg.status_ex),
        nm_id: Set(agg.nm_id),
        imt_name: Set(agg.imt_name.clone()),
        user_comment: Set(agg.user_comment.clone()),
        wb_comment: Set(agg.wb_comment.clone()),
        dt: Set(agg.dt.to_rfc3339()),
        order_dt: Set(agg.order_dt.map(|d| d.to_rfc3339())),
        dt_update: Set(agg.dt_update.map(|d| d.to_rfc3339())),
        delivery_dt: Set(agg.delivery_dt.map(|d| d.to_rfc3339())),
        price: Set(agg.price),
        currency_code: Set(agg.currency_code.clone()),
        srid: Set(agg.srid.clone()),
        origin_id_info: Set(agg.origin_id_info.clone()),
        actions: Set(agg.actions.clone()),
        is_archive: Set(agg.is_archive),
        is_deleted: Set(agg.base.metadata.is_deleted),
        is_posted: Set(agg.base.metadata.is_posted),
        created_at: Set(Some(agg.base.metadata.created_at)),
        updated_at: Set(Some(agg.base.metadata.updated_at)),
        version: Set(agg.base.metadata.version),
    };
    active.insert(conn()).await?;
    Ok(uuid)
}

pub async fn update(agg: &WbReturnsClaims) -> Result<()> {
    let active = ActiveModel {
        id: Set(agg.base.id.value().to_string()),
        code: Set(agg.base.code.clone()),
        description: Set(agg.base.description.clone()),
        comment: Set(agg.base.comment.clone()),
        connection_id: Set(agg.connection_id.clone()),
        organization_id: Set(agg.organization_id.clone()),
        marketplace_id: Set(agg.marketplace_id.clone()),
        claim_id: Set(agg.claim_id.clone()),
        claim_type: Set(agg.claim_type),
        status: Set(agg.status),
        status_ex: Set(agg.status_ex),
        nm_id: Set(agg.nm_id),
        imt_name: Set(agg.imt_name.clone()),
        user_comment: Set(agg.user_comment.clone()),
        wb_comment: Set(agg.wb_comment.clone()),
        dt: Set(agg.dt.to_rfc3339()),
        order_dt: Set(agg.order_dt.map(|d| d.to_rfc3339())),
        dt_update: Set(agg.dt_update.map(|d| d.to_rfc3339())),
        delivery_dt: Set(agg.delivery_dt.map(|d| d.to_rfc3339())),
        price: Set(agg.price),
        currency_code: Set(agg.currency_code.clone()),
        srid: Set(agg.srid.clone()),
        origin_id_info: Set(agg.origin_id_info.clone()),
        actions: Set(agg.actions.clone()),
        is_archive: Set(agg.is_archive),
        is_deleted: Set(agg.base.metadata.is_deleted),
        is_posted: Set(agg.base.metadata.is_posted),
        created_at: sea_orm::ActiveValue::NotSet,
        updated_at: Set(Some(agg.base.metadata.updated_at)),
        version: Set(agg.base.metadata.version),
    };
    active.update(conn()).await?;
    Ok(())
}

/// Upsert by (connection_id, claim_id) — основной метод для загрузки из API.
/// Возвращает (uuid, was_inserted).
pub async fn upsert_by_claim_key(agg: &WbReturnsClaims) -> Result<(Uuid, bool)> {
    match get_by_claim_key(&agg.connection_id, &agg.claim_id).await? {
        None => {
            let uuid = insert(agg).await?;
            Ok((uuid, true))
        }
        Some(existing) => {
            let mut updated = agg.clone();
            updated.base.id = existing.base.id;
            updated.base.metadata.created_at = existing.base.metadata.created_at;
            updated.base.metadata.version = existing.base.metadata.version + 1;
            update(&updated).await?;
            Ok((existing.base.id.value(), false))
        }
    }
}
