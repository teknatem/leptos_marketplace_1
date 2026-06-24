use anyhow::Result;
use chrono::Utc;
use contracts::domain::a034_ym_realization::aggregate::{
    YmRealization, YmRealizationHeader, YmRealizationSourceMeta, YmRealizationTotals,
};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use sea_orm::entity::prelude::*;
use sea_orm::{
    ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set, Statement, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::data::db::get_connection;

const REGISTRATOR_TYPE: &str = "a034_ym_realization";

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a034_ym_realization")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub document_no: String,
    pub document_date: String,
    pub connection_id: String,
    pub organization_id: String,
    pub marketplace_id: String,
    pub lines_count: i32,
    pub total_sales_revenue: f64,
    pub total_return_revenue: f64,
    pub net_revenue: f64,
    pub header_json: String,
    pub totals_json: String,
    /// Строки-продажи (delivered) — отдельная коллекция, не смешивается с возвратами.
    pub sales_lines_json: String,
    /// Строки-возвраты (returned) — отдельная коллекция.
    pub return_lines_json: String,
    pub source_meta_json: String,
    pub fetched_at: String,
    pub is_deleted: bool,
    pub is_posted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for YmRealization {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());
        let header: YmRealizationHeader =
            serde_json::from_str(&m.header_json).unwrap_or(YmRealizationHeader {
                document_no: m.document_no.clone(),
                document_date: m.document_date.clone(),
                connection_id: m.connection_id.clone(),
                organization_id: m.organization_id.clone(),
                marketplace_id: m.marketplace_id.clone(),
            });
        let totals = serde_json::from_str(&m.totals_json).unwrap_or(YmRealizationTotals {
            sales_revenue: m.total_sales_revenue,
            return_revenue: m.total_return_revenue,
            net_revenue: m.net_revenue,
            sales_qty: 0.0,
            return_qty: 0.0,
            net_qty: 0.0,
        });
        let sales_lines = serde_json::from_str(&m.sales_lines_json).unwrap_or_default();
        let return_lines = serde_json::from_str(&m.return_lines_json).unwrap_or_default();
        let source_meta =
            serde_json::from_str(&m.source_meta_json).unwrap_or(YmRealizationSourceMeta {
                source: "ym_goods_realization".to_string(),
                fetched_at: m.fetched_at.clone(),
            });

        YmRealization {
            base: BaseAggregate::with_metadata(
                contracts::domain::a034_ym_realization::aggregate::YmRealizationId::new(uuid),
                m.code,
                m.description,
                m.comment,
                metadata,
            ),
            header,
            totals,
            sales_lines,
            return_lines,
            source_meta,
            is_posted: m.is_posted,
        }
    }
}

fn to_active_model(
    document: &YmRealization,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<ActiveModel> {
    let header_json = serde_json::to_string(&document.header)?;
    let totals_json = serde_json::to_string(&document.totals)?;
    let sales_lines_json = serde_json::to_string(&document.sales_lines)?;
    let return_lines_json = serde_json::to_string(&document.return_lines)?;
    let source_meta_json = serde_json::to_string(&document.source_meta)?;

    Ok(ActiveModel {
        id: Set(document.base.id.value().to_string()),
        code: Set(document.base.code.clone()),
        description: Set(document.base.description.clone()),
        comment: Set(document.base.comment.clone()),
        document_no: Set(document.header.document_no.clone()),
        document_date: Set(document.header.document_date.clone()),
        connection_id: Set(document.header.connection_id.clone()),
        organization_id: Set(document.header.organization_id.clone()),
        marketplace_id: Set(document.header.marketplace_id.clone()),
        lines_count: Set(document.lines_count() as i32),
        total_sales_revenue: Set(document.totals.sales_revenue),
        total_return_revenue: Set(document.totals.return_revenue),
        net_revenue: Set(document.totals.net_revenue),
        header_json: Set(header_json),
        totals_json: Set(totals_json),
        sales_lines_json: Set(sales_lines_json),
        return_lines_json: Set(return_lines_json),
        source_meta_json: Set(source_meta_json),
        fetched_at: Set(document.source_meta.fetched_at.clone()),
        is_deleted: Set(document.base.metadata.is_deleted),
        is_posted: Set(document.base.metadata.is_posted || document.is_posted),
        created_at: Set(created_at.or(Some(document.base.metadata.created_at))),
        updated_at: Set(Some(Utc::now())),
        version: Set(document.base.metadata.version),
    })
}

/// Заменяет документы за период по кабинету: удаляет GL-проводки слоя ybuh по
/// существующим документам, удаляет сами документы и вставляет новые. Идемпотентно.
pub async fn replace_for_period(
    connection_id: &str,
    date_from: &str,
    date_to: &str,
    documents: &[YmRealization],
) -> Result<usize> {
    let db = get_connection();
    let txn = db.begin().await?;

    let existing_ids: Vec<String> = Entity::find()
        .filter(Column::ConnectionId.eq(connection_id))
        .filter(Column::DocumentDate.gte(date_from))
        .filter(Column::DocumentDate.lte(date_to))
        .all(&txn)
        .await?
        .into_iter()
        .map(|item| item.id)
        .collect();

    for id in &existing_ids {
        crate::projections::general_ledger::repository::delete_by_registrator_with_conn(
            &txn,
            REGISTRATOR_TYPE,
            id,
        )
        .await?;
    }

    Entity::delete_many()
        .filter(Column::ConnectionId.eq(connection_id))
        .filter(Column::DocumentDate.gte(date_from))
        .filter(Column::DocumentDate.lte(date_to))
        .exec(&txn)
        .await?;

    for document in documents {
        Entity::insert(to_active_model(document, Some(Utc::now()))?)
            .exec(&txn)
            .await?;
    }

    txn.commit().await?;
    Ok(documents.len())
}

pub async fn upsert_document(document: &YmRealization) -> Result<()> {
    let db = get_connection();
    let existing = Entity::find_by_id(document.base.id.value().to_string())
        .one(db)
        .await?;
    let created_at = existing.as_ref().and_then(|item| item.created_at);
    let active_model = to_active_model(document, created_at)?;
    if existing.is_some() {
        active_model.update(db).await?;
    } else {
        active_model.insert(db).await?;
    }
    Ok(())
}

/// Обновляет существующий документ; не создаёт новый (защита от гонки с replace_for_period).
pub async fn update_document_with_conn<C: ConnectionTrait>(
    db: &C,
    document: &YmRealization,
) -> Result<()> {
    let id = document.base.id.value().to_string();
    let existing = Entity::find_by_id(id.clone())
        .one(db)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found for update: {}", id))?;
    let active_model = to_active_model(document, existing.created_at)?;
    active_model.update(db).await?;
    Ok(())
}

pub async fn get_by_id(id: Uuid) -> Result<Option<YmRealization>> {
    let db = get_connection();
    let model = Entity::find_by_id(id.to_string()).one(db).await?;
    Ok(model.map(Into::into))
}

pub async fn exists_with_conn<C: ConnectionTrait>(db: &C, id: &str) -> Result<bool> {
    Ok(Entity::find_by_id(id.to_string()).one(db).await?.is_some())
}

pub async fn list_ids_by_period(
    date_from: &str,
    date_to: &str,
    only_posted: bool,
) -> Result<Vec<String>> {
    let db = get_connection();
    let mut query = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .filter(Column::DocumentDate.gte(date_from))
        .filter(Column::DocumentDate.lte(date_to));
    if only_posted {
        query = query.filter(Column::IsPosted.eq(true));
    }
    let items = query.all(db).await?;
    Ok(items.into_iter().map(|item| item.id).collect())
}

#[derive(Debug, Clone)]
pub struct YmRealizationListQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub connection_id: Option<String>,
    pub search_query: Option<String>,
    pub sort_by: String,
    pub sort_desc: bool,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone)]
pub struct YmRealizationListRow {
    pub id: String,
    pub document_no: String,
    pub document_date: String,
    pub lines_count: i32,
    pub total_sales_revenue: f64,
    pub total_return_revenue: f64,
    pub net_revenue: f64,
    pub connection_id: String,
    pub connection_name: Option<String>,
    pub organization_name: Option<String>,
    pub fetched_at: String,
    pub is_posted: bool,
}

#[derive(Debug, Clone)]
pub struct YmRealizationListResult {
    pub items: Vec<YmRealizationListRow>,
    pub total: usize,
}

pub async fn list_sql(query: YmRealizationListQuery) -> Result<YmRealizationListResult> {
    let db = get_connection();

    let mut conditions = vec!["d.is_deleted = 0".to_string()];
    if let Some(ref date_from) = query.date_from {
        if !date_from.is_empty() {
            conditions.push(format!(
                "d.document_date >= '{}'",
                date_from.replace('\'', "''")
            ));
        }
    }
    if let Some(ref date_to) = query.date_to {
        if !date_to.is_empty() {
            conditions.push(format!(
                "d.document_date <= '{}'",
                date_to.replace('\'', "''")
            ));
        }
    }
    if let Some(ref connection_id) = query.connection_id {
        if !connection_id.is_empty() {
            conditions.push(format!(
                "d.connection_id = '{}'",
                connection_id.replace('\'', "''")
            ));
        }
    }
    if let Some(ref search) = query.search_query {
        if !search.is_empty() {
            let escaped = search.replace('\'', "''");
            conditions.push(format!(
                "(d.document_no LIKE '%{0}%' OR c.description LIKE '%{0}%' OR o.description LIKE '%{0}%')",
                escaped
            ));
        }
    }

    let where_clause = conditions.join(" AND ");
    let sort_column = match query.sort_by.as_str() {
        "document_no" => "d.document_no",
        "document_date" => "d.document_date",
        "lines_count" => "d.lines_count",
        "total_sales_revenue" => "d.total_sales_revenue",
        "total_return_revenue" => "d.total_return_revenue",
        "net_revenue" => "d.net_revenue",
        "connection_name" => "c.description",
        "organization_name" => "o.description",
        "fetched_at" => "d.fetched_at",
        _ => "d.document_date",
    };
    let sort_dir = if query.sort_desc { "DESC" } else { "ASC" };

    let count_sql = format!(
        "SELECT COUNT(*) as cnt
         FROM a034_ym_realization d
         LEFT JOIN a006_connection_mp c ON c.id = d.connection_id
         LEFT JOIN a002_organization o ON o.id = d.organization_id
         WHERE {}",
        where_clause
    );

    let list_sql = format!(
        "SELECT
            d.id,
            d.document_no,
            d.document_date,
            d.lines_count,
            d.total_sales_revenue,
            d.total_return_revenue,
            d.net_revenue,
            d.connection_id,
            c.description as connection_name,
            o.description as organization_name,
            d.fetched_at,
            d.is_posted
         FROM a034_ym_realization d
         LEFT JOIN a006_connection_mp c ON c.id = d.connection_id
         LEFT JOIN a002_organization o ON o.id = d.organization_id
         WHERE {}
         ORDER BY {} {}
         LIMIT {} OFFSET {}",
        where_clause, sort_column, sort_dir, query.limit, query.offset
    );

    let count_result = db
        .query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            count_sql,
        ))
        .await?;
    let total = count_result
        .and_then(|row| row.try_get::<i64>("", "cnt").ok())
        .unwrap_or(0) as usize;

    let rows = db
        .query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            list_sql,
        ))
        .await?;

    let items = rows
        .into_iter()
        .map(|row| YmRealizationListRow {
            id: row.try_get("", "id").unwrap_or_default(),
            document_no: row.try_get("", "document_no").unwrap_or_default(),
            document_date: row.try_get("", "document_date").unwrap_or_default(),
            lines_count: row.try_get("", "lines_count").unwrap_or(0),
            total_sales_revenue: row.try_get("", "total_sales_revenue").unwrap_or(0.0),
            total_return_revenue: row.try_get("", "total_return_revenue").unwrap_or(0.0),
            net_revenue: row.try_get("", "net_revenue").unwrap_or(0.0),
            connection_id: row.try_get("", "connection_id").unwrap_or_default(),
            connection_name: row.try_get("", "connection_name").ok(),
            organization_name: row.try_get("", "organization_name").ok(),
            fetched_at: row.try_get("", "fetched_at").unwrap_or_default(),
            is_posted: row.try_get::<bool>("", "is_posted").unwrap_or(false),
        })
        .collect();

    Ok(YmRealizationListResult { items, total })
}

/// Строка реализации a034 по заказу (для дашборда «Вся история»).
#[derive(Clone, Debug)]
pub struct LineByOrder {
    pub doc_id: String,
    pub document_no: String,
    pub document_date: String,
    pub shop_sku: String,
    pub offer_name: String,
    pub quantity: f64,
    pub revenue_amount: f64,
    pub is_return: bool,
}

/// Все строки реализации по номеру заказа YM (`order_id`). Продажи и возвраты
/// хранятся в отдельных JSON-коллекциях (`sales_lines_json` / `return_lines_json`) —
/// разворачиваем обе через `json_each` и объединяем.
pub async fn lines_by_order_id(order_id: &str) -> Result<Vec<LineByOrder>> {
    let db = get_connection();

    let sql = "
        SELECT
            d.id                                       AS doc_id,
            d.document_no                              AS document_no,
            d.document_date                            AS document_date,
            json_extract(li.value, '$.shop_sku')       AS shop_sku,
            json_extract(li.value, '$.offer_name')     AS offer_name,
            json_extract(li.value, '$.quantity')       AS quantity,
            json_extract(li.value, '$.revenue_amount') AS revenue_amount,
            json_extract(li.value, '$.is_return')      AS is_return
        FROM a034_ym_realization d, json_each(d.sales_lines_json) li
        WHERE d.is_deleted = 0
          AND json_extract(li.value, '$.order_id') = ?
        UNION ALL
        SELECT
            d.id                                       AS doc_id,
            d.document_no                              AS document_no,
            d.document_date                            AS document_date,
            json_extract(li.value, '$.shop_sku')       AS shop_sku,
            json_extract(li.value, '$.offer_name')     AS offer_name,
            json_extract(li.value, '$.quantity')       AS quantity,
            json_extract(li.value, '$.revenue_amount') AS revenue_amount,
            json_extract(li.value, '$.is_return')      AS is_return
        FROM a034_ym_realization d, json_each(d.return_lines_json) li
        WHERE d.is_deleted = 0
          AND json_extract(li.value, '$.order_id') = ?
        ORDER BY document_date ASC";

    let rows = db
        .query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            sql,
            [order_id.into(), order_id.into()],
        ))
        .await?;

    let items = rows
        .into_iter()
        .map(|row| LineByOrder {
            doc_id: row.try_get("", "doc_id").unwrap_or_default(),
            document_no: row.try_get("", "document_no").unwrap_or_default(),
            document_date: row.try_get("", "document_date").unwrap_or_default(),
            shop_sku: row.try_get("", "shop_sku").unwrap_or_default(),
            offer_name: row.try_get("", "offer_name").unwrap_or_default(),
            quantity: row.try_get("", "quantity").unwrap_or(0.0),
            revenue_amount: row.try_get("", "revenue_amount").unwrap_or(0.0),
            // json_extract возвращает 0/1 для boolean — читаем как i64.
            is_return: row.try_get::<i64>("", "is_return").unwrap_or(0) != 0,
        })
        .collect();

    Ok(items)
}

/// Все документы за период по кабинету (для импорта/replace), без пагинации.
pub async fn order_by_document_date(connection_id: &str) -> Result<Vec<YmRealization>> {
    let db = get_connection();
    let models = Entity::find()
        .filter(Column::ConnectionId.eq(connection_id))
        .filter(Column::IsDeleted.eq(false))
        .order_by_asc(Column::DocumentDate)
        .all(db)
        .await?;
    Ok(models.into_iter().map(Into::into).collect())
}
