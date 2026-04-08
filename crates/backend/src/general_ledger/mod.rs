//! General Ledger — самостоятельный учётный слой.
//!
//! Содержит всё необходимое для работы с главной книгой:
//! - таблица sys_general_ledger (repository, service)
//! - реестр видов оборотов (turnover_registry)
//! - план счетов (account_registry)

pub mod account_registry;
pub mod account_view;
pub mod drilldown_dimensions;
pub mod drilldown_session_repository;
pub mod report_repository;
pub mod repository;
pub mod service;
pub mod turnover_registry;
pub mod weekly_reconciliation;

pub use account_registry::{get_account, ACCOUNT_REGISTRY};
pub use turnover_registry::{get_turnover_class, TURNOVER_CLASSES};
