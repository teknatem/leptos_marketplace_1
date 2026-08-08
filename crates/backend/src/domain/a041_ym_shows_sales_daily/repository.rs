//! Репозиторий a041 — дневная воронка YM из отчёта «Аналитика продаж».
//!
//! По образцу `a036_wb_sales_funnel_daily::repository`: документ = кабинет × дата,
//! импорт заменяет период целиком (`replace_for_period`) и в той же транзакции
//! пересобирает маркетинговые движения p916.

use anyhow::Result;
use chrono::Utc;
use contracts::domain::a041_ym_shows_sales_daily::aggregate::{
    YmShowsSalesDaily, YmShowsSalesDailyHeader, YmShowsSalesDailyId, YmShowsSalesDailySourceMeta,
};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use sea_orm::entity::prelude::*;
use sea_orm::{ConnectionTrait, QueryOrder, Set, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a041_ym_shows_sales_daily")]
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
    #[sea_orm(nullable)]
    pub campaign_id: Option<String>,
    pub lines_count: i32,
    // Денормализованные итоги дня. NULL — метрики не было в отчёте (N/A ≠ 0).
    #[sea_orm(nullable)]
    pub total_shows: Option<i64>,
    #[sea_orm(nullable)]
    pub total_clicks: Option<i64>,
    #[sea_orm(nullable)]
    pub total_to_cart: Option<i64>,
    #[sea_orm(nullable)]
    pub total_order_items: Option<i64>,
    #[sea_orm(nullable)]
    pub total_delivered_count: Option<i64>,
    #[sea_orm(nullable)]
    pub total_canceled_count: Option<i64>,
    #[sea_orm(nullable)]
    pub total_returned_count: Option<i64>,
    pub header_json: String,
    pub totals_json: String,
    pub lines_json: String,
    pub source_meta_json: String,
    pub fetched_at: String,
    pub is_deleted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

impl From<Model> for YmShowsSalesDaily {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: false,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());
        let header: YmShowsSalesDailyHeader =
            serde_json::from_str(&m.header_json).unwrap_or(YmShowsSalesDailyHeader {
                document_no: m.document_no.clone(),
                document_date: m.document_date.clone(),
                connection_id: m.connection_id.clone(),
                organization_id: m.organization_id.clone(),
                marketplace_id: m.marketplace_id.clone(),
                campaign_id: m.campaign_id.clone(),
            });
        let totals = serde_json::from_str(&m.totals_json).unwrap_or_default();
        let lines = serde_json::from_str(&m.lines_json).unwrap_or_default();
        let source_meta =
            serde_json::from_str(&m.source_meta_json).unwrap_or(YmShowsSalesDailySourceMeta {
                source: "ym_shows_sales".to_string(),
                fetched_at: m.fetched_at.clone(),
            });

        YmShowsSalesDaily {
            base: BaseAggregate::with_metadata(
                YmShowsSalesDailyId::new(uuid),
                m.code,
                m.description,
                m.comment,
                metadata,
            ),
            header,
            totals,
            lines,
            source_meta,
        }
    }
}

pub async fn get_by_id(id: Uuid) -> Result<Option<YmShowsSalesDaily>> {
    let result = Entity::find_by_id(id.to_string()).one(conn()).await?;
    Ok(result.map(Into::into))
}

/// `id` документов за период — для пересбора воронки (u508).
pub async fn list_ids_by_period(
    date_from: &str,
    date_to: &str,
    connection_mp_refs: &[String],
) -> Result<Vec<String>> {
    let mut query = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .filter(Column::DocumentDate.gte(date_from))
        .filter(Column::DocumentDate.lte(date_to));
    if !connection_mp_refs.is_empty() {
        query = query.filter(Column::ConnectionId.is_in(connection_mp_refs.to_vec()));
    }
    Ok(query
        .order_by_asc(Column::DocumentDate)
        .all(conn())
        .await?
        .into_iter()
        .map(|item| item.id)
        .collect())
}

/// Точечное перепроведение одного документа: пересобирает его движения p916.
/// Идемпотентно (delete-by-registrator + insert в одной транзакции).
pub async fn post_document(id: Uuid) -> Result<()> {
    let document = get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    use crate::projections::p916_mp_sales_funnel_turnovers::{
        builder as funnel_builder, repository as funnel_repo,
    };
    let registrator_ref = id.to_string();
    let rows = funnel_builder::from_ym_shows_sales_daily(&document, &registrator_ref);

    let db = get_connection();
    let txn = db.begin().await?;
    funnel_repo::delete_by_registrator_with_conn(&txn, funnel_builder::REG_A041, &registrator_ref)
        .await?;
    funnel_repo::insert_many_with_conn(&txn, &rows).await?;
    txn.commit().await?;
    Ok(())
}

/// Замена периода целиком: удаляем документы кабинета за `[date_from, date_to]`,
/// вставляем новые и в той же транзакции пересобираем маркетинговые движения p916.
/// Отчёт YM пересчитывается задним числом, поэтому окно всегда перезаливается, а не
/// дополняется.
pub async fn replace_for_period(
    connection_id: &str,
    date_from: &str,
    date_to: &str,
    documents: &[YmShowsSalesDaily],
) -> Result<usize> {
    let db = get_connection();
    let started_at = std::time::Instant::now();
    tracing::info!(
        "a041_ym_shows_sales_daily replace_for_period: connection={}, period={}..{}, documents={}",
        connection_id,
        date_from,
        date_to,
        documents.len()
    );
    let txn = db.begin().await?;

    Entity::delete_many()
        .filter(Column::ConnectionId.eq(connection_id))
        .filter(Column::DocumentDate.gte(date_from))
        .filter(Column::DocumentDate.lte(date_to))
        .exec(&txn)
        .await?;

    for document in documents {
        insert_with_conn(&txn, document).await?;
    }

    use crate::projections::p916_mp_sales_funnel_turnovers::{
        builder as funnel_builder, repository as funnel_repo,
    };
    funnel_repo::delete_marketing_for_period_with_conn(
        &txn,
        funnel_builder::REG_A041,
        connection_id,
        date_from,
        date_to,
    )
    .await?;
    for document in documents {
        let registrator_ref = document.base.id.value().to_string();
        let rows = funnel_builder::from_ym_shows_sales_daily(document, &registrator_ref);
        funnel_repo::insert_many_with_conn(&txn, &rows).await?;
    }

    txn.commit().await?;
    tracing::info!(
        "a041_ym_shows_sales_daily replace_for_period: committed connection={}, inserted={}, elapsed_ms={}",
        connection_id,
        documents.len(),
        started_at.elapsed().as_millis()
    );
    Ok(documents.len())
}

async fn insert_with_conn<C: ConnectionTrait>(db: &C, document: &YmShowsSalesDaily) -> Result<()> {
    let header_json = serde_json::to_string(&document.header)?;
    let totals_json = serde_json::to_string(&document.totals)?;
    let lines_json = serde_json::to_string(&document.lines)?;
    let source_meta_json = serde_json::to_string(&document.source_meta)?;

    let active_model = ActiveModel {
        id: Set(document.base.id.value().to_string()),
        code: Set(document.base.code.clone()),
        description: Set(document.base.description.clone()),
        comment: Set(document.base.comment.clone()),
        document_no: Set(document.header.document_no.clone()),
        document_date: Set(document.header.document_date.clone()),
        connection_id: Set(document.header.connection_id.clone()),
        organization_id: Set(document.header.organization_id.clone()),
        marketplace_id: Set(document.header.marketplace_id.clone()),
        campaign_id: Set(document.header.campaign_id.clone()),
        lines_count: Set(document.lines.len() as i32),
        total_shows: Set(document.totals.shows),
        total_clicks: Set(document.totals.clicks),
        total_to_cart: Set(document.totals.to_cart),
        total_order_items: Set(document.totals.order_items),
        total_delivered_count: Set(document.totals.delivered_count),
        total_canceled_count: Set(document.totals.canceled_count),
        total_returned_count: Set(document.totals.returned_count),
        header_json: Set(header_json),
        totals_json: Set(totals_json),
        lines_json: Set(lines_json),
        source_meta_json: Set(source_meta_json),
        fetched_at: Set(document.source_meta.fetched_at.clone()),
        is_deleted: Set(false),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        version: Set(1),
    };

    Entity::insert(active_model).exec(db).await?;
    Ok(())
}
