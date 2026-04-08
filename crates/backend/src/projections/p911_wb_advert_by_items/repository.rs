use anyhow::Result;
use sea_orm::entity::prelude::*;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Select, Set,
};
use serde::{Deserialize, Serialize};

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "p911_wb_advert_by_items")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub connection_mp_ref: String,
    pub entry_date: String,
    pub layer: String,
    pub turnover_code: String,
    pub value_kind: String,
    pub agg_kind: String,
    pub amount: f64,
    #[sea_orm(nullable)]
    pub nomenclature_ref: Option<String>,
    pub registrator_type: String,
    pub registrator_ref: String,
    #[sea_orm(nullable)]
    pub general_ledger_ref: Option<String>,
    pub is_problem: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

pub async fn get_by_id(id: &str) -> Result<Option<Model>> {
    Ok(Entity::find_by_id(id.to_string()).one(conn()).await?)
}

pub async fn save_entry(entry: &Model) -> Result<()> {
    save_entry_with_conn(conn(), entry).await
}

pub async fn save_entry_with_conn<C: ConnectionTrait>(db: &C, entry: &Model) -> Result<()> {
    let active = ActiveModel {
        id: Set(entry.id.clone()),
        connection_mp_ref: Set(entry.connection_mp_ref.clone()),
        entry_date: Set(entry.entry_date.clone()),
        layer: Set(entry.layer.clone()),
        turnover_code: Set(entry.turnover_code.clone()),
        value_kind: Set(entry.value_kind.clone()),
        agg_kind: Set(entry.agg_kind.clone()),
        amount: Set(entry.amount),
        nomenclature_ref: Set(entry.nomenclature_ref.clone()),
        registrator_type: Set(entry.registrator_type.clone()),
        registrator_ref: Set(entry.registrator_ref.clone()),
        general_ledger_ref: Set(entry.general_ledger_ref.clone()),
        is_problem: Set(entry.is_problem),
        created_at: Set(entry.created_at.clone()),
        updated_at: Set(entry.updated_at.clone()),
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

pub async fn upsert_entry(entry: &Model) -> Result<()> {
    save_entry(entry).await
}

pub async fn delete_by_registrator_ref(registrator_ref: &str) -> Result<u64> {
    delete_by_registrator_ref_with_conn(conn(), registrator_ref).await
}

pub async fn delete_by_registrator_ref_with_conn<C: ConnectionTrait>(
    db: &C,
    registrator_ref: &str,
) -> Result<u64> {
    let result = Entity::delete_many()
        .filter(Column::RegistratorRef.eq(registrator_ref))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

pub async fn list_by_registrator_ref(registrator_ref: &str) -> Result<Vec<Model>> {
    Ok(Entity::find()
        .filter(Column::RegistratorRef.eq(registrator_ref))
        .order_by_asc(Column::EntryDate)
        .order_by_asc(Column::Id)
        .all(conn())
        .await?)
}

pub async fn list_by_general_ledger_ref(general_ledger_ref: &str) -> Result<Vec<Model>> {
    Ok(Entity::find()
        .filter(Column::GeneralLedgerRef.eq(general_ledger_ref))
        .order_by_asc(Column::EntryDate)
        .order_by_asc(Column::Id)
        .all(conn())
        .await?)
}

#[allow(clippy::too_many_arguments)]
pub async fn list_with_filters(
    date_from: Option<String>,
    date_to: Option<String>,
    connection_mp_ref: Option<String>,
    nomenclature_ref: Option<String>,
    layer: Option<String>,
    turnover_code: Option<String>,
    registrator_ref: Option<String>,
    general_ledger_ref: Option<String>,
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
        &nomenclature_ref,
        &layer,
        &turnover_code,
        &registrator_ref,
        &general_ledger_ref,
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
    nomenclature_ref: Option<String>,
    layer: Option<String>,
    turnover_code: Option<String>,
    registrator_ref: Option<String>,
    general_ledger_ref: Option<String>,
) -> Result<u64> {
    let query = apply_filters(
        Entity::find(),
        &date_from,
        &date_to,
        &connection_mp_ref,
        &nomenclature_ref,
        &layer,
        &turnover_code,
        &registrator_ref,
        &general_ledger_ref,
    );
    Ok(query.count(conn()).await?)
}

#[allow(clippy::too_many_arguments)]
fn apply_filters(
    mut query: Select<Entity>,
    date_from: &Option<String>,
    date_to: &Option<String>,
    connection_mp_ref: &Option<String>,
    nomenclature_ref: &Option<String>,
    layer: &Option<String>,
    turnover_code: &Option<String>,
    registrator_ref: &Option<String>,
    general_ledger_ref: &Option<String>,
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
    if let Some(value) = nomenclature_ref {
        query = query.filter(Column::NomenclatureRef.eq(value.clone()));
    }
    if let Some(value) = layer {
        query = query.filter(Column::Layer.eq(value.clone()));
    }
    if let Some(value) = turnover_code {
        query = query.filter(Column::TurnoverCode.eq(value.clone()));
    }
    if let Some(value) = registrator_ref {
        query = query.filter(Column::RegistratorRef.eq(value.clone()));
    }
    if let Some(value) = general_ledger_ref {
        query = query.filter(Column::GeneralLedgerRef.eq(value.clone()));
    }
    query
}

fn apply_sort(mut query: Select<Entity>, sort_by: Option<&str>, sort_desc: bool) -> Select<Entity> {
    let column = match sort_by.unwrap_or("entry_date") {
        "entry_date" => Column::EntryDate,
        "layer" => Column::Layer,
        "turnover_code" => Column::TurnoverCode,
        "amount" => Column::Amount,
        "registrator_ref" => Column::RegistratorRef,
        "general_ledger_ref" => Column::GeneralLedgerRef,
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
