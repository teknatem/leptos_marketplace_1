use chrono::Utc;
use contracts::domain::a039_mail_message::aggregate::{MailMessage, MailMessageId};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::data::db::get_connection;
use sea_orm::entity::prelude::*;
use sea_orm::{
    ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "a039_mail_message")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub direction: String,
    pub imap_uid: Option<i64>,
    pub message_id_hdr: Option<String>,
    pub in_reply_to_ref: Option<String>,
    pub from_addr: String,
    pub to_addr: String,
    pub subject: String,
    pub body_excerpt: String,
    pub user_ref: Option<String>,
    pub intent: Option<String>,
    pub agent_type: Option<String>,
    pub chat_ref: Option<String>,
    pub message_ref: Option<String>,
    pub artifact_ref: Option<String>,
    pub status: String,
    pub error: Option<String>,
    pub due_at: Option<String>,
    pub is_deleted: bool,
    pub is_posted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for MailMessage {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: m.is_posted,
            version: m.version,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());

        MailMessage {
            base: BaseAggregate::with_metadata(
                MailMessageId(uuid),
                m.code,
                m.description,
                m.comment.clone(),
                metadata,
            ),
            direction: m.direction,
            imap_uid: m.imap_uid,
            message_id_hdr: m.message_id_hdr,
            in_reply_to_ref: m.in_reply_to_ref,
            from_addr: m.from_addr,
            to_addr: m.to_addr,
            subject: m.subject,
            body_excerpt: m.body_excerpt,
            user_ref: m.user_ref,
            intent: m.intent,
            agent_type: m.agent_type,
            chat_ref: m.chat_ref,
            message_ref: m.message_ref,
            artifact_ref: m.artifact_ref,
            status: m.status,
            error: m.error,
            due_at: m.due_at,
        }
    }
}

fn conn() -> &'static DatabaseConnection {
    get_connection()
}

fn to_active(item: &MailMessage, is_insert: bool) -> ActiveModel {
    let now = Utc::now();
    ActiveModel {
        id: Set(item.to_string_id()),
        code: Set(item.base.code.clone()),
        description: Set(item.base.description.clone()),
        comment: Set(item.base.comment.clone()),
        direction: Set(item.direction.clone()),
        imap_uid: Set(item.imap_uid),
        message_id_hdr: Set(item.message_id_hdr.clone()),
        in_reply_to_ref: Set(item.in_reply_to_ref.clone()),
        from_addr: Set(item.from_addr.clone()),
        to_addr: Set(item.to_addr.clone()),
        subject: Set(item.subject.clone()),
        body_excerpt: Set(item.body_excerpt.clone()),
        user_ref: Set(item.user_ref.clone()),
        intent: Set(item.intent.clone()),
        agent_type: Set(item.agent_type.clone()),
        chat_ref: Set(item.chat_ref.clone()),
        message_ref: Set(item.message_ref.clone()),
        artifact_ref: Set(item.artifact_ref.clone()),
        status: Set(item.status.clone()),
        error: Set(item.error.clone()),
        due_at: Set(item.due_at.clone()),
        is_deleted: Set(item.base.metadata.is_deleted),
        is_posted: Set(false),
        created_at: Set(Some(if is_insert {
            now
        } else {
            item.base.metadata.created_at
        })),
        updated_at: Set(Some(now)),
        version: Set(if is_insert {
            1
        } else {
            item.base.metadata.version + 1
        }),
    }
}

pub async fn list_all() -> anyhow::Result<Vec<MailMessage>> {
    let items: Vec<MailMessage> = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .order_by_desc(Column::CreatedAt)
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(items)
}

pub async fn list_paginated(
    limit: u64,
    offset: u64,
    sort_by: &str,
    sort_desc: bool,
) -> anyhow::Result<(Vec<MailMessage>, u64)> {
    let total = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .count(conn())
        .await?;

    let mut query = Entity::find().filter(Column::IsDeleted.eq(false));
    query = match sort_by {
        "status" => {
            if sort_desc {
                query.order_by_desc(Column::Status)
            } else {
                query.order_by_asc(Column::Status)
            }
        }
        "direction" => {
            if sort_desc {
                query.order_by_desc(Column::Direction)
            } else {
                query.order_by_asc(Column::Direction)
            }
        }
        _ => {
            if sort_desc {
                query.order_by_desc(Column::CreatedAt)
            } else {
                query.order_by_asc(Column::CreatedAt)
            }
        }
    };

    let items: Vec<MailMessage> = query
        .offset(offset)
        .limit(limit)
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok((items, total))
}

pub async fn find_by_id(id: &str) -> anyhow::Result<Option<MailMessage>> {
    let model = Entity::find_by_id(id.to_string())
        .filter(Column::IsDeleted.eq(false))
        .one(conn())
        .await?;
    Ok(model.map(Into::into))
}

/// Найти входящую запись по IMAP UID (дедуп поллинга).
pub async fn find_inbound_by_uid(uid: i64) -> anyhow::Result<Option<MailMessage>> {
    let model = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .filter(Column::Direction.eq("inbound"))
        .filter(Column::ImapUid.eq(uid))
        .one(conn())
        .await?;
    Ok(model.map(Into::into))
}

/// Входящие письма с одним из указанных статусов (для регламента ответов).
pub async fn list_inbound_by_statuses(statuses: &[&str]) -> anyhow::Result<Vec<MailMessage>> {
    let owned: Vec<String> = statuses.iter().map(|s| s.to_string()).collect();
    let items: Vec<MailMessage> = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .filter(Column::Direction.eq("inbound"))
        .filter(Column::Status.is_in(owned))
        .order_by_asc(Column::CreatedAt)
        .all(conn())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(items)
}

pub async fn insert(item: &MailMessage) -> anyhow::Result<()> {
    Entity::insert(to_active(item, true)).exec(conn()).await?;
    Ok(())
}

pub async fn update(item: &MailMessage) -> anyhow::Result<()> {
    Entity::update(to_active(item, false)).exec(conn()).await?;
    Ok(())
}

pub async fn soft_delete(id: &str) -> anyhow::Result<()> {
    let now = Utc::now();
    Entity::update_many()
        .col_expr(Column::IsDeleted, Expr::value(true))
        .col_expr(Column::UpdatedAt, Expr::value(Some(now)))
        .filter(Column::Id.eq(id))
        .exec(conn())
        .await?;
    Ok(())
}
