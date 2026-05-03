use super::manager::TaskManager;
use contracts::system::tasks::metadata::TaskMetadata;
use contracts::system::tasks::progress::TaskProgressResponse;
use contracts::system::tasks::runs::LiveMemoryProgressItem;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

static GLOBAL_REGISTRY: OnceLock<Arc<TaskManagerRegistry>> = OnceLock::new();

/// Регистрирует глобальный реестр (вызывается при инициализации).
pub fn set_global_registry(registry: Arc<TaskManagerRegistry>) {
    let _ = GLOBAL_REGISTRY.set(registry);
}

/// Возвращает ссылку на глобальный реестр.
pub fn get_global_registry() -> Option<&'static Arc<TaskManagerRegistry>> {
    GLOBAL_REGISTRY.get()
}

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

    /// Возвращает метаданные всех зарегистрированных типов задач.
    pub fn list_metadata(&self) -> Vec<&'static TaskMetadata> {
        self.managers.values().map(|m| m.metadata()).collect()
    }

    /// Снимок прогресса только из памяти (по всем менеджерам): без SQL и без чтения логов.
    pub fn snapshot_live_memory_progress(&self) -> Vec<LiveMemoryProgressItem> {
        let mut out = Vec::new();
        for m in self.managers.values() {
            let meta = m.metadata();
            let task_type = meta.task_type.to_string();
            let task_display_name = meta.display_name.to_string();
            for p in m.list_live_progress_sessions() {
                let progress: TaskProgressResponse = p.into();
                out.push(LiveMemoryProgressItem {
                    task_type: task_type.clone(),
                    task_display_name: task_display_name.clone(),
                    progress,
                });
            }
        }
        out
    }
}

impl Default for TaskManagerRegistry {
    fn default() -> Self {
        Self::new()
    }
}
