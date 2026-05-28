use anyhow::Result;
use chrono::Utc;
use contracts::domain::a033_wb_day_close::{
    WbDayClose, WbDayCloseAdvertNoOrderLine, WbDayCloseAdvertOrderAccrualLine, WbDayCloseId,
    WbDayCloseLine, WbDayCloseProblem, WbDayCloseTotals,
};
use contracts::domain::common::{AggregateId, BaseAggregate, EntityMetadata};
use sea_orm::entity::prelude::*;
use sea_orm::{
    ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::data::db::get_connection;

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

// ─────────────────────────────────────────────────────────────────────────────
// Sea-ORM Model
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a033_wb_day_close")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub connection_id: String,
    pub business_date: String,
    pub is_archived: bool,
    pub archived_at: Option<String>,
    pub archived_reason: Option<String>,
    pub replaces_id: Option<String>,
    pub last_recalculated_at: Option<String>,
    pub snapshot_hash: String,
    pub lines_json: String,
    pub problems_json: String,
    pub totals_json: String,
    #[sea_orm(default_value = "[]")]
    pub advert_no_order_json: String,
    #[sea_orm(default_value = "[]")]
    pub advert_order_accrual_json: String,
    #[sea_orm(default_value = "0")]
    pub gl_advert_no_order: f64,
    #[sea_orm(default_value = "0")]
    pub gl_advert_order_accrual: f64,
    #[sea_orm(default_value = "0")]
    pub gl_advert_order_expense: f64,
    #[sea_orm(default_value = "0")]
    pub snap_advert_order_expense: f64,
    pub is_deleted: bool,
    pub is_posted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// ─────────────────────────────────────────────────────────────────────────────
// Conversion Model → WbDayClose
// ─────────────────────────────────────────────────────────────────────────────

impl From<Model> for WbDayClose {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());

        let lines: Vec<WbDayCloseLine> = serde_json::from_str(&m.lines_json).unwrap_or_default();
        let problems: Vec<WbDayCloseProblem> =
            serde_json::from_str(&m.problems_json).unwrap_or_default();
        let totals: WbDayCloseTotals = serde_json::from_str(&m.totals_json).unwrap_or_default();
        let advert_clicks_no_order_lines: Vec<WbDayCloseAdvertNoOrderLine> =
            serde_json::from_str(&m.advert_no_order_json).unwrap_or_default();
        let advert_clicks_order_accrual_lines: Vec<WbDayCloseAdvertOrderAccrualLine> =
            serde_json::from_str(&m.advert_order_accrual_json).unwrap_or_default();

        WbDayClose {
            base: BaseAggregate::with_metadata(
                WbDayCloseId::new(uuid),
                m.code,
                m.description,
                m.comment,
                metadata,
            ),
            connection_id: m.connection_id,
            business_date: m.business_date,
            is_archived: m.is_archived,
            archived_at: m.archived_at,
            archived_reason: m.archived_reason,
            replaces_id: m.replaces_id,
            last_recalculated_at: m.last_recalculated_at,
            snapshot_hash: m.snapshot_hash,
            lines,
            problems,
            totals,
            advert_clicks_no_order_lines,
            advert_clicks_order_accrual_lines,
            gl_advert_no_order: m.gl_advert_no_order,
            gl_advert_order_accrual: m.gl_advert_order_accrual,
            gl_advert_order_expense: m.gl_advert_order_expense,
            snap_advert_order_expense: m.snap_advert_order_expense,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn to_active_model(doc: &WbDayClose) -> ActiveModel {
    let lines_json = serde_json::to_string(&doc.lines).unwrap_or_else(|_| "[]".to_string());
    let problems_json = serde_json::to_string(&doc.problems).unwrap_or_else(|_| "[]".to_string());
    let totals_json = serde_json::to_string(&doc.totals).unwrap_or_else(|_| "{}".to_string());
    let advert_no_order_json = serde_json::to_string(&doc.advert_clicks_no_order_lines)
        .unwrap_or_else(|_| "[]".to_string());
    let advert_order_accrual_json = serde_json::to_string(&doc.advert_clicks_order_accrual_lines)
        .unwrap_or_else(|_| "[]".to_string());

    ActiveModel {
        id: Set(doc.base.id.as_string()),
        code: Set(doc.base.code.clone()),
        description: Set(doc.base.description.clone()),
        comment: Set(doc.base.comment.clone()),
        connection_id: Set(doc.connection_id.clone()),
        business_date: Set(doc.business_date.clone()),
        is_archived: Set(doc.is_archived),
        archived_at: Set(doc.archived_at.clone()),
        archived_reason: Set(doc.archived_reason.clone()),
        replaces_id: Set(doc.replaces_id.clone()),
        last_recalculated_at: Set(doc.last_recalculated_at.clone()),
        snapshot_hash: Set(doc.snapshot_hash.clone()),
        lines_json: Set(lines_json),
        problems_json: Set(problems_json),
        totals_json: Set(totals_json),
        advert_no_order_json: Set(advert_no_order_json),
        advert_order_accrual_json: Set(advert_order_accrual_json),
        gl_advert_no_order: Set(doc.gl_advert_no_order),
        gl_advert_order_accrual: Set(doc.gl_advert_order_accrual),
        gl_advert_order_expense: Set(doc.gl_advert_order_expense),
        snap_advert_order_expense: Set(doc.snap_advert_order_expense),
        is_deleted: Set(doc.base.metadata.is_deleted),
        is_posted: Set(doc.base.metadata.is_posted),
        created_at: Set(Some(doc.base.metadata.created_at)),
        updated_at: Set(Some(doc.base.metadata.updated_at)),
        version: Set(doc.base.metadata.version),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CRUD
// ─────────────────────────────────────────────────────────────────────────────

pub async fn insert(doc: &WbDayClose) -> Result<()> {
    let am = to_active_model(doc);
    Entity::insert(am).exec(conn()).await?;
    Ok(())
}

pub async fn update(doc: &WbDayClose) -> Result<()> {
    let am = to_active_model(doc);
    Entity::update(am).exec(conn()).await?;
    Ok(())
}

/// Атомарная архивация + вставка нового — в одной транзакции.
pub async fn archive_and_insert_new(old: &WbDayClose, new: &WbDayClose) -> Result<()> {
    let db = conn();
    let tx = db.begin().await?;

    let old_am = to_active_model(old);
    Entity::update(old_am).exec(&tx).await?;

    let new_am = to_active_model(new);
    Entity::insert(new_am).exec(&tx).await?;

    tx.commit().await?;
    Ok(())
}

pub async fn get_by_id(id: Uuid) -> Result<Option<WbDayClose>> {
    Ok(Entity::find_by_id(id.to_string())
        .filter(Column::IsDeleted.eq(false))
        .one(conn())
        .await?
        .map(Into::into))
}

/// Активный (не архивный) документ за (connection_id, business_date).
pub async fn get_active(connection_id: &str, business_date: &str) -> Result<Option<WbDayClose>> {
    Ok(Entity::find()
        .filter(Column::ConnectionId.eq(connection_id))
        .filter(Column::BusinessDate.eq(business_date))
        .filter(Column::IsArchived.eq(false))
        .filter(Column::IsDeleted.eq(false))
        .one(conn())
        .await?
        .map(Into::into))
}

/// Все архивные версии за (connection_id, business_date), по убыванию даты создания.
pub async fn list_archived(connection_id: &str, business_date: &str) -> Result<Vec<WbDayClose>> {
    Ok(Entity::find()
        .filter(Column::ConnectionId.eq(connection_id))
        .filter(Column::BusinessDate.eq(business_date))
        .filter(Column::IsArchived.eq(true))
        .filter(Column::IsDeleted.eq(false))
        .order_by_desc(Column::CreatedAt)
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
}

// ─────────────────────────────────────────────────────────────────────────────
// Paginated list
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct ListQuery {
    pub connection_id: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub include_archived: Option<bool>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

pub async fn list_paginated(query: ListQuery) -> Result<Vec<WbDayClose>> {
    let mut q = Entity::find().filter(Column::IsDeleted.eq(false));

    if let Some(cid) = query.connection_id {
        q = q.filter(Column::ConnectionId.eq(cid));
    }
    if let Some(d) = query.date_from {
        q = q.filter(Column::BusinessDate.gte(d));
    }
    if let Some(d) = query.date_to {
        q = q.filter(Column::BusinessDate.lte(d));
    }
    if !query.include_archived.unwrap_or(true) {
        q = q.filter(Column::IsArchived.eq(false));
    }

    q = q
        .order_by_desc(Column::BusinessDate)
        .order_by_desc(Column::CreatedAt);

    if let Some(off) = query.offset {
        q = q.offset(off);
    }
    if let Some(lim) = query.limit {
        q = q.limit(lim);
    }

    Ok(q.all(conn()).await?.into_iter().map(Into::into).collect())
}

/// Все документы за день (активный + архивные), для compare.
pub async fn list_by_day(connection_id: &str, business_date: &str) -> Result<Vec<WbDayClose>> {
    Ok(Entity::find()
        .filter(Column::ConnectionId.eq(connection_id))
        .filter(Column::BusinessDate.eq(business_date))
        .filter(Column::IsDeleted.eq(false))
        .order_by_asc(Column::IsArchived)
        .order_by_desc(Column::CreatedAt)
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
}
