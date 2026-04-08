use anyhow::Result;
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Select, Set};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "p909_mp_order_line_turnovers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub connection_mp_ref: String,
    pub order_key: String,
    pub line_key: String,
    pub line_event_key: String,
    pub event_kind: String,
    pub entry_date: String,
    pub layer: String,
    pub turnover_code: String,
    pub value_kind: String,
    pub agg_kind: String,
    pub amount: f64,
    #[sea_orm(nullable)]
    pub nomenclature_ref: Option<String>,
    #[sea_orm(nullable)]
    pub marketplace_product_ref: Option<String>,
    pub registrator_type: String,
    pub registrator_ref: String,
    pub link_status: String,
    #[sea_orm(nullable)]
    pub general_ledger_ref: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LinkGroupKey {
    pub connection_mp_ref: String,
    pub line_event_key: String,
    pub turnover_code: String,
}

fn derive_link_status(layers: &BTreeSet<String>) -> String {
    let has_plan = layers.contains("plan");
    let has_oper = layers.contains("oper");
    let has_fact = layers.contains("fact");

    match (has_plan, has_oper, has_fact) {
        (true, true, true) => "full",
        (true, true, false) => "oper_plan",
        (true, false, true) => "fact_plan",
        (false, true, true) => "oper_fact",
        _ => "single",
    }
    .to_string()
}

async fn update_group_link_status(
    connection_mp_ref: &str,
    line_event_key: &str,
    turnover_code: &str,
) -> Result<()> {
    let mut group = Entity::find()
        .filter(Column::ConnectionMpRef.eq(connection_mp_ref))
        .filter(Column::LineEventKey.eq(line_event_key))
        .filter(Column::TurnoverCode.eq(turnover_code))
        .all(conn())
        .await?;

    if group.is_empty() {
        return Ok(());
    }

    let layers = group
        .iter()
        .map(|item| item.layer.clone())
        .collect::<BTreeSet<_>>();
    let link_status = derive_link_status(&layers);

    for mut entry in group.drain(..) {
        if entry.link_status != link_status {
            entry.link_status = link_status.clone();
            let active: ActiveModel = entry.into();
            active.update(conn()).await?;
        }
    }

    Ok(())
}

pub async fn refresh_group_link_status(
    connection_mp_ref: &str,
    line_event_key: &str,
    turnover_code: &str,
) -> Result<()> {
    update_group_link_status(connection_mp_ref, line_event_key, turnover_code).await
}

pub async fn get_by_id(id: &str) -> Result<Option<Model>> {
    Ok(Entity::find_by_id(id.to_string()).one(conn()).await?)
}

pub async fn save_entry(entry: &Model) -> Result<()> {
    let active = ActiveModel {
        id: Set(entry.id.clone()),
        connection_mp_ref: Set(entry.connection_mp_ref.clone()),
        order_key: Set(entry.order_key.clone()),
        line_key: Set(entry.line_key.clone()),
        line_event_key: Set(entry.line_event_key.clone()),
        event_kind: Set(entry.event_kind.clone()),
        entry_date: Set(entry.entry_date.clone()),
        layer: Set(entry.layer.clone()),
        turnover_code: Set(entry.turnover_code.clone()),
        value_kind: Set(entry.value_kind.clone()),
        agg_kind: Set(entry.agg_kind.clone()),
        amount: Set(entry.amount),
        nomenclature_ref: Set(entry.nomenclature_ref.clone()),
        marketplace_product_ref: Set(entry.marketplace_product_ref.clone()),
        registrator_type: Set(entry.registrator_type.clone()),
        registrator_ref: Set(entry.registrator_ref.clone()),
        link_status: Set(entry.link_status.clone()),
        general_ledger_ref: Set(entry.general_ledger_ref.clone()),
        created_at: Set(entry.created_at.clone()),
        updated_at: Set(entry.updated_at.clone()),
    };

    if Entity::find_by_id(entry.id.clone())
        .one(conn())
        .await?
        .is_some()
    {
        active.update(conn()).await?;
    } else {
        active.insert(conn()).await?;
    }

    update_group_link_status(
        &entry.connection_mp_ref,
        &entry.line_event_key,
        &entry.turnover_code,
    )
    .await?;

    Ok(())
}

pub async fn upsert_entry(entry: &Model) -> Result<()> {
    save_entry(entry).await
}

/// Прямой INSERT одной записи без SELECT-проверки и без обновления статуса группы.
/// Используется в batch-контексте перепроведения. После вставки всех строк
/// вызывающий код должен вызвать refresh_group_link_status для затронутых групп.
pub async fn insert_entry_raw(entry: &Model) -> Result<()> {
    let active = ActiveModel {
        id: Set(entry.id.clone()),
        connection_mp_ref: Set(entry.connection_mp_ref.clone()),
        order_key: Set(entry.order_key.clone()),
        line_key: Set(entry.line_key.clone()),
        line_event_key: Set(entry.line_event_key.clone()),
        event_kind: Set(entry.event_kind.clone()),
        entry_date: Set(entry.entry_date.clone()),
        layer: Set(entry.layer.clone()),
        turnover_code: Set(entry.turnover_code.clone()),
        value_kind: Set(entry.value_kind.clone()),
        agg_kind: Set(entry.agg_kind.clone()),
        amount: Set(entry.amount),
        nomenclature_ref: Set(entry.nomenclature_ref.clone()),
        marketplace_product_ref: Set(entry.marketplace_product_ref.clone()),
        registrator_type: Set(entry.registrator_type.clone()),
        registrator_ref: Set(entry.registrator_ref.clone()),
        link_status: Set(entry.link_status.clone()),
        general_ledger_ref: Set(entry.general_ledger_ref.clone()),
        created_at: Set(entry.created_at.clone()),
        updated_at: Set(entry.updated_at.clone()),
    };
    active.insert(conn()).await?;
    Ok(())
}

pub async fn delete_by_id(id: &str) -> Result<()> {
    if let Some(current) = get_by_id(id).await? {
        Entity::delete_by_id(id.to_string()).exec(conn()).await?;
        update_group_link_status(
            &current.connection_mp_ref,
            &current.line_event_key,
            &current.turnover_code,
        )
        .await?;
        return Ok(());
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn list_with_filters(
    date_from: Option<String>,
    date_to: Option<String>,
    connection_mp_ref: Option<String>,
    order_key: Option<String>,
    line_key: Option<String>,
    layer: Option<String>,
    turnover_code: Option<String>,
    link_status: Option<String>,
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
        &order_key,
        &line_key,
        &layer,
        &turnover_code,
        &link_status,
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
    order_key: Option<String>,
    line_key: Option<String>,
    layer: Option<String>,
    turnover_code: Option<String>,
    link_status: Option<String>,
) -> Result<u64> {
    let query = apply_filters(
        Entity::find(),
        &date_from,
        &date_to,
        &connection_mp_ref,
        &order_key,
        &line_key,
        &layer,
        &turnover_code,
        &link_status,
    );
    Ok(query.count(conn()).await?)
}

pub async fn list_by_connection_and_line_key(
    connection_mp_ref: &str,
    line_key: &str,
) -> Result<Vec<Model>> {
    Ok(Entity::find()
        .filter(Column::ConnectionMpRef.eq(connection_mp_ref))
        .filter(Column::LineKey.eq(line_key))
        .all(conn())
        .await?)
}

pub async fn list_by_registrator_ref(registrator_ref: &str) -> Result<Vec<Model>> {
    Ok(Entity::find()
        .filter(Column::RegistratorRef.eq(registrator_ref))
        .all(conn())
        .await?)
}

pub async fn list_link_groups_by_registrator_ref(
    registrator_ref: &str,
) -> Result<Vec<LinkGroupKey>> {
    let rows = list_by_registrator_ref(registrator_ref).await?;
    let groups: BTreeSet<LinkGroupKey> = rows
        .into_iter()
        .map(|row| LinkGroupKey {
            connection_mp_ref: row.connection_mp_ref,
            line_event_key: row.line_event_key,
            turnover_code: row.turnover_code,
        })
        .collect();
    Ok(groups.into_iter().collect())
}

pub async fn delete_many_by_registrator_ref(registrator_ref: &str) -> Result<u64> {
    let result = Entity::delete_many()
        .filter(Column::RegistratorRef.eq(registrator_ref))
        .exec(conn())
        .await?;
    Ok(result.rows_affected)
}

pub async fn list_by_registrator_ref_and_layer(
    registrator_ref: &str,
    layer: &str,
) -> Result<Vec<Model>> {
    Ok(Entity::find()
        .filter(Column::RegistratorRef.eq(registrator_ref))
        .filter(Column::Layer.eq(layer))
        .all(conn())
        .await?)
}

pub async fn delete_by_entry_date_range(date_from: &str, date_to: &str) -> Result<u64> {
    let result = Entity::delete_many()
        .filter(Column::EntryDate.gte(date_from))
        .filter(Column::EntryDate.lte(date_to))
        .exec(conn())
        .await?;
    Ok(result.rows_affected)
}

#[allow(clippy::too_many_arguments)]
fn apply_filters(
    mut query: Select<Entity>,
    date_from: &Option<String>,
    date_to: &Option<String>,
    connection_mp_ref: &Option<String>,
    order_key: &Option<String>,
    line_key: &Option<String>,
    layer: &Option<String>,
    turnover_code: &Option<String>,
    link_status: &Option<String>,
) -> Select<Entity> {
    if let Some(value) = date_from {
        query = query.filter(Column::EntryDate.gte(value.clone()));
    }
    if let Some(value) = date_to {
        query = query.filter(Column::EntryDate.lte(value.clone()));
    }
    if let Some(value) = connection_mp_ref {
        query = query.filter(Column::ConnectionMpRef.eq(value.clone()));
    }
    if let Some(value) = order_key {
        query = query.filter(Column::OrderKey.eq(value.clone()));
    }
    if let Some(value) = line_key {
        query = query.filter(Column::LineKey.eq(value.clone()));
    }
    if let Some(value) = layer {
        query = query.filter(Column::Layer.eq(value.clone()));
    }
    if let Some(value) = turnover_code {
        query = query.filter(Column::TurnoverCode.eq(value.clone()));
    }
    if let Some(value) = link_status {
        query = query.filter(Column::LinkStatus.eq(value.clone()));
    }
    query
}

fn apply_sort(mut query: Select<Entity>, sort_by: Option<&str>, sort_desc: bool) -> Select<Entity> {
    let column = match sort_by.unwrap_or("entry_date") {
        "entry_date" => Column::EntryDate,
        "layer" => Column::Layer,
        "turnover_code" => Column::TurnoverCode,
        "order_key" => Column::OrderKey,
        "line_key" => Column::LineKey,
        "link_status" => Column::LinkStatus,
        "event_kind" => Column::EventKind,
        "amount" => Column::Amount,
        _ => Column::EntryDate,
    };

    query = if sort_desc {
        query.order_by_desc(column)
    } else {
        query.order_by_asc(column)
    };

    if !matches!(column, Column::EntryDate) {
        query.order_by_desc(Column::EntryDate)
    } else {
        query.order_by_desc(Column::Id)
    }
}
