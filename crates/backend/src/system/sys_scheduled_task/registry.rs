use super::manager::TaskManager;
use std::collections::HashMap;
use std::sync::Arc;

/// Реестр менеджеров задач.
/// Позволяет регистрировать различные реализации TaskManager и получать их по типу задачи.
pub struct TaskManagerRegistry {
    managers: HashMap<String, Arc<dyn TaskManager>>,
}

impl TaskManagerRegistry {
    pub fn new() -> Self {
        Self {
            managers: HashMap::new(),
        }
    }

    /// Регистрирует менеджер задач.
    pub fn register<T: TaskManager + 'static>(&mut self, manager: T) {
        let task_type = manager.task_type().to_string();
        self.managers.insert(task_type, Arc::new(manager));
    }

    /// Возвращает менеджер задач по его типу.
    pub fn get(&self, task_type: &str) -> Option<Arc<dyn TaskManager>> {
        self.managers.get(task_type).cloned()
    }
}

impl Default for TaskManagerRegistry {
    fn default() -> Self {
        Self::new()
    }
}
