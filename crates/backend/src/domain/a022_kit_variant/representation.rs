//! Представление агрегата a022_kit_variant для сервиса представлений.

use std::collections::HashMap;

use contracts::general_ledger::AggregateRepresentation;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};

use super::repository::{Column, Entity};
use crate::shared::data::db::get_connection;
use crate::shared::representation::{build, chunked};

/// Название типа (зеркалит metadata element_name; generated-метаданные устарели).
const TYPE_NAME: &str = "Вариант комплектации";

/// Батч-резолв представлений: название типа + код варианта (даты/номера нет).
pub async fn represent_many(ids: &[String]) -> HashMap<String, AggregateRepresentation> {
    chunked(ids, |chunk| async move {
        let rows = Entity::find()
            .select_only()
            .column(Column::Id)
            .column(Column::Code)
            .filter(Column::Id.is_in(chunk))
            .into_tuple::<(String, String)>()
            .all(get_connection())
            .await
            .unwrap_or_default();
        rows.into_iter()
            .map(|(id, code)| (id, build(TYPE_NAME, None, Some(code))))
            .collect()
    })
    .await
}
