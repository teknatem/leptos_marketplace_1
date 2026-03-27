use anyhow::Result;
use sea_orm::entity::prelude::*;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Select, Set,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sys_general_ledger")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub posting_id: String,
    pub entry_date: String,
    pub layer: String,
    pub registrator_type: String,
    pub registrator_ref: String,
    pub debit_account: String,
    pub credit_account: String,
    pub amount: f64,
    #[sea_orm(nullable)]
    pub qty: Option<f64>,
    pub turnover_code: String,
    pub detail_kind: String,
    pub detail_id: String,
    pub resource_name: String,
    pub resource_sign: i32,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

pub async fn save_entry(entry: &Model) -> Result<()> {
    save_entry_with_conn(conn(), entry).await
}

pub async fn save_entry_with_conn<C: ConnectionTrait>(db: &C, entry: &Model) -> Result<()> {
    let active = ActiveModel {
        id: Set(entry.id.clone()),
        posting_id: Set(entry.posting_id.clone()),
        entry_date: Set(entry.entry_date.clone()),
        layer: Set(entry.layer.clone()),
        registrator_type: Set(entry.registrator_type.clone()),
        registrator_ref: Set(entry.registrator_ref.clone()),
        debit_account: Set(entry.debit_account.clone()),
        credit_account: Set(entry.credit_account.clone()),
        amount: Set(entry.amount),
        qty: Set(entry.qty),
        turnover_code: Set(entry.turnover_code.clone()),
        detail_kind: Set(entry.detail_kind.clone()),
        detail_id: Set(entry.detail_id.clone()),
        resource_name: Set(entry.resource_name.clone()),
        resource_sign: Set(entry.resource_sign),
        created_at: Set(entry.created_at.clone()),
    };

    if Entity::find_by_id(entry.id.clone())
        .one(db)
        .await?
        .is_some()
    {
        active.update(db).await?;
    } else {
        active.insert(db).await?;
    }

    Ok(())
}

pub async fn delete_by_registrator_ref(registrator_ref: &str) -> Result<u64> {
    let result = Entity::delete_many()
        .filter(Column::RegistratorRef.eq(registrator_ref))
        .exec(conn())
        .await?;
    Ok(result.rows_affected)
}

pub async fn delete_by_posting_id(posting_id: &str) -> Result<u64> {
    let result = Entity::delete_many()
        .filter(Column::PostingId.eq(posting_id))
        .exec(conn())
        .await?;
    Ok(result.rows_affected)
}

pub async fn get_by_id(id: &str) -> Result<Option<Model>> {
    Ok(Entity::find_by_id(id.to_string()).one(conn()).await?)
}

pub async fn list_by_detail_kind_and_detail_id(
    detail_kind: &str,
    detail_id: &str,
) -> Result<Vec<Model>> {
    Ok(Entity::find()
        .filter(Column::DetailKind.eq(detail_kind))
        .filter(Column::DetailId.eq(detail_id))
        .order_by_asc(Column::CreatedAt)
        .order_by_asc(Column::Id)
        .all(conn())
        .await?)
}

pub async fn delete_by_detail_ids(detail_kind: &str, detail_ids: &[String]) -> Result<u64> {
    delete_by_detail_ids_with_conn(conn(), detail_kind, detail_ids).await
}

pub async fn delete_by_detail_ids_with_conn<C: ConnectionTrait>(
    db: &C,
    detail_kind: &str,
    detail_ids: &[String],
) -> Result<u64> {
    if detail_ids.is_empty() {
        return Ok(0);
    }

    let result = Entity::delete_many()
        .filter(Column::DetailKind.eq(detail_kind))
        .filter(Column::DetailId.is_in(detail_ids.iter().cloned()))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

pub async fn count_by_detail_ids(
    detail_kind: &str,
    detail_ids: &[String],
) -> Result<HashMap<String, usize>> {
    if detail_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = Entity::find()
        .filter(Column::DetailKind.eq(detail_kind))
        .filter(Column::DetailId.is_in(detail_ids.iter().cloned()))
        .all(conn())
        .await?;

    let mut counts = HashMap::new();
    for row in rows {
        *counts.entry(row.detail_id).or_insert(0) += 1;
    }

    Ok(counts)
}

#[allow(clippy::too_many_arguments)]
pub async fn list_with_filters(
    date_from: Option<String>,
    date_to: Option<String>,
    registrator_ref: Option<String>,
    registrator_type: Option<String>,
    layer: Option<String>,
    debit_account: Option<String>,
    credit_account: Option<String>,
    turnover_code: Option<String>,
    sort_by: Option<String>,
    sort_desc: bool,
    offset: Option<u64>,
    limit: Option<u64>,
) -> Result<Vec<Model>> {
    let mut query = apply_filters(
        Entity::find(),
        &date_from,
        &date_to,
        &registrator_ref,
        &registrator_type,
        &layer,
        &debit_account,
        &credit_account,
        &turnover_code,
    );
    query = apply_sort(query, sort_by.as_deref().unwrap_or("entry_date"), sort_desc);
    if let Some(value) = offset {
        query = query.offset(value);
    }
    if let Some(value) = limit {
        query = query.limit(value);
    }
    Ok(query.all(conn()).await?)
}

#[allow(clippy::too_many_arguments)]
pub async fn count_with_filters(
    date_from: Option<String>,
    date_to: Option<String>,
    registrator_ref: Option<String>,
    registrator_type: Option<String>,
    layer: Option<String>,
    debit_account: Option<String>,
    credit_account: Option<String>,
    turnover_code: Option<String>,
) -> Result<u64> {
    let query = apply_filters(
        Entity::find(),
        &date_from,
        &date_to,
        &registrator_ref,
        &registrator_type,
        &layer,
        &debit_account,
        &credit_account,
        &turnover_code,
    );
    Ok(query.count(conn()).await?)
}

#[allow(clippy::too_many_arguments)]
fn apply_filters(
    mut query: Select<Entity>,
    date_from: &Option<String>,
    date_to: &Option<String>,
    registrator_ref: &Option<String>,
    registrator_type: &Option<String>,
    layer: &Option<String>,
    debit_account: &Option<String>,
    credit_account: &Option<String>,
    turnover_code: &Option<String>,
) -> Select<Entity> {
    if let Some(v) = date_from {
        query = query.filter(Column::EntryDate.gte(v.clone()));
    }
    if let Some(v) = date_to {
        query = query.filter(Column::EntryDate.lte(v.clone()));
    }
    if let Some(v) = registrator_ref {
        query = query.filter(Column::RegistratorRef.eq(v.clone()));
    }
    if let Some(v) = registrator_type {
        query = query.filter(Column::RegistratorType.eq(v.clone()));
    }
    if let Some(v) = layer {
        query = query.filter(Column::Layer.eq(v.clone()));
    }
    if let Some(v) = debit_account {
        query = query.filter(Column::DebitAccount.eq(v.clone()));
    }
    if let Some(v) = credit_account {
        query = query.filter(Column::CreditAccount.eq(v.clone()));
    }
    if let Some(v) = turnover_code {
        query = query.filter(Column::TurnoverCode.eq(v.clone()));
    }
    query
}

fn apply_sort(mut query: Select<Entity>, sort_by: &str, sort_desc: bool) -> Select<Entity> {
    match (sort_by, sort_desc) {
        ("entry_date", true) => query = query.order_by_desc(Column::EntryDate),
        ("entry_date", false) => query = query.order_by_asc(Column::EntryDate),
        ("layer", true) => query = query.order_by_desc(Column::Layer),
        ("layer", false) => query = query.order_by_asc(Column::Layer),
        ("debit_account", true) => query = query.order_by_desc(Column::DebitAccount),
        ("debit_account", false) => query = query.order_by_asc(Column::DebitAccount),
        ("credit_account", true) => query = query.order_by_desc(Column::CreditAccount),
        ("credit_account", false) => query = query.order_by_asc(Column::CreditAccount),
        ("amount", true) => query = query.order_by_desc(Column::Amount),
        ("amount", false) => query = query.order_by_asc(Column::Amount),
        ("qty", true) => query = query.order_by_desc(Column::Qty),
        ("qty", false) => query = query.order_by_asc(Column::Qty),
        ("turnover_code", true) => query = query.order_by_desc(Column::TurnoverCode),
        ("turnover_code", false) => query = query.order_by_asc(Column::TurnoverCode),
        ("registrator_type", true) => query = query.order_by_desc(Column::RegistratorType),
        ("registrator_type", false) => query = query.order_by_asc(Column::RegistratorType),
        ("registrator_ref", true) => query = query.order_by_desc(Column::RegistratorRef),
        ("registrator_ref", false) => query = query.order_by_asc(Column::RegistratorRef),
        ("detail_kind", true) => query = query.order_by_desc(Column::DetailKind),
        ("detail_kind", false) => query = query.order_by_asc(Column::DetailKind),
        ("created_at", true) => query = query.order_by_desc(Column::CreatedAt),
        ("created_at", false) => query = query.order_by_asc(Column::CreatedAt),
        _ if sort_desc => query = query.order_by_desc(Column::EntryDate),
        _ => query = query.order_by_asc(Column::EntryDate),
    }

    query
}
