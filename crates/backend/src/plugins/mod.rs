//! Подсистема **Plugins** (backend) — надстройка над платформой.
//!
//! Отдельная верхнеуровневая ветка (сиблинг к `domain/`, `dashboards/` …),
//! следующая базовым конвенциям агрегатов: `repository` (SeaORM-доступ к таблице
//! `plugin`), `service` (CRUD + валидация + тестовые данные).
//!
//! Фаза 1 — декларативные плагины (без Rhai). Движок (`engine/`), host-API
//! (`host_api`), `representation` и `change_token` добавляются в следующих фазах.

pub mod engine;
pub mod repository;
pub mod service;
