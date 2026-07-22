use anyhow::Result;
use sea_orm::entity::prelude::*;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde::{Deserialize, Serialize};

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "p913_wb_advert_order_attr")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub connection_mp_ref: String,
    pub entry_date: String,
    pub turnover_code: String,
    pub amount: f64,
    #[sea_orm(nullable)]
    pub nomenclature_ref: Option<String>,
    pub wb_advert_campaign_code: String,
    pub order_key: String,
    pub registrator_type: String,
    pub registrator_ref: String,
    #[sea_orm(nullable)]
    pub general_ledger_ref: Option<String>,
    pub is_problem: bool,
    pub created_at: String,
    pub updated_at: String,
    /// Сумма заказа (price_with_disc / finished_price / price), к которой привязан расход на рекламу.
    /// Используется для отображения связанной "Реализации" в отчётах.
    #[sea_orm(default_value = 0.0)]
    pub sale_amount: f64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

pub async fn save_entry(entry: &Model) -> Result<()> {
    save_entry_with_conn(conn(), entry).await
}

pub async fn save_entry_with_conn<C: ConnectionTrait>(db: &C, entry: &Model) -> Result<()> {
    let active = ActiveModel {
        id: Set(entry.id.clone()),
        connection_mp_ref: Set(entry.connection_mp_ref.clone()),
        entry_date: Set(entry.entry_date.clone()),
        turnover_code: Set(entry.turnover_code.clone()),
        amount: Set(entry.amount),
        nomenclature_ref: Set(entry.nomenclature_ref.clone()),
        wb_advert_campaign_code: Set(entry.wb_advert_campaign_code.clone()),
        order_key: Set(entry.order_key.clone()),
        registrator_type: Set(entry.registrator_type.clone()),
        registrator_ref: Set(entry.registrator_ref.clone()),
        general_ledger_ref: Set(entry.general_ledger_ref.clone()),
        is_problem: Set(entry.is_problem),
        created_at: Set(entry.created_at.clone()),
        updated_at: Set(entry.updated_at.clone()),
        sale_amount: Set(entry.sale_amount),
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

/// Insert freshly built projection rows without per-row existence checks.
/// The caller must remove the registrator's previous rows in the same transaction first.
pub async fn insert_entries_bulk_with_conn<C: ConnectionTrait>(
    db: &C,
    entries: &[Model],
) -> Result<()> {
    insert_prepared_entries_with_conn(db, prepare_entries(entries)).await
}

pub fn prepare_entries(entries: &[Model]) -> Vec<ActiveModel> {
    entries
        .iter()
        .map(|entry| ActiveModel {
            id: Set(entry.id.clone()),
            connection_mp_ref: Set(entry.connection_mp_ref.clone()),
            entry_date: Set(entry.entry_date.clone()),
            turnover_code: Set(entry.turnover_code.clone()),
            amount: Set(entry.amount),
            nomenclature_ref: Set(entry.nomenclature_ref.clone()),
            wb_advert_campaign_code: Set(entry.wb_advert_campaign_code.clone()),
            order_key: Set(entry.order_key.clone()),
            registrator_type: Set(entry.registrator_type.clone()),
            registrator_ref: Set(entry.registrator_ref.clone()),
            general_ledger_ref: Set(entry.general_ledger_ref.clone()),
            is_problem: Set(entry.is_problem),
            created_at: Set(entry.created_at.clone()),
            updated_at: Set(entry.updated_at.clone()),
            sale_amount: Set(entry.sale_amount),
        })
        .collect()
}

pub async fn insert_prepared_entries_with_conn<C: ConnectionTrait>(
    db: &C,
    active_models: Vec<ActiveModel>,
) -> Result<()> {
    if active_models.is_empty() {
        return Ok(());
    }
    Entity::insert_many(active_models).exec(db).await?;
    Ok(())
}

pub async fn delete_by_registrator(registrator_type: &str, registrator_ref: &str) -> Result<u64> {
    delete_by_registrator_with_conn(conn(), registrator_type, registrator_ref).await
}

pub async fn delete_by_registrator_with_conn<C: ConnectionTrait>(
    db: &C,
    registrator_type: &str,
    registrator_ref: &str,
) -> Result<u64> {
    let result = Entity::delete_many()
        .filter(Column::RegistratorType.eq(registrator_type))
        .filter(Column::RegistratorRef.eq(registrator_ref))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

/// Удаляет все p913-строки a026 за период по кабинету.
/// При `advert_ids = Some` — только кампании из списка (scoped replace).
/// Удаляет в т.ч. «осиротевшие» строки, чей registrator_ref уже отсутствует в a026.
pub async fn delete_a026_by_connection_and_date_range_with_conn<C: ConnectionTrait>(
    db: &C,
    connection_mp_ref: &str,
    date_from: &str,
    date_to: &str,
    advert_ids: Option<&[i64]>,
) -> Result<u64> {
    let mut query = Entity::delete_many()
        .filter(Column::RegistratorType.eq("a026_wb_advert_daily"))
        .filter(Column::ConnectionMpRef.eq(connection_mp_ref))
        .filter(Column::EntryDate.gte(date_from))
        .filter(Column::EntryDate.lte(date_to));

    if let Some(ids) = advert_ids {
        if ids.is_empty() {
            return Ok(0);
        }
        let codes: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
        query = query.filter(Column::WbAdvertCampaignCode.is_in(codes));
    }

    let result = query.exec(db).await?;
    Ok(result.rows_affected)
}

pub async fn list_by_order_key_and_turnover(
    order_key: &str,
    turnover_code: &str,
) -> Result<Vec<Model>> {
    Ok(Entity::find()
        .filter(Column::OrderKey.eq(order_key))
        .filter(Column::TurnoverCode.eq(turnover_code))
        .order_by_asc(Column::EntryDate)
        .order_by_asc(Column::Id)
        .all(conn())
        .await?)
}

pub async fn count_with_filters(
    date_from: Option<String>,
    date_to: Option<String>,
    connection_mp_ref: Option<String>,
    turnover_code: Option<String>,
    order_key: Option<String>,
    wb_advert_campaign_code: Option<String>,
) -> Result<u64> {
    let mut q = Entity::find();
    if let Some(v) = date_from {
        q = q.filter(Column::EntryDate.gte(v));
    }
    if let Some(v) = date_to {
        q = q.filter(Column::EntryDate.lte(v));
    }
    if let Some(v) = connection_mp_ref {
        q = q.filter(Column::ConnectionMpRef.eq(v));
    }
    if let Some(v) = turnover_code {
        q = q.filter(Column::TurnoverCode.eq(v));
    }
    if let Some(v) = order_key {
        q = q.filter(Column::OrderKey.contains(v));
    }
    if let Some(v) = wb_advert_campaign_code {
        q = q.filter(Column::WbAdvertCampaignCode.eq(v));
    }
    Ok(q.count(conn()).await?)
}

#[allow(clippy::too_many_arguments)]
pub async fn list_with_filters(
    date_from: Option<String>,
    date_to: Option<String>,
    connection_mp_ref: Option<String>,
    turnover_code: Option<String>,
    order_key: Option<String>,
    wb_advert_campaign_code: Option<String>,
    sort_by: Option<String>,
    sort_desc: bool,
    offset: Option<u64>,
    limit: Option<u64>,
) -> Result<Vec<Model>> {
    let mut q = Entity::find();
    if let Some(v) = date_from {
        q = q.filter(Column::EntryDate.gte(v));
    }
    if let Some(v) = date_to {
        q = q.filter(Column::EntryDate.lte(v));
    }
    if let Some(v) = connection_mp_ref {
        q = q.filter(Column::ConnectionMpRef.eq(v));
    }
    if let Some(v) = turnover_code {
        q = q.filter(Column::TurnoverCode.eq(v));
    }
    if let Some(v) = order_key {
        q = q.filter(Column::OrderKey.contains(v));
    }
    if let Some(v) = wb_advert_campaign_code {
        q = q.filter(Column::WbAdvertCampaignCode.eq(v));
    }
    let col = match sort_by.as_deref() {
        Some("amount") => Column::Amount,
        Some("order_key") => Column::OrderKey,
        Some("turnover_code") => Column::TurnoverCode,
        _ => Column::EntryDate,
    };
    q = if sort_desc {
        q.order_by_desc(col).order_by_desc(Column::Id)
    } else {
        q.order_by_asc(col).order_by_asc(Column::Id)
    };
    if let Some(off) = offset {
        q = q.offset(off);
    }
    if let Some(lim) = limit {
        q = q.limit(lim);
    }
    Ok(q.all(conn()).await?)
}

pub async fn list_by_registrator(
    registrator_type: &str,
    registrator_ref: &str,
) -> Result<Vec<Model>> {
    Ok(Entity::find()
        .filter(Column::RegistratorType.eq(registrator_type))
        .filter(Column::RegistratorRef.eq(registrator_ref))
        .order_by_asc(Column::EntryDate)
        .order_by_asc(Column::OrderKey)
        .order_by_asc(Column::Id)
        .all(conn())
        .await?)
}

/// Возвращает суммарную атрибуцию (advert_clicks_order_accrual) по каждому order_key
/// из переданного списка. Ключи без записей в таблице отсутствуют в результате
/// (считается 0.0).
///
/// `exclude_registrator_ref` — если указан, записи с этим registrator_ref
/// исключаются из подсчёта. Используется при перепроведении: текущий документ
/// должен быть исключён, чтобы его собственные старые записи не «штрафовали»
/// ранее выбранные заказы.
/// Агрегаты p913 по wb_advert_campaign_code (= advert_id из a030).
/// Возвращает HashMap<campaign_code, (reserve_sum, expense_sum, realization_sum)>:
/// - reserve_sum: SUM(amount) where turnover_code='advert_clicks_order_accrual'
/// - expense_sum: SUM(amount) where turnover_code='advert_clicks_order_expense'
/// - realization_sum: SUM(sale_amount) where turnover_code='advert_clicks_order_expense'
pub async fn aggregate_by_campaign() -> Result<std::collections::HashMap<String, (f64, f64, f64)>> {
    use sea_orm::{ConnectionTrait, Statement};

    let sql = "SELECT \
        wb_advert_campaign_code AS campaign_code, \
        COALESCE(SUM(CASE WHEN turnover_code = 'advert_clicks_order_accrual' THEN amount ELSE 0 END), 0) AS reserve_sum, \
        COALESCE(SUM(CASE WHEN turnover_code = 'advert_clicks_order_expense' THEN amount ELSE 0 END), 0) AS expense_sum, \
        COALESCE(SUM(CASE WHEN turnover_code = 'advert_clicks_order_expense' THEN sale_amount ELSE 0 END), 0) AS realization_sum \
        FROM p913_wb_advert_order_attr \
        GROUP BY wb_advert_campaign_code"
        .to_string();

    let stmt = Statement::from_string(sea_orm::DatabaseBackend::Sqlite, sql);
    let rows = conn().query_all(stmt).await?;

    let mut map = std::collections::HashMap::with_capacity(rows.len());
    for row in rows {
        let key: String = row.try_get("", "campaign_code").unwrap_or_default();
        let reserve: f64 = row.try_get("", "reserve_sum").unwrap_or(0.0);
        let expense: f64 = row.try_get("", "expense_sum").unwrap_or(0.0);
        let realization: f64 = row.try_get("", "realization_sum").unwrap_or(0.0);
        map.insert(key, (reserve, expense, realization));
    }
    Ok(map)
}

pub async fn sum_reserve_by_order_keys(
    order_keys: &[String],
    exclude_registrator_ref: Option<&str>,
) -> Result<std::collections::HashMap<String, f64>> {
    use sea_orm::{ConnectionTrait, Statement};

    if order_keys.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    let placeholders: String = order_keys
        .iter()
        .map(|k| format!("'{}'", k.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(", ");

    let exclude_clause = match exclude_registrator_ref {
        Some(r) => format!(" AND registrator_ref != '{}'", r.replace('\'', "''")),
        None => String::new(),
    };

    let sql = format!(
        "SELECT order_key, SUM(amount) AS total \
         FROM p913_wb_advert_order_attr \
         WHERE order_key IN ({placeholders}) \
           AND turnover_code = 'advert_clicks_order_accrual'{exclude_clause} \
         GROUP BY order_key"
    );

    let stmt = Statement::from_string(sea_orm::DatabaseBackend::Sqlite, sql);
    let rows = conn().query_all(stmt).await?;

    let mut map = std::collections::HashMap::with_capacity(rows.len());
    for row in rows {
        let key: String = row.try_get("", "order_key").unwrap_or_default();
        let total: f64 = row.try_get("", "total").unwrap_or(0.0);
        map.insert(key, total);
    }
    Ok(map)
}
