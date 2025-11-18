use anyhow::Result;
use chrono::{NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set};
use serde::{Deserialize, Serialize};

use crate::shared::data::db::get_connection;

/// Модель Wildberries Finance Report entry
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "p903_wb_finance_report")]
pub struct Model {
    // Composite Primary Key
    #[sea_orm(primary_key, auto_increment = false)]
    pub rr_dt: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub rrd_id: i64,

    // Metadata
    pub connection_mp_ref: String,
    pub organization_ref: String,

    // Main Fields (22 specified fields)
    #[sea_orm(nullable)]
    pub acquiring_fee: Option<f64>,
    #[sea_orm(nullable)]
    pub acquiring_percent: Option<f64>,
    #[sea_orm(nullable)]
    pub additional_payment: Option<f64>,
    #[sea_orm(nullable)]
    pub bonus_type_name: Option<String>,
    #[sea_orm(nullable)]
    pub commission_percent: Option<f64>,
    #[sea_orm(nullable)]
    pub delivery_amount: Option<f64>,
    #[sea_orm(nullable)]
    pub delivery_rub: Option<f64>,
    #[sea_orm(nullable)]
    pub nm_id: Option<i64>,
    #[sea_orm(nullable)]
    pub penalty: Option<f64>,
    #[sea_orm(nullable)]
    pub ppvz_vw: Option<f64>,
    #[sea_orm(nullable)]
    pub ppvz_vw_nds: Option<f64>,
    #[sea_orm(nullable)]
    pub ppvz_sales_commission: Option<f64>,
    #[sea_orm(nullable)]
    pub quantity: Option<i32>,
    #[sea_orm(nullable)]
    pub rebill_logistic_cost: Option<f64>,
    #[sea_orm(nullable)]
    pub retail_amount: Option<f64>,
    #[sea_orm(nullable)]
    pub retail_price: Option<f64>,
    #[sea_orm(nullable)]
    pub retail_price_withdisc_rub: Option<f64>,
    #[sea_orm(nullable)]
    pub return_amount: Option<f64>,
    #[sea_orm(nullable)]
    pub sa_name: Option<String>,
    #[sea_orm(nullable)]
    pub storage_fee: Option<f64>,
    #[sea_orm(nullable)]
    pub subject_name: Option<String>,
    #[sea_orm(nullable)]
    pub supplier_oper_name: Option<String>,
    #[sea_orm(nullable)]
    pub cashback_amount: Option<f64>,
    #[sea_orm(nullable)]
    pub ppvz_for_pay: Option<f64>,
    #[sea_orm(nullable)]
    pub ppvz_kvw_prc: Option<f64>,
    #[sea_orm(nullable)]
    pub ppvz_kvw_prc_base: Option<f64>,
    #[sea_orm(nullable)]
    pub srv_dbs: Option<i32>,

    // Technical fields
    pub loaded_at_utc: String,
    pub payload_version: i32,
    #[sea_orm(nullable)]
    pub extra: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// Структура для передачи данных в upsert
#[derive(Debug, Clone)]
pub struct WbFinanceReportEntry {
    // Composite Key
    pub rr_dt: NaiveDate,
    pub rrd_id: i64,

    // Metadata
    pub connection_mp_ref: String,
    pub organization_ref: String,

    // Main Fields
    pub acquiring_fee: Option<f64>,
    pub acquiring_percent: Option<f64>,
    pub additional_payment: Option<f64>,
    pub bonus_type_name: Option<String>,
    pub commission_percent: Option<f64>,
    pub delivery_amount: Option<f64>,
    pub delivery_rub: Option<f64>,
    pub nm_id: Option<i64>,
    pub penalty: Option<f64>,
    pub ppvz_vw: Option<f64>,
    pub ppvz_vw_nds: Option<f64>,
    pub ppvz_sales_commission: Option<f64>,
    pub quantity: Option<i32>,
    pub rebill_logistic_cost: Option<f64>,
    pub retail_amount: Option<f64>,
    pub retail_price: Option<f64>,
    pub retail_price_withdisc_rub: Option<f64>,
    pub return_amount: Option<f64>,
    pub sa_name: Option<String>,
    pub storage_fee: Option<f64>,
    pub subject_name: Option<String>,
    pub supplier_oper_name: Option<String>,
    pub cashback_amount: Option<f64>,
    pub ppvz_for_pay: Option<f64>,
    pub ppvz_kvw_prc: Option<f64>,
    pub ppvz_kvw_prc_base: Option<f64>,
    pub srv_dbs: Option<i32>,

    // Technical
    pub payload_version: i32,
    pub extra: Option<String>,
}

/// Удалить все записи за указанную дату (для обновления данных)
pub async fn delete_by_date(date: NaiveDate) -> Result<u64> {
    let db = get_connection();
    let date_str = date.format("%Y-%m-%d").to_string();

    let result = Entity::delete_many()
        .filter(Column::RrDt.eq(date_str))
        .exec(db)
        .await?;

    Ok(result.rows_affected)
}

/// Upsert записи в finance_report по композитному ключу (rr_dt, rrd_id)
pub async fn upsert_entry(entry: &WbFinanceReportEntry) -> Result<()> {
    let db = get_connection();
    let rr_dt_str = entry.rr_dt.format("%Y-%m-%d").to_string();

    // Проверяем, существует ли запись
    let existing = Entity::find()
        .filter(Column::RrDt.eq(&rr_dt_str))
        .filter(Column::RrdId.eq(entry.rrd_id))
        .one(db)
        .await?;

    let loaded_at_utc = Utc::now().to_rfc3339();

    if let Some(existing_model) = existing {
        // Обновить существующую запись
        let mut active_model: ActiveModel = existing_model.into();
        
        active_model.connection_mp_ref = Set(entry.connection_mp_ref.clone());
        active_model.organization_ref = Set(entry.organization_ref.clone());
        active_model.acquiring_fee = Set(entry.acquiring_fee);
        active_model.acquiring_percent = Set(entry.acquiring_percent);
        active_model.additional_payment = Set(entry.additional_payment);
        active_model.bonus_type_name = Set(entry.bonus_type_name.clone());
        active_model.commission_percent = Set(entry.commission_percent);
        active_model.delivery_amount = Set(entry.delivery_amount);
        active_model.delivery_rub = Set(entry.delivery_rub);
        active_model.nm_id = Set(entry.nm_id);
        active_model.penalty = Set(entry.penalty);
        active_model.ppvz_vw = Set(entry.ppvz_vw);
        active_model.ppvz_vw_nds = Set(entry.ppvz_vw_nds);
        active_model.ppvz_sales_commission = Set(entry.ppvz_sales_commission);
        active_model.quantity = Set(entry.quantity);
        active_model.rebill_logistic_cost = Set(entry.rebill_logistic_cost);
        active_model.retail_amount = Set(entry.retail_amount);
        active_model.retail_price = Set(entry.retail_price);
        active_model.retail_price_withdisc_rub = Set(entry.retail_price_withdisc_rub);
        active_model.return_amount = Set(entry.return_amount);
        active_model.sa_name = Set(entry.sa_name.clone());
        active_model.storage_fee = Set(entry.storage_fee);
        active_model.subject_name = Set(entry.subject_name.clone());
        active_model.supplier_oper_name = Set(entry.supplier_oper_name.clone());
        active_model.cashback_amount = Set(entry.cashback_amount);
        active_model.ppvz_for_pay = Set(entry.ppvz_for_pay);
        active_model.ppvz_kvw_prc = Set(entry.ppvz_kvw_prc);
        active_model.ppvz_kvw_prc_base = Set(entry.ppvz_kvw_prc_base);
        active_model.srv_dbs = Set(entry.srv_dbs);
        active_model.loaded_at_utc = Set(loaded_at_utc);
        active_model.payload_version = Set(entry.payload_version);
        active_model.extra = Set(entry.extra.clone());

        active_model.update(db).await?;
    } else {
        // Вставить новую запись
        let new_model = ActiveModel {
            rr_dt: Set(rr_dt_str),
            rrd_id: Set(entry.rrd_id),
            connection_mp_ref: Set(entry.connection_mp_ref.clone()),
            organization_ref: Set(entry.organization_ref.clone()),
            acquiring_fee: Set(entry.acquiring_fee),
            acquiring_percent: Set(entry.acquiring_percent),
            additional_payment: Set(entry.additional_payment),
            bonus_type_name: Set(entry.bonus_type_name.clone()),
            commission_percent: Set(entry.commission_percent),
            delivery_amount: Set(entry.delivery_amount),
            delivery_rub: Set(entry.delivery_rub),
            nm_id: Set(entry.nm_id),
            penalty: Set(entry.penalty),
            ppvz_vw: Set(entry.ppvz_vw),
            ppvz_vw_nds: Set(entry.ppvz_vw_nds),
            ppvz_sales_commission: Set(entry.ppvz_sales_commission),
            quantity: Set(entry.quantity),
            rebill_logistic_cost: Set(entry.rebill_logistic_cost),
            retail_amount: Set(entry.retail_amount),
            retail_price: Set(entry.retail_price),
            retail_price_withdisc_rub: Set(entry.retail_price_withdisc_rub),
            return_amount: Set(entry.return_amount),
            sa_name: Set(entry.sa_name.clone()),
            storage_fee: Set(entry.storage_fee),
            subject_name: Set(entry.subject_name.clone()),
            supplier_oper_name: Set(entry.supplier_oper_name.clone()),
            cashback_amount: Set(entry.cashback_amount),
            ppvz_for_pay: Set(entry.ppvz_for_pay),
            ppvz_kvw_prc: Set(entry.ppvz_kvw_prc),
            ppvz_kvw_prc_base: Set(entry.ppvz_kvw_prc_base),
            srv_dbs: Set(entry.srv_dbs),
            loaded_at_utc: Set(loaded_at_utc),
            payload_version: Set(entry.payload_version),
            extra: Set(entry.extra.clone()),
        };

        new_model.insert(db).await?;
    }

    Ok(())
}

/// Получить список записей с фильтрами
pub async fn list_with_filters(
    date_from: &str,
    date_to: &str,
    nm_id: Option<i64>,
    sa_name: Option<String>,
    connection_mp_ref: Option<String>,
    organization_ref: Option<String>,
    sort_by: &str,
    sort_desc: bool,
    limit: i32,
    offset: i32,
) -> Result<(Vec<Model>, i32)> {
    let db = get_connection();

    // Построить запрос с фильтрами
    let mut query = Entity::find();

    // Фильтр по дате
    query = query
        .filter(Column::RrDt.gte(date_from))
        .filter(Column::RrDt.lte(date_to));

    // Фильтр по nm_id
    if let Some(nm) = nm_id {
        query = query.filter(Column::NmId.eq(nm));
    }

    // Фильтр по sa_name (артикул продавца)
    if let Some(ref sa) = sa_name {
        if !sa.is_empty() {
            query = query.filter(Column::SaName.contains(sa));
        }
    }

    // Фильтр по connection_mp_ref
    if let Some(ref conn) = connection_mp_ref {
        query = query.filter(Column::ConnectionMpRef.eq(conn));
    }

    // Фильтр по organization_ref
    if let Some(ref org) = organization_ref {
        query = query.filter(Column::OrganizationRef.eq(org));
    }

    // Подсчет общего количества записей (до пагинации)
    let total_count = query.clone().count(db).await? as i32;

    // Сортировка
    query = match sort_by {
        "rr_dt" => {
            if sort_desc {
                query.order_by_desc(Column::RrDt)
            } else {
                query.order_by_asc(Column::RrDt)
            }
        }
        "nm_id" => {
            if sort_desc {
                query.order_by_desc(Column::NmId)
            } else {
                query.order_by_asc(Column::NmId)
            }
        }
        "sa_name" => {
            if sort_desc {
                query.order_by_desc(Column::SaName)
            } else {
                query.order_by_asc(Column::SaName)
            }
        }
        "quantity" => {
            if sort_desc {
                query.order_by_desc(Column::Quantity)
            } else {
                query.order_by_asc(Column::Quantity)
            }
        }
        "retail_amount" => {
            if sort_desc {
                query.order_by_desc(Column::RetailAmount)
            } else {
                query.order_by_asc(Column::RetailAmount)
            }
        }
        _ => {
            if sort_desc {
                query.order_by_desc(Column::RrDt)
            } else {
                query.order_by_asc(Column::RrDt)
            }
        }
    };

    // Пагинация
    let items = query
        .limit(limit as u64)
        .offset(offset as u64)
        .all(db)
        .await?;

    Ok((items, total_count))
}

/// Получить запись по композитному ключу
pub async fn get_by_id(rr_dt: &str, rrd_id: i64) -> Result<Option<Model>> {
    let db = get_connection();

    let item = Entity::find()
        .filter(Column::RrDt.eq(rr_dt))
        .filter(Column::RrdId.eq(rrd_id))
        .one(db)
        .await?;

    Ok(item)
}

