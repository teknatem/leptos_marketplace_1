use anyhow::Result;
use async_trait::async_trait;
use chrono::NaiveDate;
use contracts::system::tasks::aggregate::ScheduledTask;
use contracts::system::tasks::metadata::TaskMetadata;
use contracts::system::tasks::progress::{TaskProgress, TaskStatus};
use std::sync::Arc;

/// Итог выполнения `TaskManager::run` для планировщика и watermark.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskRunOutcome {
    pub status: TaskStatus,
    pub loaded_to: Option<NaiveDate>,
    pub move_watermark: bool,
}

impl TaskRunOutcome {
    pub fn completed() -> Self {
        Self {
            status: TaskStatus::Completed,
            loaded_to: None,
            move_watermark: true,
        }
    }

    pub fn completed_loaded_to(loaded_to: NaiveDate) -> Self {
        Self {
            status: TaskStatus::Completed,
            loaded_to: Some(loaded_to),
            move_watermark: true,
        }
    }

    pub fn completed_with_errors() -> Self {
        Self {
            status: TaskStatus::CompletedWithErrors,
            loaded_to: None,
            move_watermark: false,
        }
    }

    /// Сдвигать `last_successful_run_at` только при полном успехе.
    pub fn advances_watermark(&self) -> bool {
        self.move_watermark && matches!(self.status, TaskStatus::Completed)
    }
}

/// Трейт для менеджеров запланированных задач.
/// Каждый конкретный тип задачи должен иметь свою реализацию этого трейта.
#[async_trait]
pub trait TaskManager: Send + Sync {
    /// Возвращает тип задачи, который обрабатывает этот менеджер.
    fn task_type(&self) -> &'static str;

    /// Возвращает статические метаданные задачи (описание, API, ограничения).
    fn metadata(&self) -> &'static TaskMetadata;

    /// Запускает выполнение задачи.
    /// `task`: Агрегат ScheduledTask, содержащий конфигурацию задачи.
    /// `session_id`: Уникальный идентификатор текущей сессии выполнения.
    /// `logger`: Логгер для записи прогресса и сообщений задачи.
    async fn run(
        &self,
        task: &ScheduledTask,
        session_id: &str,
        logger: Arc<super::logger::TaskLogger>,
    ) -> Result<TaskRunOutcome>;

    /// Получает текущий прогресс выполнения задачи по session_id.
    fn get_progress(&self, session_id: &str) -> Option<TaskProgress>;

    /// Все сессии со статусом `Running` в памяти этого менеджера (без БД и без диска).
    fn list_live_progress_sessions(&self) -> Vec<TaskProgress> {
        Vec::new()
    }
}
