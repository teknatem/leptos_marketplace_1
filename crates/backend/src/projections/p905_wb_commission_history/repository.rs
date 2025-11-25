use anyhow::Result;
use chrono::Utc;
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set};
use serde::{Deserialize, Serialize};

use crate::shared::data::db::get_connection;

/// Модель Wildberries Commission History entry
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "p905_wb_commission_history")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    pub date: String,
    pub subject_id: i32,
    pub subject_name: String,
    pub parent_id: i32,
    pub parent_name: String,
    pub kgvp_booking: f64,
    pub kgvp_marketplace: f64,
    pub kgvp_pickup: f64,
    pub kgvp_supplier: f64,
    pub kgvp_supplier_express: f64,
    pub paid_storage_kgvp: f64,
    pub raw_json: String,
    pub loaded_at_utc: String,
    pub payload_version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// Структура для передачи данных в upsert
#[derive(Debug, Clone)]
pub struct CommissionEntry {
    pub id: String,
    pub date: chrono::NaiveDate,
    pub subject_id: i32,
    pub subject_name: String,
    pub parent_id: i32,
    pub parent_name: String,
    pub kgvp_booking: f64,
    pub kgvp_marketplace: f64,
    pub kgvp_pickup: f64,
    pub kgvp_supplier: f64,
    pub kgvp_supplier_express: f64,
    pub paid_storage_kgvp: f64,
    pub raw_json: String,
    pub payload_version: i32,
}

/// Upsert записи в p905_wb_commission_history
pub async fn upsert_entry(entry: &CommissionEntry) -> Result<()> {
    let db = get_connection();
    let date_str = entry.date.format("%Y-%m-%d").to_string();
    let loaded_at_utc = Utc::now().to_rfc3339();

    // Проверяем, существует ли запись с таким date и subject_id
    let existing = Entity::find()
        .filter(Column::Date.eq(&date_str))
        .filter(Column::SubjectId.eq(entry.subject_id))
        .one(db)
        .await?;

    if let Some(existing_model) = existing {
        // Обновить существующую запись
        let mut active_model: ActiveModel = existing_model.into();

        active_model.subject_name = Set(entry.subject_name.clone());
        active_model.parent_id = Set(entry.parent_id);
        active_model.parent_name = Set(entry.parent_name.clone());
        active_model.kgvp_booking = Set(entry.kgvp_booking);
        active_model.kgvp_marketplace = Set(entry.kgvp_marketplace);
        active_model.kgvp_pickup = Set(entry.kgvp_pickup);
        active_model.kgvp_supplier = Set(entry.kgvp_supplier);
        active_model.kgvp_supplier_express = Set(entry.kgvp_supplier_express);
        active_model.paid_storage_kgvp = Set(entry.paid_storage_kgvp);
        active_model.raw_json = Set(entry.raw_json.clone());
        active_model.loaded_at_utc = Set(loaded_at_utc);
        active_model.payload_version = Set(entry.payload_version);

        active_model.update(db).await?;
    } else {
        // Вставить новую запись
        let new_model = ActiveModel {
            id: Set(entry.id.clone()),
            date: Set(date_str),
            subject_id: Set(entry.subject_id),
            subject_name: Set(entry.subject_name.clone()),
            parent_id: Set(entry.parent_id),
            parent_name: Set(entry.parent_name.clone()),
            kgvp_booking: Set(entry.kgvp_booking),
            kgvp_marketplace: Set(entry.kgvp_marketplace),
            kgvp_pickup: Set(entry.kgvp_pickup),
            kgvp_supplier: Set(entry.kgvp_supplier),
            kgvp_supplier_express: Set(entry.kgvp_supplier_express),
            paid_storage_kgvp: Set(entry.paid_storage_kgvp),
            raw_json: Set(entry.raw_json.clone()),
            loaded_at_utc: Set(loaded_at_utc),
            payload_version: Set(entry.payload_version),
        };

        new_model.insert(db).await?;
    }

    Ok(())
}

/// Получить список записей с фильтрами
pub async fn list_with_filters(
    date_from: Option<String>,
    date_to: Option<String>,
    subject_id: Option<i32>,
    sort_by: &str,
    sort_desc: bool,
    limit: u64,
    offset: u64,
) -> Result<(Vec<Model>, u64)> {
    let db = get_connection();

    // Построить запрос с фильтрами
    let mut query = Entity::find();

    // Фильтр по дате
    if let Some(from) = date_from {
        query = query.filter(Column::Date.gte(from));
    }
    if let Some(to) = date_to {
        query = query.filter(Column::Date.lte(to));
    }

    // Фильтр по subject_id
    if let Some(subj_id) = subject_id {
        query = query.filter(Column::SubjectId.eq(subj_id));
    }

    // Подсчет общего количества записей (до пагинации)
    let total_count = query.clone().count(db).await?;

    // Сортировка
    query = match sort_by {
        "date" => {
            if sort_desc {
                query.order_by_desc(Column::Date)
            } else {
                query.order_by_asc(Column::Date)
            }
        }
        "subject_id" => {
            if sort_desc {
                query.order_by_desc(Column::SubjectId)
            } else {
                query.order_by_asc(Column::SubjectId)
            }
        }
        "subject_name" => {
            if sort_desc {
                query.order_by_desc(Column::SubjectName)
            } else {
                query.order_by_asc(Column::SubjectName)
            }
        }
        "parent_name" => {
            if sort_desc {
                query.order_by_desc(Column::ParentName)
            } else {
                query.order_by_asc(Column::ParentName)
            }
        }
        _ => {
            // По умолчанию сортировка по дате (новые первые)
            query.order_by_desc(Column::Date)
        }
    };

    // Пагинация
    let items = query.limit(limit).offset(offset).all(db).await?;

    Ok((items, total_count))
}

/// Получить последнюю запись для конкретной категории (subject_id)
pub async fn get_latest_by_subject(subject_id: i32) -> Result<Option<Model>> {
    let db = get_connection();

    let item = Entity::find()
        .filter(Column::SubjectId.eq(subject_id))
        .order_by_desc(Column::Date)
        .one(db)
        .await?;

    Ok(item)
}

/// Получить запись по ID
pub async fn get_by_id(id: &str) -> Result<Option<Model>> {
    let db = get_connection();

    let item = Entity::find().filter(Column::Id.eq(id)).one(db).await?;

    Ok(item)
}

/// Удалить запись по ID
pub async fn delete_by_id(id: &str) -> Result<u64> {
    let db = get_connection();

    let result = Entity::delete_many()
        .filter(Column::Id.eq(id))
        .exec(db)
        .await?;

    Ok(result.rows_affected)
}

/// Получить все уникальные subject_id из таблицы
pub async fn get_all_subject_ids() -> Result<Vec<i32>> {
    let db = get_connection();

    let items = Entity::find()
        .select_only()
        .column(Column::SubjectId)
        .distinct()
        .all(db)
        .await?;

    Ok(items.iter().map(|item| item.subject_id).collect())
}
