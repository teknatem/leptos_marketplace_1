use anyhow::Result;
use async_trait::async_trait;
use contracts::system::sys_scheduled_task::aggregate::ScheduledTask;
use contracts::system::sys_scheduled_task::progress::TaskProgress;
use std::sync::Arc;

/// Трейт для менеджеров запланированных задач.
/// Каждый конкретный тип задачи должен иметь свою реализацию этого трейта.
#[async_trait]
pub trait TaskManager: Send + Sync {
    /// Возвращает тип задачи, который обрабатывает этот менеджер.
    fn task_type(&self) -> &'static str;

    /// Запускает выполнение задачи.
    /// `task`: Агрегат ScheduledTask, содержащий конфигурацию задачи.
    /// `session_id`: Уникальный идентификатор текущей сессии выполнения.
    /// `logger`: Логгер для записи прогресса и сообщений задачи.
    async fn run(
        &self,
        task: &ScheduledTask,
        session_id: &str,
        logger: Arc<super::logger::TaskLogger>,
    ) -> Result<()>;

    /// Получает текущий прогресс выполнения задачи по session_id.
    fn get_progress(&self, session_id: &str) -> Option<TaskProgress>;
}
