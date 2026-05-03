use anyhow::Result;
use chrono::Utc;
use contracts::domain::a030_wb_advert_campaign::aggregate::{
    count_nm_positions, WbAdvertCampaign, WbAdvertCampaignHeader, WbAdvertCampaignId,
    WbAdvertCampaignSourceMeta,
};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set, Statement};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::shared::data::db::get_connection;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a030_wb_advert_campaign")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub advert_id: i64,
    pub connection_id: String,
    pub organization_id: String,
    pub marketplace_id: String,
    pub campaign_type: Option<i32>,
    pub status: Option<i32>,
    pub change_time: Option<String>,
    pub nm_count: i32,
    pub info_json: String,
    pub source_meta_json: String,
    pub is_deleted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for WbAdvertCampaign {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: false,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());
        let info_json = serde_json::from_str(&m.info_json).unwrap_or(serde_json::Value::Null);
        let source_meta =
            serde_json::from_str(&m.source_meta_json).unwrap_or(WbAdvertCampaignSourceMeta {
                source: "wb_advert_campaigns".to_string(),
                fetched_at: Utc::now().to_rfc3339(),
                info_json,
            });
        let nm_count = if m.nm_count > 0 {
            m.nm_count
        } else {
            count_nm_positions(&source_meta.info_json)
        };

        WbAdvertCampaign {
            base: BaseAggregate::with_metadata(
                WbAdvertCampaignId::new(uuid),
                m.code,
                m.description,
                m.comment,
                metadata,
            ),
            header: WbAdvertCampaignHeader {
                advert_id: m.advert_id,
                connection_id: m.connection_id,
                organization_id: m.organization_id,
                marketplace_id: m.marketplace_id,
                campaign_type: m.campaign_type,
                status: m.status,
                change_time: m.change_time,
                nm_count,
            },
            source_meta,
        }
    }
}

/// Returns `(new_count, total_count)` where `new_count` is the number of records
/// physically inserted for the first time (vs. updated existing ones).
pub async fn upsert_many(items: &[WbAdvertCampaign]) -> Result<(usize, usize)> {
    let mut new_count = 0usize;
    for item in items {
        if upsert(item).await? {
            new_count += 1;
        }
    }
    Ok((new_count, items.len()))
}

/// Returns `true` if the record was physically inserted (new), `false` if updated.
pub async fn upsert(item: &WbAdvertCampaign) -> Result<bool> {
    let db = get_connection();
    let existing = Entity::find()
        .filter(Column::ConnectionId.eq(item.header.connection_id.clone()))
        .filter(Column::AdvertId.eq(item.header.advert_id))
        .filter(Column::IsDeleted.eq(false))
        .one(db)
        .await?;

    let id = existing
        .as_ref()
        .map(|m| m.id.clone())
        .unwrap_or_else(|| item.base.id.value().to_string());
    let created_at = existing
        .as_ref()
        .and_then(|m| m.created_at)
        .or(Some(Utc::now()));
    let version = existing.as_ref().map(|m| m.version + 1).unwrap_or(1);

    // Preserve existing info_json when the incoming value is null — this allows
    // the incremental sync to skip rate-limited API calls for unchanged campaigns
    // without losing previously fetched info.
    let info_json = if item.source_meta.info_json.is_null() {
        if let Some(ref ex) = existing {
            if ex.info_json != "null" && !ex.info_json.is_empty() {
                ex.info_json.clone()
            } else {
                "null".to_string()
            }
        } else {
            "null".to_string()
        }
    } else {
        serde_json::to_string(&item.source_meta.info_json)?
    };

    // Rebuild source_meta_json with the effective info_json so both columns are consistent.
    let effective_info: serde_json::Value =
        serde_json::from_str(&info_json).unwrap_or(serde_json::Value::Null);
    let effective_nm_count = {
        let count = count_nm_positions(&effective_info);
        if count > 0 {
            count
        } else {
            item.header.nm_count
        }
    };
    let effective_meta =
        contracts::domain::a030_wb_advert_campaign::aggregate::WbAdvertCampaignSourceMeta {
            source: item.source_meta.source.clone(),
            fetched_at: item.source_meta.fetched_at.clone(),
            info_json: effective_info,
        };
    let source_meta_json = serde_json::to_string(&effective_meta)?;

    let active = ActiveModel {
        id: Set(id),
        code: Set(item.base.code.clone()),
        description: Set(item.base.description.clone()),
        comment: Set(item.base.comment.clone()),
        advert_id: Set(item.header.advert_id),
        connection_id: Set(item.header.connection_id.clone()),
        organization_id: Set(item.header.organization_id.clone()),
        marketplace_id: Set(item.header.marketplace_id.clone()),
        campaign_type: Set(item.header.campaign_type),
        status: Set(item.header.status),
        change_time: Set(item.header.change_time.clone()),
        nm_count: Set(effective_nm_count),
        info_json: Set(info_json),
        source_meta_json: Set(source_meta_json),
        is_deleted: Set(false),
        created_at: Set(created_at),
        updated_at: Set(Some(Utc::now())),
        version: Set(version),
    };

    let was_inserted = existing.is_none();
    if existing.is_some() {
        active.update(db).await?;
    } else {
        active.insert(db).await?;
    }

    Ok(was_inserted)
}

pub async fn list_by_connection(connection_id: &str) -> Result<Vec<WbAdvertCampaign>> {
    let db = get_connection();
    let rows = Entity::find()
        .filter(Column::ConnectionId.eq(connection_id))
        .filter(Column::IsDeleted.eq(false))
        .all(db)
        .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn list_all() -> Result<Vec<WbAdvertCampaign>> {
    let db = get_connection();
    let rows = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .all(db)
        .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn get_by_id(id: Uuid) -> Result<Option<WbAdvertCampaign>> {
    let db = get_connection();
    let model = Entity::find_by_id(id.to_string())
        .filter(Column::IsDeleted.eq(false))
        .one(db)
        .await?;
    Ok(model.map(Into::into))
}

pub async fn list_advert_ids_by_connection(connection_id: &str) -> Result<Vec<i64>> {
    let mut ids: Vec<i64> = list_by_connection(connection_id)
        .await?
        .into_iter()
        .map(|item| item.header.advert_id)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

/// Returns advert_ids relevant for a given period, skipping completed campaigns
/// (status = 7) whose last activity (change_time) is before date_from.
/// Campaigns without status or change_time are included for safety.
pub async fn list_advert_ids_for_period(connection_id: &str, date_from: &str) -> Result<Vec<i64>> {
    let db = get_connection();
    // Completed (status=7) campaigns whose change_time is earlier than date_from
    // cannot have any activity in [date_from, date_to], so skip them.
    let sql = format!(
        "SELECT DISTINCT advert_id \
         FROM a030_wb_advert_campaign \
         WHERE connection_id = '{}' \
           AND is_deleted = 0 \
           AND (status IS NULL OR status != 7 OR change_time IS NULL OR change_time >= '{}') \
         ORDER BY advert_id ASC",
        connection_id.replace('\'', "''"),
        date_from,
    );
    let rows = db
        .query_all(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            sql,
        ))
        .await?;
    let ids: Vec<i64> = rows
        .iter()
        .filter_map(|row| row.try_get::<i64>("", "advert_id").ok())
        .collect();
    Ok(ids)
}

/// Lightweight snapshot: advert_id → (change_time, has_info_json) для заданного подключения.
/// Используется для инкрементальной синхронизации info_json без загрузки полных агрегатов.
pub async fn list_info_snapshot(connection_id: &str) -> Result<HashMap<i64, CampaignInfoSnapshot>> {
    let db = get_connection();
    let rows = Entity::find()
        .filter(Column::ConnectionId.eq(connection_id))
        .filter(Column::IsDeleted.eq(false))
        .column(Column::AdvertId)
        .column(Column::ChangeTime)
        .column(Column::InfoJson)
        .all(db)
        .await?;

    let map = rows
        .into_iter()
        .map(|m| {
            let has_info = m.info_json != "null" && !m.info_json.is_empty();
            (
                m.advert_id,
                CampaignInfoSnapshot {
                    change_time: m.change_time,
                    has_info_json: has_info,
                },
            )
        })
        .collect();
    Ok(map)
}

#[derive(Debug, Clone)]
pub struct CampaignInfoSnapshot {
    pub change_time: Option<String>,
    pub has_info_json: bool,
}
