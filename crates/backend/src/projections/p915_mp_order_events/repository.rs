//! Репозиторий проекции `p915_mp_order_events` (таймлайн событий заказа).
//!
//! Наполняется push-ом из регистраторов при проведении: каждый регистратор
//! удаляет свои строки (delete-by-registrator) и вставляет заново. По образцу
//! `p914_mp_finance_turnovers::repository`.

use anyhow::Result;
use sea_orm::entity::prelude::*;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Select, Set,
};
use serde::{Deserialize, Serialize};

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "p915_mp_order_events")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub order_id: String,
    #[sea_orm(nullable)]
    pub marketplace_product: Option<String>,
    pub event_date: String,
    pub event_type: String,
    pub layer: String,
    #[sea_orm(nullable)]
    pub amount: Option<f64>,
    pub registrator_type: String,
    pub registrator_ref: String,
    pub connection_mp_ref: String,
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
        order_id: Set(entry.order_id.clone()),
        marketplace_product: Set(entry.marketplace_product.clone()),
        event_date: Set(entry.event_date.clone()),
        event_type: Set(entry.event_type.clone()),
        layer: Set(entry.layer.clone()),
        amount: Set(entry.amount),
        registrator_type: Set(entry.registrator_type.clone()),
        registrator_ref: Set(entry.registrator_ref.clone()),
        connection_mp_ref: Set(entry.connection_mp_ref.clone()),
        created_at_msk: Set(entry.created_at_msk.clone()),
        updated_at_msk: Set(entry.updated_at_msk.clone()),
    }
}

/// Прямой INSERT без SELECT-проверки. Используется в контексте проведения,
/// где строки регистратора предварительно удалены.
pub async fn insert_entry_raw_with_conn<C: ConnectionTrait>(db: &C, entry: &Model) -> Result<()> {
    active_from_model(entry).insert(db).await?;
    Ok(())
}

/// Удаление всех событий по ссылке регистратора (autocommit). Используется
/// при распроведении a013 (где нет внешней транзакции).
pub async fn delete_by_registrator_ref(registrator_ref: &str) -> Result<u64> {
    let result = Entity::delete_many()
        .filter(Column::RegistratorRef.eq(registrator_ref))
        .exec(conn())
        .await?;
    Ok(result.rows_affected)
}

/// Удаление событий по набору ссылок регистраторов в рамках транзакции.
/// Используется в пути перепроведения p907.
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

/// Удаление событий конкретного регистратора (по типу + ссылке) в рамках
/// транзакции. Используется при проведении/распроведении a013/a034.
pub async fn delete_by_registrator_with_conn<C: ConnectionTrait>(
    db: &C,
    registrator_type: &str,
    registrator_ref: &str,
) -> Result<u64> {
    let result = Entity::delete_many()
        .filter(Column::RegistratorType.eq(registrator_type))
        .filter(Column::RegistratorRef.eq(registrator_ref))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

/// Все события одного заказа, упорядоченные по дате и типу события.
/// Основной сценарий «быстрая выдача истории заказа».
pub async fn list_by_order_id(order_id: &str) -> Result<Vec<Model>> {
    Ok(Entity::find()
        .filter(Column::OrderId.eq(order_id))
        .order_by_asc(Column::EventDate)
        .order_by_asc(Column::EventType)
        .all(conn())
        .await?)
}

#[allow(clippy::too_many_arguments)]
fn apply_filters(
    mut query: Select<Entity>,
    date_from: &Option<String>,
    date_to: &Option<String>,
    connection_mp_ref: &Option<String>,
    order_id: &Option<String>,
    event_type: &Option<String>,
    registrator_type: &Option<String>,
    layer: &Option<String>,
) -> Select<Entity> {
    if let Some(value) = date_from {
        query = query.filter(Column::EventDate.gte(value.clone()));
    }
    if let Some(value) = date_to {
        query = query.filter(Column::EventDate.lte(value.clone()));
    }
    if let Some(value) = connection_mp_ref {
        query = query.filter(Column::ConnectionMpRef.eq(value.clone()));
    }
    if let Some(value) = order_id {
        query = query.filter(Column::OrderId.eq(value.clone()));
    }
    if let Some(value) = event_type {
        query = query.filter(Column::EventType.eq(value.clone()));
    }
    if let Some(value) = registrator_type {
        query = query.filter(Column::RegistratorType.eq(value.clone()));
    }
    if let Some(value) = layer {
        query = query.filter(Column::Layer.eq(value.clone()));
    }
    query
}

fn apply_sort(mut query: Select<Entity>, sort_by: Option<&str>, sort_desc: bool) -> Select<Entity> {
    let column = match sort_by.unwrap_or("event_date") {
        "event_date" => Column::EventDate,
        "event_type" => Column::EventType,
        "order_id" => Column::OrderId,
        "amount" => Column::Amount,
        _ => Column::EventDate,
    };

    query = if sort_desc {
        query.order_by_desc(column)
    } else {
        query.order_by_asc(column)
    };

    // Стабилизирующий вторичный порядок.
    if matches!(column, Column::EventDate) {
        query.order_by_desc(Column::Id)
    } else {
        query.order_by_desc(Column::EventDate)
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn list_with_filters(
    date_from: Option<String>,
    date_to: Option<String>,
    connection_mp_ref: Option<String>,
    order_id: Option<String>,
    event_type: Option<String>,
    registrator_type: Option<String>,
    layer: Option<String>,
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
        &order_id,
        &event_type,
        &registrator_type,
        &layer,
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
    order_id: Option<String>,
    event_type: Option<String>,
    registrator_type: Option<String>,
    layer: Option<String>,
) -> Result<u64> {
    let query = apply_filters(
        Entity::find(),
        &date_from,
        &date_to,
        &connection_mp_ref,
        &order_id,
        &event_type,
        &registrator_type,
        &layer,
    );
    Ok(query.count(conn()).await?)
}
