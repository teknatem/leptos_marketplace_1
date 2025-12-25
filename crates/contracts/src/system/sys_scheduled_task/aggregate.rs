use crate::domain::common::{
    AggregateId, AggregateRoot, BaseAggregate, EntityMetadata, EventStore, Origin,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// ID Type
// ============================================================================

/// Уникальный идентификатор регламентного задания
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScheduledTaskId(pub Uuid);

impl ScheduledTaskId {
    pub fn new(value: Uuid) -> Self {
        Self(value)
    }

    pub fn new_v4() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn value(&self) -> Uuid {
        self.0
    }
}

impl AggregateId for ScheduledTaskId {
    fn as_string(&self) -> String {
        self.0.to_string()
    }

    fn from_string(s: &str) -> Result<Self, String> {
        Uuid::parse_str(s)
            .map(ScheduledTaskId::new)
            .map_err(|e| format!("Invalid UUID: {}", e))
    }
}

// ============================================================================
// Aggregate Root
// ============================================================================

/// Регламентное задание (Scheduled Task)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    #[serde(flatten)]
    pub base: BaseAggregate<ScheduledTaskId>,

    /// Тип задания (executor key)
    pub task_type: String,

    /// Расписание (cron или интервал в секундах)
    pub schedule_cron: Option<String>,

    /// Параметры в формате JSON
    pub config_json: String,

    /// Флаг активности
    pub is_enabled: bool,

    /// Дата последнего запуска
    pub last_run_at: Option<DateTime<Utc>>,

    /// Дата следующего запуска
    pub next_run_at: Option<DateTime<Utc>>,

    /// Статус последнего выполнения
    pub last_run_status: Option<String>,

    /// Путь к лог-файлу последнего запуска
    pub last_run_log_file: Option<String>,
}

impl ScheduledTask {
    pub fn new_for_insert(
        code: String,
        description: String,
        task_type: String,
        schedule_cron: Option<String>,
        is_enabled: bool,
        config_json: String,
    ) -> Self {
        let id = ScheduledTaskId::new_v4();

        Self {
            base: BaseAggregate::new(id, code, description),
            task_type,
            schedule_cron,
            config_json,
            is_enabled,
            last_run_at: None,
            next_run_at: None,
            last_run_status: None,
            last_run_log_file: None,
        }
    }
}

impl AggregateRoot for ScheduledTask {
    type Id = ScheduledTaskId;

    fn id(&self) -> Self::Id {
        self.base.id
    }

    fn code(&self) -> &str {
        &self.base.code
    }

    fn description(&self) -> &str {
        &self.base.description
    }

    fn metadata(&self) -> &EntityMetadata {
        &self.base.metadata
    }

    fn metadata_mut(&mut self) -> &mut EntityMetadata {
        &mut self.base.metadata
    }

    fn events(&self) -> &EventStore {
        &self.base.events
    }

    fn events_mut(&mut self) -> &mut EventStore {
        &mut self.base.events
    }

    fn aggregate_index() -> &'static str {
        "sys_scheduled_task"
    }

    fn collection_name() -> &'static str {
        "sys_scheduled_tasks"
    }

    fn element_name() -> &'static str {
        "Регламентное задание"
    }

    fn list_name() -> &'static str {
        "Регламентные задания"
    }

    fn origin() -> Origin {
        Origin::Self_
    }
}
