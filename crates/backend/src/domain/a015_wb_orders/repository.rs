use anyhow::Result;
use chrono::{NaiveDate, Utc};
use contracts::domain::a015_wb_orders::aggregate::{
    WbOrders, WbOrdersGeography, WbOrdersHeader, WbOrdersId, WbOrdersLine, WbOrdersSourceMeta,
    WbOrdersState, WbOrdersWarehouse,
};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a015_wb_orders")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub document_no: String,
    pub document_date: Option<String>,
    pub g_number: Option<String>,
    pub spp: Option<f64>,
    pub is_cancel: Option<bool>,
    pub cancel_date: Option<String>,
    pub header_json: String,
    pub line_json: String,
    pub state_json: String,
    pub warehouse_json: String,
    pub geography_json: String,
    pub source_meta_json: String,
    pub marketplace_product_ref: Option<String>,
    pub nomenclature_ref: Option<String>,
    pub is_deleted: bool,
    pub is_posted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for WbOrders {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());

        let header: WbOrdersHeader = serde_json::from_str(&m.header_json).unwrap_or_else(|_| {
            panic!(
                "Failed to deserialize header_json for document_no: {}",
                m.document_no
            )
        });
        let line: WbOrdersLine = serde_json::from_str(&m.line_json).unwrap_or_else(|_| {
            panic!(
                "Failed to deserialize line_json for document_no: {}",
                m.document_no
            )
        });
        let state: WbOrdersState = serde_json::from_str(&m.state_json).unwrap_or_else(|_| {
            panic!(
                "Failed to deserialize state_json for document_no: {}",
                m.document_no
            )
        });
        let warehouse: WbOrdersWarehouse =
            serde_json::from_str(&m.warehouse_json).unwrap_or_else(|_| {
                panic!(
                    "Failed to deserialize warehouse_json for document_no: {}",
                    m.document_no
                )
            });
        let geography: WbOrdersGeography =
            serde_json::from_str(&m.geography_json).unwrap_or_else(|_| {
                panic!(
                    "Failed to deserialize geography_json for document_no: {}",
                    m.document_no
                )
            });
        let source_meta: WbOrdersSourceMeta = serde_json::from_str(&m.source_meta_json)
            .unwrap_or_else(|_| {
                panic!(
                    "Failed to deserialize source_meta_json for document_no: {}",
                    m.document_no
                )
            });

        WbOrders {
            base: BaseAggregate::with_metadata(
                WbOrdersId(uuid),
                m.code,
                m.description,
                m.comment.clone(),
                metadata,
            ),
            header,
            line,
            state,
            warehouse,
            geography,
            source_meta,
            is_posted: m.is_posted,
            marketplace_product_ref: m.marketplace_product_ref,
            nomenclature_ref: m.nomenclature_ref,
            document_date: m.document_date,
        }
    }
}

pub async fn list_all() -> Result<Vec<WbOrders>> {
    let db = get_connection();
    let models = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .all(db)
        .await?;
    Ok(models.into_iter().map(|m| m.into()).collect())
}

pub async fn get_by_id(id: Uuid) -> Result<Option<WbOrders>> {
    let db = get_connection();
    let id_str = id.to_string();
    let model = Entity::find_by_id(id_str).one(db).await?;
    Ok(model.map(|m| m.into()))
}

pub async fn get_by_document_no(document_no: &str) -> Result<Option<WbOrders>> {
    let db = get_connection();
    let model = Entity::find()
        .filter(Column::DocumentNo.eq(document_no))
        .filter(Column::IsDeleted.eq(false))
        .one(db)
        .await?;
    Ok(model.map(|m| m.into()))
}

pub async fn search_by_document_no(document_no: &str) -> Result<Vec<WbOrders>> {
    let db = get_connection();
    let search_pattern = format!("%{}%", document_no);
    let models = Entity::find()
        .filter(Column::DocumentNo.like(&search_pattern))
        .filter(Column::IsDeleted.eq(false))
        .all(db)
        .await?;
    Ok(models.into_iter().map(|m| m.into()).collect())
}

pub async fn list_by_date_range(
    date_from: Option<NaiveDate>,
    date_to: Option<NaiveDate>,
) -> Result<Vec<WbOrders>> {
    let db = get_connection();
    let mut query = Entity::find().filter(Column::IsDeleted.eq(false));

    // Фильтрация по датам на уровне SQL через document_date
    if let Some(from) = date_from {
        let from_str = from.format("%Y-%m-%d").to_string();
        query = query.filter(Column::DocumentDate.gte(from_str));
    }

    if let Some(to) = date_to {
        let to_str = to.format("%Y-%m-%d").to_string();
        query = query.filter(Column::DocumentDate.lte(to_str));
    }

    let models = query.all(db).await?;
    Ok(models.into_iter().map(|m| m.into()).collect())
}

pub async fn upsert_document(document: &WbOrders) -> Result<Uuid> {
    let db = get_connection();
    let uuid = document.base.id.value();

    let header_json = serde_json::to_string(&document.header)?;
    let line_json = serde_json::to_string(&document.line)?;
    let state_json = serde_json::to_string(&document.state)?;
    let warehouse_json = serde_json::to_string(&document.warehouse)?;
    let geography_json = serde_json::to_string(&document.geography)?;
    let source_meta_json = serde_json::to_string(&document.source_meta)?;

    // Проверяем существование по document_no (не по ID!)
    let existing = get_by_document_no(&document.header.document_no).await?;

    // Извлекаем новые поля из агрегата
    let document_date = document.document_date.clone();
    let g_number = document.source_meta.g_number.clone();
    let spp = document.line.spp;
    let is_cancel = Some(document.state.is_cancel);
    let cancel_date = document.state.cancel_dt.map(|dt| dt.to_rfc3339());

    if let Some(existing_doc) = existing {
        // UPDATE - используем UUID существующего документа
        let existing_uuid = existing_doc.base.id.value();
        let active_model = ActiveModel {
            id: Set(existing_uuid.to_string()),
            code: Set(document.base.code.clone()),
            description: Set(document.base.description.clone()),
            comment: Set(document.base.comment.clone()),
            document_no: Set(document.header.document_no.clone()),
            document_date: Set(document_date),
            g_number: Set(g_number),
            spp: Set(spp),
            is_cancel: Set(is_cancel),
            cancel_date: Set(cancel_date),
            header_json: Set(header_json),
            line_json: Set(line_json),
            state_json: Set(state_json),
            warehouse_json: Set(warehouse_json),
            geography_json: Set(geography_json),
            source_meta_json: Set(source_meta_json),
            marketplace_product_ref: Set(document.marketplace_product_ref.clone()),
            nomenclature_ref: Set(document.nomenclature_ref.clone()),
            is_deleted: Set(document.base.metadata.is_deleted),
            is_posted: Set(document.is_posted),
            updated_at: Set(Some(Utc::now())),
            version: Set(document.base.metadata.version + 1),
            created_at: sea_orm::ActiveValue::NotSet,
        };

        Entity::update(active_model).exec(db).await?;
        Ok(existing_uuid)
    } else {
        // INSERT - используем новый UUID
        let active_model = ActiveModel {
            id: Set(uuid.to_string()),
            code: Set(document.base.code.clone()),
            description: Set(document.base.description.clone()),
            comment: Set(document.base.comment.clone()),
            document_no: Set(document.header.document_no.clone()),
            document_date: Set(document_date),
            g_number: Set(g_number),
            spp: Set(spp),
            is_cancel: Set(is_cancel),
            cancel_date: Set(cancel_date),
            header_json: Set(header_json),
            line_json: Set(line_json),
            state_json: Set(state_json),
            warehouse_json: Set(warehouse_json),
            geography_json: Set(geography_json),
            source_meta_json: Set(source_meta_json),
            marketplace_product_ref: Set(document.marketplace_product_ref.clone()),
            nomenclature_ref: Set(document.nomenclature_ref.clone()),
            is_deleted: Set(false),
            is_posted: Set(document.is_posted),
            created_at: Set(Some(Utc::now())),
            updated_at: Set(Some(Utc::now())),
            version: Set(1),
        };

        Entity::insert(active_model).exec(db).await?;
        Ok(uuid)
    }
}

pub async fn soft_delete(id: Uuid) -> Result<bool> {
    let db = get_connection();
    let id_str = id.to_string();

    let existing = Entity::find_by_id(&id_str).one(db).await?;

    if let Some(model) = existing {
        let mut active_model: ActiveModel = model.into();
        active_model.is_deleted = Set(true);
        active_model.updated_at = Set(Some(Utc::now()));
        Entity::update(active_model).exec(db).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub async fn set_posted(id: Uuid, is_posted: bool) -> Result<()> {
    let db = get_connection();
    let id_str = id.to_string();

    let existing = Entity::find_by_id(&id_str).one(db).await?;

    if let Some(model) = existing {
        let mut active_model: ActiveModel = model.into();
        active_model.is_posted = Set(is_posted);
        active_model.updated_at = Set(Some(Utc::now()));
        Entity::update(active_model).exec(db).await?;
    }

    Ok(())
}

