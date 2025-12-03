use anyhow::Result;
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set, FromQueryResult};
use serde::{Deserialize, Serialize};

use crate::shared::data::db::get_connection;

/// Модель записи штрихкода номенклатуры
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "p901_nomenclature_barcodes")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub barcode: String,

    #[sea_orm(primary_key, auto_increment = false)]
    pub source: String,  // "1C" | "OZON" | "WB" | "YM"

    #[sea_orm(nullable)]
    pub nomenclature_ref: Option<String>,  // UUID на a004_nomenclature (nullable)

    #[sea_orm(nullable)]
    pub article: Option<String>,

    pub created_at: String,  // DateTime<Utc> as ISO8601
    pub updated_at: String,  // DateTime<Utc> as ISO8601

    pub is_active: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// Структура для результата с JOIN (barcode + nomenclature name)
#[derive(Debug, Clone, FromQueryResult)]
pub struct BarcodeWithNomenclature {
    pub barcode: String,
    pub source: String,
    pub nomenclature_ref: Option<String>,
    pub article: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub is_active: bool,
    pub nomenclature_name: Option<String>,  // description из a004_nomenclature
}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

/// Структура для передачи данных в upsert
#[derive(Debug, Clone)]
pub struct NomenclatureBarcodeEntry {
    pub barcode: String,
    pub source: String,
    pub nomenclature_ref: Option<String>,
    pub article: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
}

impl NomenclatureBarcodeEntry {
    /// Конвертация в ActiveModel для записи в БД
    pub fn to_active_model(&self) -> ActiveModel {
        ActiveModel {
            barcode: Set(self.barcode.clone()),
            nomenclature_ref: Set(self.nomenclature_ref.clone()),
            article: Set(self.article.clone()),
            source: Set(self.source.clone()),
            created_at: Set(self.created_at.to_rfc3339()),
            updated_at: Set(self.updated_at.to_rfc3339()),
            is_active: Set(self.is_active),
        }
    }
}

/// Upsert записи штрихкода (insert or update)
pub async fn upsert_entry(entry: &NomenclatureBarcodeEntry) -> Result<()> {
    let db = conn();

    // Проверяем существование записи по композитному ключу (barcode + source)
    let existing = Entity::find()
        .filter(Column::Barcode.eq(&entry.barcode))
        .filter(Column::Source.eq(&entry.source))
        .one(db)
        .await?;

    let active_model = entry.to_active_model();

    if existing.is_some() {
        // Update существующей записи
        Entity::update(active_model)
            .filter(Column::Barcode.eq(&entry.barcode))
            .filter(Column::Source.eq(&entry.source))
            .exec(db)
            .await?;
    } else {
        // Insert новой записи
        Entity::insert(active_model)
            .exec(db)
            .await?;
    }

    Ok(())
}

/// Получить запись по штрихкоду и источнику (composite key)
pub async fn get_by_barcode_and_source(barcode: &str, source: &str) -> Result<Option<Model>> {
    let db = conn();

    let result = Entity::find()
        .filter(Column::Barcode.eq(barcode))
        .filter(Column::Source.eq(source))
        .filter(Column::IsActive.eq(true))
        .one(db)
        .await?;

    Ok(result)
}

/// Получить запись по артикулу и источнику
/// Используется для поиска штрихкода YM по shop_sku
pub async fn get_by_article_and_source(article: &str, source: &str) -> Result<Option<Model>> {
    let db = conn();

    let result = Entity::find()
        .filter(Column::Article.eq(article))
        .filter(Column::Source.eq(source))
        .filter(Column::IsActive.eq(true))
        .one(db)
        .await?;

    Ok(result)
}

/// Получить все записи для штрихкода (всех источников)
pub async fn get_all_by_barcode(barcode: &str) -> Result<Vec<Model>> {
    let db = conn();

    let results = Entity::find()
        .filter(Column::Barcode.eq(barcode))
        .filter(Column::IsActive.eq(true))
        .order_by_asc(Column::Source)
        .all(db)
        .await?;

    Ok(results)
}

/// Получить все штрихкоды по nomenclature_ref
pub async fn get_by_nomenclature_ref(
    nomenclature_ref: &str,
    include_inactive: bool,
) -> Result<Vec<Model>> {
    let db = conn();

    let mut query = Entity::find()
        .filter(Column::NomenclatureRef.eq(nomenclature_ref));

    if !include_inactive {
        query = query.filter(Column::IsActive.eq(true));
    }

    let results = query
        .order_by_asc(Column::Barcode)
        .all(db)
        .await?;

    Ok(results)
}

/// Получить список штрихкодов с фильтрами и пагинацией (с JOIN для nomenclature_name)
pub async fn list_with_filters(
    nomenclature_ref: Option<String>,
    article: Option<String>,
    source: Option<String>,
    include_inactive: bool,
    limit: i32,
    offset: i32,
) -> Result<(Vec<BarcodeWithNomenclature>, i32)> {
    use sea_orm::{Statement, ConnectionTrait};
    let db = conn();

    // Строим WHERE условия
    let mut where_clauses = vec![];
    let mut params: Vec<sea_orm::Value> = vec![];

    if let Some(ref nomenclature_ref_val) = nomenclature_ref {
        where_clauses.push("b.nomenclature_ref = ?");
        params.push(nomenclature_ref_val.clone().into());
    }

    if let Some(ref article_val) = article {
        if !article_val.is_empty() {
            where_clauses.push("b.article LIKE ?");
            params.push(format!("%{}%", article_val).into());
        }
    }

    if let Some(ref source_val) = source {
        where_clauses.push("b.source = ?");
        params.push(source_val.clone().into());
    }

    if !include_inactive {
        where_clauses.push("b.is_active = ?");
        params.push(true.into());
    }

    let where_sql = if where_clauses.is_empty() {
        String::from("1=1")
    } else {
        where_clauses.join(" AND ")
    };

    // Запрос для подсчета total_count
    let count_sql = format!(
        "SELECT COUNT(*) as count FROM p901_nomenclature_barcodes b WHERE {}",
        where_sql
    );

    let count_stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        &count_sql,
        params.iter().cloned(),
    );

    #[derive(Debug, FromQueryResult)]
    struct CountResult {
        count: i32,
    }

    let count_result = CountResult::find_by_statement(count_stmt)
        .one(db)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Failed to get count"))?;

    let total_count = count_result.count;

    // Основной запрос с JOIN
    let sql = format!(
        "SELECT
            b.barcode,
            b.nomenclature_ref,
            b.article,
            b.source,
            b.created_at,
            b.updated_at,
            b.is_active,
            n.description as nomenclature_name
        FROM p901_nomenclature_barcodes b
        LEFT JOIN a004_nomenclature n ON b.nomenclature_ref = n.id
        WHERE {}
        ORDER BY b.updated_at DESC
        LIMIT ? OFFSET ?",
        where_sql
    );

    // Добавляем limit и offset к параметрам
    params.push((limit as i64).into());
    params.push((offset as i64).into());

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        &sql,
        params,
    );

    let results: Vec<BarcodeWithNomenclature> = BarcodeWithNomenclature::find_by_statement(stmt)
        .all(db)
        .await?;

    Ok((results, total_count))
}

/// Деактивировать штрихкод по композитному ключу (мягкое удаление)
pub async fn deactivate_by_barcode_and_source(barcode: &str, source: &str) -> Result<bool> {
    let db = conn();

    let existing = Entity::find()
        .filter(Column::Barcode.eq(barcode))
        .filter(Column::Source.eq(source))
        .one(db)
        .await?;

    if let Some(model) = existing {
        let mut active_model: ActiveModel = model.into();
        active_model.is_active = Set(false);
        active_model.updated_at = Set(Utc::now().to_rfc3339());

        Entity::update(active_model)
            .exec(db)
            .await?;

        Ok(true)
    } else {
        Ok(false)
    }
}

/// Получить все записи (для отладки)
pub async fn list_all(limit: Option<u64>) -> Result<Vec<Model>> {
    let db = conn();

    let mut query = Entity::find()
        .order_by_desc(Column::UpdatedAt);

    if let Some(limit_val) = limit {
        query = query.limit(limit_val);
    }

    let results = query.all(db).await?;

    Ok(results)
}
