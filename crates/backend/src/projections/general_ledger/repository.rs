use anyhow::Result;
use sea_orm::entity::prelude::*;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Select, Set,
    Statement,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sys_general_ledger")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub entry_date: String,
    pub layer: String,
    #[sea_orm(nullable)]
    pub cabinet_mp: Option<String>,
    pub registrator_type: String,
    pub registrator_ref: String,
    pub debit_account: String,
    pub credit_account: String,
    pub amount: f64,
    #[sea_orm(nullable)]
    pub qty: Option<f64>,
    pub turnover_code: String,
    pub resource_table: String,
    pub resource_field: String,
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
        entry_date: Set(entry.entry_date.clone()),
        layer: Set(entry.layer.clone()),
        cabinet_mp: Set(entry.cabinet_mp.clone()),
        registrator_type: Set(entry.registrator_type.clone()),
        registrator_ref: Set(entry.registrator_ref.clone()),
        debit_account: Set(entry.debit_account.clone()),
        credit_account: Set(entry.credit_account.clone()),
        amount: Set(entry.amount),
        qty: Set(entry.qty),
        turnover_code: Set(entry.turnover_code.clone()),
        resource_table: Set(entry.resource_table.clone()),
        resource_field: Set(entry.resource_field.clone()),
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
    let _ = posting_id;
    Ok(0)
}

pub async fn get_by_id(id: &str) -> Result<Option<Model>> {
    Ok(Entity::find_by_id(id.to_string()).one(conn()).await?)
}

pub async fn list_by_registrator_ref(registrator_ref: &str) -> Result<Vec<Model>> {
    Ok(Entity::find()
        .filter(Column::RegistratorRef.eq(registrator_ref))
        .order_by_asc(Column::CreatedAt)
        .order_by_asc(Column::Id)
        .all(conn())
        .await?)
}

pub async fn delete_by_registrator_refs(registrator_refs: &[String]) -> Result<u64> {
    delete_by_registrator_refs_with_conn(conn(), registrator_refs).await
}

pub async fn delete_by_registrator_refs_with_conn<C: ConnectionTrait>(
    db: &C,
    registrator_refs: &[String],
) -> Result<u64> {
    if registrator_refs.is_empty() {
        return Ok(0);
    }

    let result = Entity::delete_many()
        .filter(Column::RegistratorRef.is_in(registrator_refs.iter().cloned()))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

pub async fn count_by_registrator_refs(registrator_refs: &[String]) -> Result<HashMap<String, usize>> {
    if registrator_refs.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = Entity::find()
        .filter(Column::RegistratorRef.is_in(registrator_refs.iter().cloned()))
        .all(conn())
        .await?;

    let mut counts = HashMap::new();
    for row in rows {
        *counts.entry(row.registrator_ref).or_insert(0) += 1;
    }

    Ok(counts)
}

pub async fn count_grouped_by_turnover_code() -> Result<HashMap<String, usize>> {
    let stmt = Statement::from_string(
        conn().get_database_backend(),
        r#"
            SELECT turnover_code, COUNT(*) AS gl_entries_count
            FROM sys_general_ledger
            GROUP BY turnover_code
        "#
        .to_string(),
    );

    let rows = conn().query_all(stmt).await?;
    let mut counts = HashMap::new();

    for row in rows {
        let code: String = row.try_get("", "turnover_code")?;
        let count: i64 = row.try_get("", "gl_entries_count")?;
        counts.insert(code, count.max(0) as usize);
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
    cabinet_mp: Option<String>,
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
        &cabinet_mp,
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
    cabinet_mp: Option<String>,
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
        &cabinet_mp,
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
    cabinet_mp: &Option<String>,
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
    if let Some(v) = cabinet_mp {
        query = query.filter(Column::CabinetMp.eq(v.clone()));
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
        ("cabinet_mp", true) => query = query.order_by_desc(Column::CabinetMp),
        ("cabinet_mp", false) => query = query.order_by_asc(Column::CabinetMp),
        ("registrator_type", true) => query = query.order_by_desc(Column::RegistratorType),
        ("registrator_type", false) => query = query.order_by_asc(Column::RegistratorType),
        ("registrator_ref", true) => query = query.order_by_desc(Column::RegistratorRef),
        ("registrator_ref", false) => query = query.order_by_asc(Column::RegistratorRef),
        ("resource_table", true) => query = query.order_by_desc(Column::ResourceTable),
        ("resource_table", false) => query = query.order_by_asc(Column::ResourceTable),
        ("resource_field", true) => query = query.order_by_desc(Column::ResourceField),
        ("resource_field", false) => query = query.order_by_asc(Column::ResourceField),
        ("created_at", true) => query = query.order_by_desc(Column::CreatedAt),
        ("created_at", false) => query = query.order_by_asc(Column::CreatedAt),
        _ if sort_desc => query = query.order_by_desc(Column::EntryDate),
        _ => query = query.order_by_asc(Column::EntryDate),
    }

    query
}
