//! Представление агрегата a014_ozon_transactions для сервиса представлений.

use std::collections::HashMap;

use contracts::general_ledger::AggregateRepresentation;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};

use super::repository::{Column, Entity};
use crate::shared::data::db::get_connection;
use crate::shared::representation::{build, chunked};

/// Название типа (зеркалит metadata element_name; generated-метаданные устарели).
const TYPE_NAME: &str = "Транзакция OZON";

/// Батч-резолв представлений: название типа + номер отправления (posting_number).
pub async fn represent_many(ids: &[String]) -> HashMap<String, AggregateRepresentation> {
    chunked(ids, |chunk| async move {
        let rows = Entity::find()
            .select_only()
            .column(Column::Id)
            .column(Column::PostingNumber)
            .filter(Column::Id.is_in(chunk))
            .into_tuple::<(String, String)>()
            .all(get_connection())
            .await
            .unwrap_or_default();
        rows.into_iter()
            .map(|(id, posting_number)| (id, build(TYPE_NAME, None, Some(posting_number))))
            .collect()
    })
    .await
}
