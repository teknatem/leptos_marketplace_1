use anyhow::Result;
use chrono::{NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set, FromQueryResult};
use serde::{Deserialize, Serialize};

use crate::shared::data::db::get_connection;

/// Модель OZON Finance Realization entry
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "p902_ozon_finance_realization")]
pub struct Model {
    // Composite Key (posting_number + sku + operation_type)
    #[sea_orm(primary_key, auto_increment = false)]
    pub posting_number: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub sku: String,

    // Metadata
    pub document_type: String,
    pub registrator_ref: String, // UUID источника данных

    // References
    pub connection_mp_ref: String,
    pub organization_ref: String,
    #[sea_orm(nullable)]
    pub posting_ref: Option<String>, // Ссылка на a010_ozon_fbs_posting (UUID)

    // Даты
    pub accrual_date: String,           // Дата начисления
    #[sea_orm(nullable)]
    pub operation_date: Option<String>, // Дата операции
    #[sea_orm(nullable)]
    pub delivery_date: Option<String>,  // Дата доставки

    // Информация о доставке
    #[sea_orm(nullable)]
    pub delivery_schema: Option<String>, // Схема доставки (FBS/FBO)
    #[sea_orm(nullable)]
    pub delivery_region: Option<String>, // Регион доставки
    #[sea_orm(nullable)]
    pub delivery_city: Option<String>,   // Город доставки

    // Количество и суммы
    pub quantity: f64,
    #[sea_orm(nullable)]
    pub price: Option<f64>,           // Цена товара
    pub amount: f64,                  // Сумма продажи
    #[sea_orm(nullable)]
    pub commission_amount: Option<f64>, // Сумма комиссии
    #[sea_orm(nullable)]
    pub commission_percent: Option<f64>, // Процент комиссии
    #[sea_orm(nullable)]
    pub services_amount: Option<f64>,   // Сумма доп. услуг
    #[sea_orm(nullable)]
    pub payout_amount: Option<f64>,     // Сумма к выплате

    // Тип операции
    #[sea_orm(primary_key, auto_increment = false)]
    pub operation_type: String,      // Тип операции
    #[sea_orm(nullable)]
    pub operation_type_name: Option<String>, // Название типа операции
    pub is_return: bool,             // Флаг возврата

    // Валюта
    #[sea_orm(nullable)]
    pub currency_code: Option<String>,

    // Технические поля
    pub loaded_at_utc: String,
    pub payload_version: i32,
    #[sea_orm(nullable)]
    pub extra: Option<String>, // JSON для дополнительных полей
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// Расширенная модель с sale_date из a010_ozon_fbs_posting
#[derive(Debug, Clone, Serialize, Deserialize, sea_orm::FromQueryResult)]
pub struct ModelWithSaleDate {
    // Все поля из Model
    pub posting_number: String,
    pub sku: String,
    pub document_type: String,
    pub registrator_ref: String,
    pub connection_mp_ref: String,
    pub organization_ref: String,
    pub posting_ref: Option<String>,
    pub accrual_date: String,
    pub operation_date: Option<String>,
    pub delivery_date: Option<String>,
    pub delivery_schema: Option<String>,
    pub delivery_region: Option<String>,
    pub delivery_city: Option<String>,
    pub quantity: f64,
    pub price: Option<f64>,
    pub amount: f64,
    pub commission_amount: Option<f64>,
    pub commission_percent: Option<f64>,
    pub services_amount: Option<f64>,
    pub payout_amount: Option<f64>,
    pub operation_type: String,
    pub operation_type_name: Option<String>,
    pub is_return: bool,
    pub currency_code: Option<String>,
    pub loaded_at_utc: String,
    pub payload_version: i32,
    pub extra: Option<String>,

    // Новое поле из JOIN с a010
    pub sale_date: Option<String>,
}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

/// Структура для передачи данных в upsert
#[derive(Debug, Clone)]
pub struct OzonFinanceRealizationEntry {
    // Composite Key
    pub posting_number: String,
    pub sku: String,

    // Metadata
    pub document_type: String,
    pub registrator_ref: String,

    // References
    pub connection_mp_ref: String,
    pub organization_ref: String,
    pub posting_ref: Option<String>,

    // Даты
    pub accrual_date: NaiveDate,
    pub operation_date: Option<NaiveDate>,
    pub delivery_date: Option<NaiveDate>,

    // Информация о доставке
    pub delivery_schema: Option<String>,
    pub delivery_region: Option<String>,
    pub delivery_city: Option<String>,

    // Количество и суммы
    pub quantity: f64,
    pub price: Option<f64>,
    pub amount: f64,
    pub commission_amount: Option<f64>,
    pub commission_percent: Option<f64>,
    pub services_amount: Option<f64>,
    pub payout_amount: Option<f64>,

    // Тип операции
    pub operation_type: String,
    pub operation_type_name: Option<String>,
    pub is_return: bool,

    // Валюта
    pub currency_code: Option<String>,

    // Технические
    pub payload_version: i32,
    pub extra: Option<String>,
}

/// Upsert записи в finance_realization по композитному ключу (posting_number, sku, operation_type)
pub async fn upsert_entry(entry: &OzonFinanceRealizationEntry) -> Result<()> {
    // Проверяем, существует ли запись
    let existing = Entity::find()
        .filter(Column::PostingNumber.eq(&entry.posting_number))
        .filter(Column::Sku.eq(&entry.sku))
        .filter(Column::OperationType.eq(&entry.operation_type))
        .one(conn())
        .await?;

    let now = Utc::now();
    let accrual_date_str = entry.accrual_date.format("%Y-%m-%d").to_string();
    let operation_date_str = entry.operation_date.map(|d| d.format("%Y-%m-%d").to_string());
    let delivery_date_str = entry.delivery_date.map(|d| d.format("%Y-%m-%d").to_string());

    let active = ActiveModel {
        posting_number: Set(entry.posting_number.clone()),
        sku: Set(entry.sku.clone()),
        document_type: Set(entry.document_type.clone()),
        registrator_ref: Set(entry.registrator_ref.clone()),
        connection_mp_ref: Set(entry.connection_mp_ref.clone()),
        organization_ref: Set(entry.organization_ref.clone()),
        posting_ref: Set(entry.posting_ref.clone()),
        accrual_date: Set(accrual_date_str),
        operation_date: Set(operation_date_str),
        delivery_date: Set(delivery_date_str),
        delivery_schema: Set(entry.delivery_schema.clone()),
        delivery_region: Set(entry.delivery_region.clone()),
        delivery_city: Set(entry.delivery_city.clone()),
        quantity: Set(entry.quantity),
        price: Set(entry.price),
        amount: Set(entry.amount),
        commission_amount: Set(entry.commission_amount),
        commission_percent: Set(entry.commission_percent),
        services_amount: Set(entry.services_amount),
        payout_amount: Set(entry.payout_amount),
        operation_type: Set(entry.operation_type.clone()),
        operation_type_name: Set(entry.operation_type_name.clone()),
        is_return: Set(entry.is_return),
        currency_code: Set(entry.currency_code.clone()),
        loaded_at_utc: Set(now.to_rfc3339()),
        payload_version: Set(entry.payload_version),
        extra: Set(entry.extra.clone()),
    };

    if existing.is_some() {
        active.update(conn()).await?;
    } else {
        active.insert(conn()).await?;
    }

    Ok(())
}

/// Получить одну запись по композитному ключу (posting_number, sku, operation_type)
pub async fn get_by_id(posting_number: &str, sku: &str, operation_type: &str) -> Result<Option<Model>> {
    let item = Entity::find()
        .filter(Column::PostingNumber.eq(posting_number))
        .filter(Column::Sku.eq(sku))
        .filter(Column::OperationType.eq(operation_type))
        .one(conn())
        .await?;
    Ok(item)
}

/// Получить список записей с фильтрами (с JOIN для sale_date из a010)
pub async fn list_with_filters(
    date_from: &str,
    date_to: &str,
    posting_number: Option<String>,
    sku: Option<String>,
    connection_mp_ref: Option<String>,
    organization_ref: Option<String>,
    operation_type: Option<String>,
    is_return: Option<bool>,
    has_posting_ref: Option<bool>,
    sort_by: &str,
    sort_desc: bool,
    limit: i32,
    offset: i32,
) -> Result<(Vec<ModelWithSaleDate>, i32)> {
    use sea_orm::Statement;
    use sea_orm::ConnectionTrait;

    // Построение WHERE условий
    let mut where_conditions = vec![
        format!("p902.accrual_date >= '{}'", date_from),
        format!("p902.accrual_date <= '{}'", date_to),
    ];

    if let Some(ref pn) = posting_number {
        where_conditions.push(format!("p902.posting_number LIKE '%{}%'", pn.replace("'", "''")));
    }

    if let Some(ref s) = sku {
        where_conditions.push(format!("p902.sku LIKE '%{}%'", s.replace("'", "''")));
    }

    if let Some(ref conn_ref) = connection_mp_ref {
        where_conditions.push(format!("p902.connection_mp_ref = '{}'", conn_ref.replace("'", "''")));
    }

    if let Some(ref org) = organization_ref {
        where_conditions.push(format!("p902.organization_ref = '{}'", org.replace("'", "''")));
    }

    if let Some(ref op_type) = operation_type {
        where_conditions.push(format!("p902.operation_type = '{}'", op_type.replace("'", "''")));
    }

    if let Some(is_ret) = is_return {
        where_conditions.push(format!("p902.is_return = {}", if is_ret { 1 } else { 0 }));
    }

    if let Some(has_ref) = has_posting_ref {
        if has_ref {
            where_conditions.push("p902.posting_ref IS NOT NULL".to_string());
        } else {
            where_conditions.push("p902.posting_ref IS NULL".to_string());
        }
    }

    let where_clause = where_conditions.join(" AND ");

    // Построение ORDER BY
    let order_clause = match sort_by {
        "posting_number" => format!("p902.posting_number {}", if sort_desc { "DESC" } else { "ASC" }),
        "sku" => format!("p902.sku {}", if sort_desc { "DESC" } else { "ASC" }),
        "amount" => format!("p902.amount {}", if sort_desc { "DESC" } else { "ASC" }),
        "sale_date" => format!("sale_date {}", if sort_desc { "DESC" } else { "ASC" }),
        _ => format!("p902.accrual_date {}", if sort_desc { "DESC" } else { "ASC" }),
    };

    // Count query
    let count_sql = format!(
        "SELECT COUNT(*) as count FROM p902_ozon_finance_realization p902 WHERE {}",
        where_clause
    );

    let count_stmt = Statement::from_string(sea_orm::DatabaseBackend::Sqlite, count_sql);
    let count_result = conn().query_one(count_stmt).await?;
    let total: i32 = count_result
        .map(|row| row.try_get("", "count").unwrap_or(0))
        .unwrap_or(0);

    // Main query with JOIN to p900_sales_register
    let sql = format!(
        r#"
        SELECT
            p902.posting_number,
            p902.sku,
            p902.document_type,
            p902.registrator_ref,
            p902.connection_mp_ref,
            p902.organization_ref,
            p902.posting_ref,
            p902.accrual_date,
            p902.operation_date,
            p902.delivery_date,
            p902.delivery_schema,
            p902.delivery_region,
            p902.delivery_city,
            p902.quantity,
            p902.price,
            p902.amount,
            p902.commission_amount,
            p902.commission_percent,
            p902.services_amount,
            p902.payout_amount,
            p902.operation_type,
            p902.operation_type_name,
            p902.is_return,
            p902.currency_code,
            p902.loaded_at_utc,
            p902.payload_version,
            p902.extra,
            p900.sale_date
        FROM p902_ozon_finance_realization p902
        LEFT JOIN p900_sales_register p900
            ON p902.posting_number = p900.document_no
            AND p900.marketplace = 'OZON'
        WHERE {}
        ORDER BY {}
        LIMIT {} OFFSET {}
        "#,
        where_clause,
        order_clause,
        limit,
        offset
    );

    let stmt = Statement::from_string(sea_orm::DatabaseBackend::Sqlite, sql);
    let items = ModelWithSaleDate::find_by_statement(stmt)
        .all(conn())
        .await?;

    Ok((items, total))
}

/// Найти все записи по posting_number (для связывания с a010)
pub async fn find_by_posting_number(posting_number: &str) -> Result<Vec<Model>> {
    let items = Entity::find()
        .filter(Column::PostingNumber.eq(posting_number))
        .all(conn())
        .await?;
    Ok(items)
}

/// Обновить posting_ref для записи
pub async fn update_posting_ref(
    posting_number: &str,
    sku: &str,
    operation_type: &str,
    posting_ref: Option<String>,
) -> Result<()> {
    let existing = Entity::find()
        .filter(Column::PostingNumber.eq(posting_number))
        .filter(Column::Sku.eq(sku))
        .filter(Column::OperationType.eq(operation_type))
        .one(conn())
        .await?;

    if let Some(model) = existing {
        let mut active: ActiveModel = model.into();
        active.posting_ref = Set(posting_ref);
        active.update(conn()).await?;
    }

    Ok(())
}

/// Получить статистику по периоду
pub async fn get_stats(
    date_from: &str,
    date_to: &str,
    connection_mp_ref: Option<String>,
) -> Result<StatsData> {
    let mut query = Entity::find()
        .filter(Column::AccrualDate.gte(date_from.to_string()))
        .filter(Column::AccrualDate.lte(date_to.to_string()));

    if let Some(conn_ref) = connection_mp_ref {
        query = query.filter(Column::ConnectionMpRef.eq(conn_ref));
    }

    let items = query.all(conn()).await?;

    let total_rows = items.len() as i32;
    let total_quantity: f64 = items.iter().map(|i| i.quantity).sum();
    let total_amount: f64 = items.iter().map(|i| i.amount).sum();
    let total_commission: f64 = items
        .iter()
        .map(|i| i.commission_amount.unwrap_or(0.0))
        .sum();
    let total_payout: f64 = items
        .iter()
        .map(|i| i.payout_amount.unwrap_or(0.0))
        .sum();

    // Подсчет уникальных постингов
    use std::collections::HashSet;
    let unique_postings: HashSet<String> = items.iter().map(|i| i.posting_number.clone()).collect();
    let unique_postings_count = unique_postings.len() as i32;

    // Подсчет постингов со ссылкой
    let linked_postings_count = items
        .iter()
        .filter(|i| i.posting_ref.is_some())
        .map(|i| i.posting_number.clone())
        .collect::<HashSet<String>>()
        .len() as i32;

    Ok(StatsData {
        total_rows,
        total_quantity,
        total_amount,
        total_commission,
        total_payout,
        unique_postings: unique_postings_count,
        linked_postings: linked_postings_count,
    })
}

#[derive(Debug, Clone)]
pub struct StatsData {
    pub total_rows: i32,
    pub total_quantity: f64,
    pub total_amount: f64,
    pub total_commission: f64,
    pub total_payout: f64,
    pub unique_postings: i32,
    pub linked_postings: i32,
}

/// Удалить все записи проекции для документа-регистратора
pub async fn delete_by_registrator(registrator_ref: &str) -> Result<u64> {
    let result = Entity::delete_many()
        .filter(Column::RegistratorRef.eq(registrator_ref))
        .exec(conn())
        .await?;
    Ok(result.rows_affected)
}
