use chrono::Utc;
use contracts::system::tasks::aggregate::{
    ScheduledTask, ScheduledTaskId,
};
use contracts::domain::common::{BaseAggregate, EntityMetadata};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::data::db::get_connection;
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sys_tasks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub code: String,
    pub description: Option<String>,
    pub task_type: String,
    pub schedule_cron: Option<String>,
    pub config_json: Option<String>,
    pub is_enabled: bool,
    pub last_run_at: Option<chrono::DateTime<chrono::Utc>>,
    pub next_run_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_run_status: Option<String>,
    pub last_run_log_file: Option<String>,
    pub is_deleted: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for ScheduledTask {
    fn from(m: Model) -> Self {
        let metadata = EntityMetadata {
            created_at: m.created_at.unwrap_or_else(Utc::now),
            updated_at: m.updated_at.unwrap_or_else(Utc::now),
            is_deleted: m.is_deleted,
            is_posted: true,
            version: 1,
        };
        let uuid = Uuid::parse_str(&m.id).unwrap_or_else(|_| Uuid::new_v4());

        ScheduledTask {
            base: BaseAggregate::with_metadata(
                ScheduledTaskId(uuid),
                m.code,
                m.description.unwrap_or_default(),
                None, // comment
                metadata,
            ),
            task_type: m.task_type,
            schedule_cron: m.schedule_cron,
            config_json: m.config_json.unwrap_or_default(),
            is_enabled: m.is_enabled,
            last_run_at: m.last_run_at,
            next_run_at: m.next_run_at,
            last_run_status: m.last_run_status,
            last_run_log_file: m.last_run_log_file,
        }
    }
}

pub async fn list_all() -> Result<Vec<ScheduledTask>, DbErr> {
    let db = get_connection();
    let models = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .order_by_asc(Column::Code)
        .all(db)
        .await?;
    Ok(models.into_iter().map(ScheduledTask::from).collect())
}

pub async fn list_enabled() -> Result<Vec<ScheduledTask>, DbErr> {
    let db = get_connection();
    let models = Entity::find()
        .filter(Column::IsDeleted.eq(false))
        .filter(Column::IsEnabled.eq(true))
        .all(db)
        .await?;
    Ok(models.into_iter().map(ScheduledTask::from).collect())
}

pub async fn get_by_id(id: Uuid) -> Result<Option<ScheduledTask>, DbErr> {
    let db = get_connection();
    let model = Entity::find_by_id(id.to_string()).one(db).await?;
    Ok(model.map(ScheduledTask::from))
}

pub async fn get_by_code(code: &str) -> Result<Option<ScheduledTask>, DbErr> {
    let db = get_connection();
    let model = Entity::find()
        .filter(Column::Code.eq(code))
        .filter(Column::IsDeleted.eq(false))
        .one(db)
        .await?;
    Ok(model.map(ScheduledTask::from))
}

pub async fn save(task: &ScheduledTask) -> Result<(), DbErr> {
    let db = get_connection();
    let id_str = task.base.id.0.to_string();

    let existing = Entity::find_by_id(&id_str).one(db).await?;

    let mut active: ActiveModel = match existing {
        Some(ref m) => m.clone().into(),
        None => {
            let am = ActiveModel {
                id: Set(id_str),
                created_at: Set(Some(task.base.metadata.created_at)),
                ..Default::default()
            };
            am
        }
    };

    active.code = Set(task.base.code.clone());
    active.description = Set(Some(task.base.description.clone()));
    active.task_type = Set(task.task_type.clone());
    active.schedule_cron = Set(task.schedule_cron.clone());
    active.config_json = Set(Some(task.config_json.clone()));
    active.is_enabled = Set(task.is_enabled);
    active.last_run_at = Set(task.last_run_at);
    active.next_run_at = Set(task.next_run_at);
    active.last_run_status = Set(task.last_run_status.clone());
    active.last_run_log_file = Set(task.last_run_log_file.clone());
    active.is_deleted = Set(task.base.metadata.is_deleted);
    active.updated_at = Set(Some(Utc::now()));

    if existing.is_some() {
        active.update(db).await?;
    } else {
        active.insert(db).await?;
    }

    Ok(())
}

pub async fn soft_delete(id: Uuid) -> Result<(), DbErr> {
    let db = get_connection();
    Entity::update_many()
        .col_expr(Column::IsDeleted, Expr::value(true))
        .col_expr(Column::UpdatedAt, Expr::value(Utc::now()))
        .filter(Column::Id.eq(id.to_string()))
        .exec(db)
        .await?;
    Ok(())
}
