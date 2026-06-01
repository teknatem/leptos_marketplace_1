//! Представление агрегата a028_missing_cost_registry для сервиса представлений.

use std::collections::HashMap;

use contracts::general_ledger::AggregateRepresentation;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};

use super::repository::{Column, Entity};
use crate::shared::data::db::get_connection;
use crate::shared::representation::{build, chunked};

/// Название типа (метаданных у агрегата нет — задаём явно).
const TYPE_NAME: &str = "Реестр себестоимости";

/// Батч-резолв представлений: название типа + дата документа + номер документа.
pub async fn represent_many(ids: &[String]) -> HashMap<String, AggregateRepresentation> {
    chunked(ids, |chunk| async move {
        let rows = Entity::find()
            .select_only()
            .column(Column::Id)
            .column(Column::DocumentDate)
            .column(Column::DocumentNo)
            .filter(Column::Id.is_in(chunk))
            .into_tuple::<(String, String, String)>()
            .all(get_connection())
            .await
            .unwrap_or_default();
        rows.into_iter()
            .map(|(id, date, doc_no)| (id, build(TYPE_NAME, Some(date), Some(doc_no))))
            .collect()
    })
    .await
}
