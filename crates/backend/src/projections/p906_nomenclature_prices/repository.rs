use anyhow::Result;
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use sea_orm::{
    ColumnTrait, EntityTrait, FromQueryResult, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};

use crate::shared::data::db::get_connection;

/// Модель записи цены номенклатуры
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "p906_nomenclature_prices")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// Период (дата) из 1С
    pub period: String,

    /// UUID номенклатуры из 1С
    pub nomenclature_ref: String,

    /// Цена
    pub price: f64,

    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// Структура с JOIN для отображения имени номенклатуры
#[derive(Debug, Clone, FromQueryResult, Serialize, Deserialize)]
pub struct PriceWithNomenclature {
    pub id: String,
    pub period: String,
    pub nomenclature_ref: String,
    pub price: f64,
    pub created_at: String,
    pub updated_at: String,
    pub nomenclature_name: Option<String>,
    pub nomenclature_article: Option<String>,
}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

/// Структура для передачи данных при создании/обновлении
#[derive(Debug, Clone)]
pub struct NomenclaturePriceEntry {
    pub id: String,
    pub period: String,
    pub nomenclature_ref: String,
    pub price: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl NomenclaturePriceEntry {
    /// Конвертация в ActiveModel для записи в БД
    pub fn to_active_model(&self) -> ActiveModel {
        ActiveModel {
            id: Set(self.id.clone()),
            period: Set(self.period.clone()),
            nomenclature_ref: Set(self.nomenclature_ref.clone()),
            price: Set(self.price),
            created_at: Set(self.created_at.to_rfc3339()),
            updated_at: Set(self.updated_at.to_rfc3339()),
        }
    }
}

/// Вставить новую запись
pub async fn insert_entry(entry: &NomenclaturePriceEntry) -> Result<()> {
    let db = conn();
    let active_model = entry.to_active_model();
    Entity::insert(active_model).exec(db).await?;
    Ok(())
}

/// Upsert записи (insert or update)
pub async fn upsert_entry(entry: &NomenclaturePriceEntry) -> Result<()> {
    let db = conn();

    // Проверяем существование записи по period + nomenclature_ref
    let existing = Entity::find()
        .filter(Column::Period.eq(&entry.period))
        .filter(Column::NomenclatureRef.eq(&entry.nomenclature_ref))
        .one(db)
        .await?;

    if let Some(existing_model) = existing {
        // Update существующей записи
        let mut active_model: ActiveModel = existing_model.into();
        active_model.price = Set(entry.price);
        active_model.updated_at = Set(entry.updated_at.to_rfc3339());
        Entity::update(active_model).exec(db).await?;
    } else {
        // Insert новой записи
        let active_model = entry.to_active_model();
        Entity::insert(active_model).exec(db).await?;
    }

    Ok(())
}

/// Получить запись по ID
pub async fn get_by_id(id: &str) -> Result<Option<Model>> {
    let db = conn();
    let result = Entity::find_by_id(id).one(db).await?;
    Ok(result)
}

/// Получить записи по периоду
pub async fn get_by_period(period: &str) -> Result<Vec<Model>> {
    let db = conn();
    let results = Entity::find()
        .filter(Column::Period.eq(period))
        .order_by_asc(Column::NomenclatureRef)
        .all(db)
        .await?;
    Ok(results)
}

/// Получить записи по номенклатуре
pub async fn get_by_nomenclature_ref(nomenclature_ref: &str) -> Result<Vec<Model>> {
    let db = conn();
    let results = Entity::find()
        .filter(Column::NomenclatureRef.eq(nomenclature_ref))
        .order_by_desc(Column::Period)
        .all(db)
        .await?;
    Ok(results)
}

/// Удалить все записи по периоду (перед загрузкой нового периода)
pub async fn delete_by_period(period: &str) -> Result<u64> {
    let db = conn();
    let result = Entity::delete_many()
        .filter(Column::Period.eq(period))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

/// Удалить все записи по диапазону периодов (перед загрузкой)
pub async fn delete_by_period_range(period_from: &str, period_to: &str) -> Result<u64> {
    let db = conn();
    let result = Entity::delete_many()
        .filter(Column::Period.gte(period_from))
        .filter(Column::Period.lte(period_to))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

/// Удалить все записи (очистка таблицы)
pub async fn delete_all() -> Result<u64> {
    let db = conn();
    let result = Entity::delete_many().exec(db).await?;
    Ok(result.rows_affected)
}

/// Получить список цен с фильтрами и пагинацией (с JOIN для nomenclature_name)
pub async fn list_with_filters(
    period: Option<String>,
    nomenclature_ref: Option<String>,
    q: Option<String>,
    sort_by: Option<String>,
    sort_desc: Option<bool>,
    limit: Option<u64>,
    offset: Option<u64>,
) -> Result<(Vec<PriceWithNomenclature>, i64)> {
    use sea_orm::{ConnectionTrait, Statement};
    let db = conn();

    fn escape_like(s: &str) -> String {
        // Escape LIKE wildcards for SQLite: %, _ and the escape char itself.
        // We use ESCAPE '\\' in SQL.
        let mut out = String::with_capacity(s.len());
        for ch in s.chars() {
            match ch {
                '\\' => out.push_str("\\\\"),
                '%' => out.push_str("\\%"),
                '_' => out.push_str("\\_"),
                _ => out.push(ch),
            }
        }
        out
    }

    // Строим WHERE условия
    let mut where_clauses = vec![];
    let mut params: Vec<sea_orm::Value> = vec![];

    if let Some(ref period_val) = period {
        where_clauses.push("p.period = ?");
        params.push(period_val.clone().into());
    }

    if let Some(ref nomenclature_ref_val) = nomenclature_ref {
        where_clauses.push("p.nomenclature_ref = ?");
        params.push(nomenclature_ref_val.clone().into());
    }

    if let Some(ref q_val) = q {
        let q_trimmed = q_val.trim();
        if q_trimmed.len() >= 3 {
            let escaped = escape_like(&q_trimmed.to_lowercase());
            let like = format!("%{}%", escaped);
            // NOTE: ESCAPE must be a *single* character in SQLite.
            // We use backslash as escape char: ESCAPE '\'
            where_clauses.push(
                "(lower(n.article) LIKE ? ESCAPE '\\' OR lower(n.description) LIKE ? ESCAPE '\\')",
            );
            params.push(like.clone().into());
            params.push(like.into());
        }
    }

    let where_sql = if where_clauses.is_empty() {
        String::from("1=1")
    } else {
        where_clauses.join(" AND ")
    };

    // Запрос для подсчета total_count
    let count_sql = format!(
        "SELECT COUNT(*) as count
        FROM p906_nomenclature_prices p
        LEFT JOIN a004_nomenclature n ON p.nomenclature_ref = n.id
        WHERE {}",
        where_sql
    );

    let count_stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        &count_sql,
        params.iter().cloned(),
    );

    #[derive(Debug, FromQueryResult)]
    struct CountResult {
        count: i64,
    }

    let count_result = CountResult::find_by_statement(count_stmt)
        .one(db)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Failed to get count"))?;

    let total_count = count_result.count;

    // ORDER BY whitelist (avoid SQL injection)
    let order_col = match sort_by.as_deref() {
        Some("period") => "p.period",
        Some("article") => "n.article",
        Some("code1c") => "p.nomenclature_ref",
        Some("nomenclature_name") => "n.description",
        Some("price") => "p.price",
        _ => "p.period",
    };
    let order_dir = if sort_desc.unwrap_or(true) {
        "DESC"
    } else {
        "ASC"
    };
    let order_sql = format!(
        "{} {},
         p.period DESC,
         n.description ASC,
         p.id ASC",
        order_col, order_dir
    );

    // Основной запрос с JOIN
    let mut sql = format!(
        "SELECT
            p.id,
            p.period,
            p.nomenclature_ref,
            p.price,
            p.created_at,
            p.updated_at,
            n.description as nomenclature_name,
            n.article as nomenclature_article
        FROM p906_nomenclature_prices p
        LEFT JOIN a004_nomenclature n ON p.nomenclature_ref = n.id
        WHERE {}
        ORDER BY {}",
        where_sql, order_sql
    );

    // Добавляем LIMIT и OFFSET
    if let Some(lim) = limit {
        sql.push_str(&format!(" LIMIT {}", lim));
    }
    if let Some(off) = offset {
        sql.push_str(&format!(" OFFSET {}", off));
    }

    let stmt = Statement::from_sql_and_values(db.get_database_backend(), &sql, params);

    let results: Vec<PriceWithNomenclature> = PriceWithNomenclature::find_by_statement(stmt)
        .all(db)
        .await?;

    Ok((results, total_count))
}

/// Получить все записи (для отладки)
pub async fn list_all(limit: Option<u64>) -> Result<Vec<Model>> {
    let db = conn();

    let mut query = Entity::find().order_by_desc(Column::Period);

    if let Some(limit_val) = limit {
        query = query.limit(limit_val);
    }

    let results = query.all(db).await?;
    Ok(results)
}

/// Получить уникальные периоды (для фильтра в UI)
pub async fn get_unique_periods() -> Result<Vec<String>> {
    use sea_orm::{ConnectionTrait, Statement};
    let db = conn();

    let sql = "SELECT DISTINCT period FROM p906_nomenclature_prices ORDER BY period DESC";
    let stmt = Statement::from_string(db.get_database_backend(), sql.to_string());

    #[derive(Debug, FromQueryResult)]
    struct PeriodResult {
        period: String,
    }

    let results: Vec<PeriodResult> = PeriodResult::find_by_statement(stmt).all(db).await?;

    Ok(results.into_iter().map(|r| r.period).collect())
}

/// Получить актуальную цену для номенклатуры на указанную дату
/// Логика: найти запись с MAX(period) WHERE period <= sale_date
pub async fn get_price_for_date(nomenclature_ref: &str, sale_date: &str) -> Result<Option<f64>> {
    let db = conn();

    // Находим цену с максимальной датой period, которая <= sale_date
    let result = Entity::find()
        .filter(Column::NomenclatureRef.eq(nomenclature_ref))
        .filter(Column::Period.lte(sale_date))
        .order_by_desc(Column::Period)
        .one(db)
        .await?;

    Ok(result.map(|m| m.price))
}
