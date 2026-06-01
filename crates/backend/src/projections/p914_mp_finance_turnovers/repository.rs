use anyhow::Result;
use sea_orm::entity::prelude::*;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Select, Set,
};
use serde::{Deserialize, Serialize};

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "p914_mp_finance_turnovers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub transaction_date: String,
    #[sea_orm(nullable)]
    pub general_ledger_ref: Option<String>,
    pub registrator_type: String,
    pub registrator_ref: String,
    pub connection_mp_ref: String,
    #[sea_orm(nullable)]
    pub nomenclature_ref: Option<String>,
    #[sea_orm(nullable)]
    pub marketplace_product_ref: Option<String>,
    pub turnover_code: String,
    pub order_key: String,
    #[sea_orm(nullable)]
    pub order_ref: Option<String>,
    #[sea_orm(nullable)]
    pub order_registrator_type: Option<String>,
    pub event_kind: String,
    #[sea_orm(nullable)]
    pub customer_kind: Option<String>,
    #[sea_orm(nullable)]
    pub fulfillment_type: Option<String>,
    pub layer: String,
    pub amount: f64,
    #[sea_orm(nullable)]
    pub quantity: Option<f64>,
    pub created_at_msk: String,
    pub updated_at_msk: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

fn active_from_model(entry: &Model) -> ActiveModel {
    ActiveModel {
        id: Set(entry.id.clone()),
        transaction_date: Set(entry.transaction_date.clone()),
        general_ledger_ref: Set(entry.general_ledger_ref.clone()),
        registrator_type: Set(entry.registrator_type.clone()),
        registrator_ref: Set(entry.registrator_ref.clone()),
        connection_mp_ref: Set(entry.connection_mp_ref.clone()),
        nomenclature_ref: Set(entry.nomenclature_ref.clone()),
        marketplace_product_ref: Set(entry.marketplace_product_ref.clone()),
        turnover_code: Set(entry.turnover_code.clone()),
        order_key: Set(entry.order_key.clone()),
        order_ref: Set(entry.order_ref.clone()),
        order_registrator_type: Set(entry.order_registrator_type.clone()),
        event_kind: Set(entry.event_kind.clone()),
        customer_kind: Set(entry.customer_kind.clone()),
        fulfillment_type: Set(entry.fulfillment_type.clone()),
        layer: Set(entry.layer.clone()),
        amount: Set(entry.amount),
        quantity: Set(entry.quantity),
        created_at_msk: Set(entry.created_at_msk.clone()),
        updated_at_msk: Set(entry.updated_at_msk.clone()),
    }
}

pub async fn get_by_id(id: &str) -> Result<Option<Model>> {
    Ok(Entity::find_by_id(id.to_string()).one(conn()).await?)
}

/// Все строки оборотов, относящиеся к одному регистратору (строке-источнику
/// финансового отчёта МП). Используется на детальной странице регистратора.
pub async fn list_by_registrator(
    registrator_type: &str,
    registrator_ref: &str,
) -> Result<Vec<Model>> {
    Ok(Entity::find()
        .filter(Column::RegistratorType.eq(registrator_type))
        .filter(Column::RegistratorRef.eq(registrator_ref))
        .order_by_asc(Column::TurnoverCode)
        .all(conn())
        .await?)
}

/// Upsert одной строки (с проверкой существования).
pub async fn save_entry(entry: &Model) -> Result<()> {
    save_entry_with_conn(conn(), entry).await
}

pub async fn save_entry_with_conn<C: ConnectionTrait>(db: &C, entry: &Model) -> Result<()> {
    if Entity::find_by_id(entry.id.clone()).one(db).await?.is_some() {
        active_from_model(entry).update(db).await?;
    } else {
        active_from_model(entry).insert(db).await?;
    }
    Ok(())
}

/// Прямой INSERT без SELECT-проверки. Используется в batch-контексте
/// перепроведения, где строки предварительно удалены.
pub async fn insert_entry_raw_with_conn<C: ConnectionTrait>(db: &C, entry: &Model) -> Result<()> {
    active_from_model(entry).insert(db).await?;
    Ok(())
}

pub async fn delete_by_registrator_ref(registrator_ref: &str) -> Result<u64> {
    let result = Entity::delete_many()
        .filter(Column::RegistratorRef.eq(registrator_ref))
        .exec(conn())
        .await?;
    Ok(result.rows_affected)
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

#[allow(clippy::too_many_arguments)]
fn apply_filters(
    mut query: Select<Entity>,
    date_from: &Option<String>,
    date_to: &Option<String>,
    connection_mp_ref: &Option<String>,
    registrator_type: &Option<String>,
    turnover_code: &Option<String>,
    order_key: &Option<String>,
    event_kind: &Option<String>,
) -> Select<Entity> {
    if let Some(value) = date_from {
        query = query.filter(Column::TransactionDate.gte(value.clone()));
    }
    if let Some(value) = date_to {
        query = query.filter(Column::TransactionDate.lte(value.clone()));
    }
    if let Some(value) = connection_mp_ref {
        query = query.filter(Column::ConnectionMpRef.eq(value.clone()));
    }
    if let Some(value) = registrator_type {
        query = query.filter(Column::RegistratorType.eq(value.clone()));
    }
    if let Some(value) = turnover_code {
        query = query.filter(Column::TurnoverCode.eq(value.clone()));
    }
    if let Some(value) = order_key {
        query = query.filter(Column::OrderKey.eq(value.clone()));
    }
    if let Some(value) = event_kind {
        query = query.filter(Column::EventKind.eq(value.clone()));
    }
    query
}

fn apply_sort(mut query: Select<Entity>, sort_by: Option<&str>, sort_desc: bool) -> Select<Entity> {
    let column = match sort_by.unwrap_or("transaction_date") {
        "transaction_date" => Column::TransactionDate,
        "turnover_code" => Column::TurnoverCode,
        "order_key" => Column::OrderKey,
        "event_kind" => Column::EventKind,
        "amount" => Column::Amount,
        "registrator_type" => Column::RegistratorType,
        _ => Column::TransactionDate,
    };

    query = if sort_desc {
        query.order_by_desc(column)
    } else {
        query.order_by_asc(column)
    };

    if matches!(column, Column::TransactionDate) {
        query.order_by_desc(Column::Id)
    } else {
        query.order_by_desc(Column::TransactionDate)
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn list_with_filters(
    date_from: Option<String>,
    date_to: Option<String>,
    connection_mp_ref: Option<String>,
    registrator_type: Option<String>,
    turnover_code: Option<String>,
    order_key: Option<String>,
    event_kind: Option<String>,
    sort_by: Option<String>,
    sort_desc: bool,
    offset: Option<u64>,
    limit: Option<u64>,
) -> Result<Vec<Model>> {
    let mut query = apply_filters(
        Entity::find(),
        &date_from,
        &date_to,
        &connection_mp_ref,
        &registrator_type,
        &turnover_code,
        &order_key,
        &event_kind,
    );
    query = apply_sort(query, sort_by.as_deref(), sort_desc);
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
    connection_mp_ref: Option<String>,
    registrator_type: Option<String>,
    turnover_code: Option<String>,
    order_key: Option<String>,
    event_kind: Option<String>,
) -> Result<u64> {
    let query = apply_filters(
        Entity::find(),
        &date_from,
        &date_to,
        &connection_mp_ref,
        &registrator_type,
        &turnover_code,
        &order_key,
        &event_kind,
    );
    Ok(query.count(conn()).await?)
}
