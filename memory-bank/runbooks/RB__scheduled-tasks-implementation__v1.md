---
type: runbook
version: 1.0
date: 2025-12-23
---
# Runbook: Implementing a New Scheduled Task

## Procedure
1. **Define Job Type**: Choose a unique string key (e.g., `u507_custom_sync`).
2. **Implement Manager**:
   - Create a new file in `crates/backend/src/system/sys_scheduled_task/managers/`.
   - Implement the `TaskManager` trait.
   - Wrap the relevant UseCase executor.
3. **Register Manager**:
   - Add the manager to `crates/backend/src/system/sys_scheduled_task/initialization.rs`.
   - Register it in `initialize_scheduled_tasks()`.
4. **Update Frontend UI (Optional)**:
   - Add the new job type to the dropdown in `crates/frontend/src/system/tasks/ui/details/mod.rs`.
5. **Test**:
   - Create a task in the UI with the new type.
   - Set a schedule or run manually (via DB update or temporary trigger).
   - Monitor logs in the UI.

## Verification
- Check `marketplace.db` table `sys_scheduled_tasks` for the new record.
- Check `./task_logs/` for the generated JSON log file.


