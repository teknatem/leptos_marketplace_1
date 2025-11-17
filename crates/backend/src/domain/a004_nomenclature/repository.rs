use chrono::Utc;
use contracts::domain::a004_nomenclature::aggregate::{Nomenclature, NomenclatureId};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a004_nomenclature")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub full_description: String,
    pub comment: Option<String>,
    pub is_folder: bool,
    pub parent_id: Option<String>,
    pub article: String,
    pub mp_ref_count: i32,
    // Измерения (классификация)
    pub dim1_category: String,
    pub dim2_line: String,
    pub dim3_model: String,
    pub dim4_format: String,
    pub dim5_sink: String,
    pub dim6_size: String,
    pub is_assembly: bool,
    pub base_nomenclature_ref: Option<String>,
    pub is_deleted: bool,
    pub is_posted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for Nomenclature {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());

        Nomenclature {
            base: BaseAggregate::with_metadata(
                NomenclatureId(uuid),
                m.code,
                m.description,
                m.comment.clone(),
                metadata,
            ),
            full_description: m.full_description,
            is_folder: m.is_folder,
            parent_id: m.parent_id,
            article: m.article,
            mp_ref_count: m.mp_ref_count,
            dim1_category: m.dim1_category,
            dim2_line: m.dim2_line,
            dim3_model: m.dim3_model,
            dim4_format: m.dim4_format,
            dim5_sink: m.dim5_sink,
            dim6_size: m.dim6_size,
            is_assembly: m.is_assembly,
            base_nomenclature_ref: m.base_nomenclature_ref,
        }
    }
}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

pub async fn list_all() -> anyhow::Result<Vec<Nomenclature>> {
    let mut items: Vec<Nomenclature> = Entity::find()
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    // Sort: folders first, then by description (case-insensitive)
    items.sort_by(|a, b| match (a.is_folder, b.is_folder) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a
            .base
            .description
            .to_lowercase()
            .cmp(&b.base.description.to_lowercase()),
    });
    Ok(items)
}

pub async fn get_by_id(id: Uuid) -> anyhow::Result<Option<Nomenclature>> {
    let result = Entity::find_by_id(id.to_string()).one(conn()).await?;
    Ok(result.map(Into::into))
}

pub async fn insert(aggregate: &Nomenclature) -> anyhow::Result<Uuid> {
    let uuid = aggregate.base.id.value();
    let active = ActiveModel {
        id: Set(uuid.to_string()),
        code: Set(aggregate.base.code.clone()),
        description: Set(aggregate.base.description.clone()),
        full_description: Set(aggregate.full_description.clone()),
        comment: Set(aggregate.base.comment.clone()),
        is_folder: Set(aggregate.is_folder),
        parent_id: Set(aggregate.parent_id.clone()),
        article: Set(aggregate.article.clone()),
        mp_ref_count: Set(aggregate.mp_ref_count),
        dim1_category: Set(aggregate.dim1_category.clone()),
        dim2_line: Set(aggregate.dim2_line.clone()),
        dim3_model: Set(aggregate.dim3_model.clone()),
        dim4_format: Set(aggregate.dim4_format.clone()),
        dim5_sink: Set(aggregate.dim5_sink.clone()),
        dim6_size: Set(aggregate.dim6_size.clone()),
        is_assembly: Set(aggregate.is_assembly),
        base_nomenclature_ref: Set(aggregate.base_nomenclature_ref.clone()),
        is_deleted: Set(aggregate.base.metadata.is_deleted),
        is_posted: Set(aggregate.base.metadata.is_posted),
        created_at: Set(Some(aggregate.base.metadata.created_at)),
        updated_at: Set(Some(aggregate.base.metadata.updated_at)),
        version: Set(aggregate.base.metadata.version),
    };
    active.insert(conn()).await?;
    Ok(uuid)
}

pub async fn update(aggregate: &Nomenclature) -> anyhow::Result<()> {
    let id = aggregate.base.id.value().to_string();
    let active = ActiveModel {
        id: Set(id),
        code: Set(aggregate.base.code.clone()),
        description: Set(aggregate.base.description.clone()),
        full_description: Set(aggregate.full_description.clone()),
        comment: Set(aggregate.base.comment.clone()),
        is_folder: Set(aggregate.is_folder),
        parent_id: Set(aggregate.parent_id.clone()),
        article: Set(aggregate.article.clone()),
        mp_ref_count: Set(aggregate.mp_ref_count),
        dim1_category: Set(aggregate.dim1_category.clone()),
        dim2_line: Set(aggregate.dim2_line.clone()),
        dim3_model: Set(aggregate.dim3_model.clone()),
        dim4_format: Set(aggregate.dim4_format.clone()),
        dim5_sink: Set(aggregate.dim5_sink.clone()),
        dim6_size: Set(aggregate.dim6_size.clone()),
        is_assembly: Set(aggregate.is_assembly),
        base_nomenclature_ref: Set(aggregate.base_nomenclature_ref.clone()),
        is_deleted: Set(aggregate.base.metadata.is_deleted),
        is_posted: Set(aggregate.base.metadata.is_posted),
        updated_at: Set(Some(aggregate.base.metadata.updated_at)),
        version: Set(aggregate.base.metadata.version),
        created_at: sea_orm::ActiveValue::NotSet,
    };
    active.update(conn()).await?;
    Ok(())
}

pub async fn soft_delete(id: Uuid) -> anyhow::Result<bool> {
    use sea_orm::sea_query::Expr;
    let result = Entity::update_many()
        .col_expr(Column::IsDeleted, Expr::value(true))
        .col_expr(Column::UpdatedAt, Expr::value(Utc::now()))
        .filter(Column::Id.eq(id.to_string()))
        .exec(conn())
        .await?;
    Ok(result.rows_affected > 0)
}

/// Найти номенклатуру по артикулу
/// Возвращает только элементы (не папки) и не удаленные
/// ВАЖНО: article должен быть уже trimmed
pub async fn find_by_article(article: &str) -> anyhow::Result<Vec<Nomenclature>> {
    // Загружаем все элементы и фильтруем на стороне приложения для корректного trim
    let all_items: Vec<Model> = Entity::find()
        .filter(Column::IsFolder.eq(false))
        .filter(Column::IsDeleted.eq(false))
        .all(conn())
        .await?;

    let items: Vec<Nomenclature> = all_items
        .into_iter()
        .filter(|m| m.article.trim() == article)
        .map(Into::into)
        .collect();

    Ok(items)
}

/// Найти номенклатуру по артикулу (без учета регистра)
/// Возвращает только элементы (не папки) и не удаленные
/// ВАЖНО: article должен быть уже trimmed
pub async fn find_by_article_ignore_case(article: &str) -> anyhow::Result<Vec<Nomenclature>> {
    let article_lower = article.to_lowercase();

    // Загружаем все элементы и фильтруем на стороне приложения для корректного trim и lowercase
    let all_items: Vec<Model> = Entity::find()
        .filter(Column::IsFolder.eq(false))
        .filter(Column::IsDeleted.eq(false))
        .all(conn())
        .await?;

    let items: Vec<Nomenclature> = all_items
        .into_iter()
        .filter(|m| m.article.trim().to_lowercase() == article_lower)
        .map(Into::into)
        .collect();

    Ok(items)
}

/// Обновить счетчик ссылок на маркетплейс для номенклатуры
pub async fn update_mp_ref_count(nomenclature_id: Uuid, count: i32) -> anyhow::Result<()> {
    use sea_orm::sea_query::Expr;
    Entity::update_many()
        .col_expr(Column::MpRefCount, Expr::value(count))
        .col_expr(Column::UpdatedAt, Expr::value(Utc::now()))
        .filter(Column::Id.eq(nomenclature_id.to_string()))
        .exec(conn())
        .await?;
    Ok(())
}

/// Структура для возврата списка уникальных значений измерений
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionValues {
    pub dim1_category: Vec<String>,
    pub dim2_line: Vec<String>,
    pub dim3_model: Vec<String>,
    pub dim4_format: Vec<String>,
    pub dim5_sink: Vec<String>,
    pub dim6_size: Vec<String>,
}

/// Получить все уникальные значения измерений
pub async fn get_distinct_dimension_values() -> anyhow::Result<DimensionValues> {
    use std::collections::BTreeSet;

    // Получаем все записи
    let all_items: Vec<Model> = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .all(conn())
        .await?;

    // Используем BTreeSet для автоматической сортировки и уникальности
    let mut dim1_set = BTreeSet::new();
    let mut dim2_set = BTreeSet::new();
    let mut dim3_set = BTreeSet::new();
    let mut dim4_set = BTreeSet::new();
    let mut dim5_set = BTreeSet::new();
    let mut dim6_set = BTreeSet::new();

    for item in all_items {
        // Добавляем только непустые значения (после trim)
        let dim1 = item.dim1_category.trim();
        if !dim1.is_empty() {
            dim1_set.insert(dim1.to_string());
        }

        let dim2 = item.dim2_line.trim();
        if !dim2.is_empty() {
            dim2_set.insert(dim2.to_string());
        }

        let dim3 = item.dim3_model.trim();
        if !dim3.is_empty() {
            dim3_set.insert(dim3.to_string());
        }

        let dim4 = item.dim4_format.trim();
        if !dim4.is_empty() {
            dim4_set.insert(dim4.to_string());
        }

        let dim5 = item.dim5_sink.trim();
        if !dim5.is_empty() {
            dim5_set.insert(dim5.to_string());
        }

        let dim6 = item.dim6_size.trim();
        if !dim6.is_empty() {
            dim6_set.insert(dim6.to_string());
        }
    }

    Ok(DimensionValues {
        dim1_category: dim1_set.into_iter().collect(),
        dim2_line: dim2_set.into_iter().collect(),
        dim3_model: dim3_set.into_iter().collect(),
        dim4_format: dim4_set.into_iter().collect(),
        dim5_sink: dim5_set.into_iter().collect(),
        dim6_size: dim6_set.into_iter().collect(),
    })
}

/// Удалить записи по списку ID (жесткое удаление)
pub async fn delete_by_ids(ids: Vec<Uuid>) -> anyhow::Result<u64> {
    if ids.is_empty() {
        return Ok(0);
    }

    let id_strings: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
    
    let result = Entity::delete_many()
        .filter(Column::Id.is_in(id_strings))
        .exec(conn())
        .await?;
    
    Ok(result.rows_affected)
}