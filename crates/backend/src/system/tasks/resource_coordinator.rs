use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

static GLOBAL_COORDINATOR: LazyLock<Arc<TaskResourceCoordinator>> =
    LazyLock::new(|| Arc::new(TaskResourceCoordinator::default()));

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceConflict {
    pub resource: String,
    pub owner_session_id: String,
    pub owner_task: String,
}

#[derive(Debug, Clone)]
struct ResourceOwner {
    session_id: String,
    task: String,
}

#[derive(Debug, Default)]
pub struct TaskResourceCoordinator {
    owners: Arc<Mutex<HashMap<String, ResourceOwner>>>,
}

/// Guard for all resources used by one task run. Dropping the run future (including abort)
/// drops this guard and releases every resource atomically acquired for the session.
#[derive(Debug)]
pub struct TaskResourceGuard {
    owners: Arc<Mutex<HashMap<String, ResourceOwner>>>,
    resource_keys: Vec<String>,
    session_id: String,
}

impl TaskResourceCoordinator {
    pub fn try_acquire(
        &self,
        task_id: &str,
        task_label: &str,
        session_id: &str,
        write_tables: &[&str],
    ) -> Result<TaskResourceGuard, ResourceConflict> {
        // The synthetic task resource closes the race between scheduled and manual starts of
        // the same task, including tasks which do not write database tables.
        let mut resources = vec![(format!("task:{task_id}"), format!("task {task_label}"))];
        resources.extend(
            write_tables
                .iter()
                .map(|table| (format!("table:{table}"), (*table).to_string())),
        );
        resources.sort_by(|a, b| a.0.cmp(&b.0));
        resources.dedup_by(|a, b| a.0 == b.0);

        let mut owners = self.owners.lock().unwrap_or_else(|e| e.into_inner());
        for (key, display_name) in &resources {
            if let Some(owner) = owners.get(key) {
                return Err(ResourceConflict {
                    resource: display_name.clone(),
                    owner_session_id: owner.session_id.clone(),
                    owner_task: owner.task.clone(),
                });
            }
        }

        for (key, _) in &resources {
            owners.insert(
                key.clone(),
                ResourceOwner {
                    session_id: session_id.to_string(),
                    task: task_label.to_string(),
                },
            );
        }

        Ok(TaskResourceGuard {
            owners: Arc::clone(&self.owners),
            resource_keys: resources.into_iter().map(|(key, _)| key).collect(),
            session_id: session_id.to_string(),
        })
    }
}

impl Drop for TaskResourceGuard {
    fn drop(&mut self) {
        let mut owners = self.owners.lock().unwrap_or_else(|e| e.into_inner());
        for key in &self.resource_keys {
            if owners
                .get(key)
                .is_some_and(|owner| owner.session_id == self.session_id)
            {
                owners.remove(key);
            }
        }
    }
}

pub fn get_global_resource_coordinator() -> Arc<TaskResourceCoordinator> {
    Arc::clone(&GLOBAL_COORDINATOR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlapping_tables_are_exclusive_and_release_on_drop() {
        let coordinator = TaskResourceCoordinator::default();
        let first = coordinator
            .try_acquire("task-1", "first", "session-1", &["table_a", "table_b"])
            .unwrap();

        let conflict = coordinator
            .try_acquire("task-2", "second", "session-2", &["table_b"])
            .unwrap_err();
        assert_eq!(conflict.resource, "table_b");
        assert_eq!(conflict.owner_session_id, "session-1");

        drop(first);
        coordinator
            .try_acquire("task-2", "second", "session-2", &["table_b"])
            .unwrap();
    }

    #[test]
    fn disjoint_tables_can_run_in_parallel() {
        let coordinator = TaskResourceCoordinator::default();
        let _first = coordinator
            .try_acquire("task-1", "first", "session-1", &["table_a"])
            .unwrap();
        let _second = coordinator
            .try_acquire("task-2", "second", "session-2", &["table_b"])
            .unwrap();
    }

    #[test]
    fn same_task_is_exclusive_even_without_tables() {
        let coordinator = TaskResourceCoordinator::default();
        let _first = coordinator
            .try_acquire("task-1", "first", "session-1", &[])
            .unwrap();
        let conflict = coordinator
            .try_acquire("task-1", "first", "session-2", &[])
            .unwrap_err();
        assert_eq!(conflict.resource, "task first");
    }
}
