---
type: decision
status: accepted
date: 2025-12-23
---
# ADR0003: Scheduled Tasks Architecture

## Context
The project requires automated data synchronization from multiple marketplaces (Ozon, WB, Yandex) and 1C:UT11. This needs to run in the background with real-time monitoring in the UI.

## Decision
1. **Vertical Slice Architecture**: Implement as a system feature (`sys_scheduled_task`) spanning contracts, backend, and frontend.
2. **Registry Pattern**: Use a `TaskRegistry` to map `job_type` (string) to `TaskManager` implementations.
3. **File-Based Logging**: Write execution progress and logs to JSON files (one per task/session) in a dedicated directory instead of the database.
4. **Tokio Background Worker**: Run a periodic loop in the backend to check and spawn tasks.
5. **UseCase Wrapping**: Scheduled tasks should wrap existing UseCase executors rather than duplicating logic.

## Rationale
- **DB Load**: Moving logs out of the DB avoids excessive ORM operations and database growth.
- **Flexibility**: The registry allows adding new task types without modifying the core worker.
- **Real-time**: JSON files allow the frontend to poll for progress independently of DB transactions.


