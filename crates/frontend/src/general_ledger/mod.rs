//! General Ledger — фронтенд-слой.
//!
//! API-клиент, типы запросов/ответов и UI-страницы для Главной книги.
//! Самостоятельный модуль, не зависит от domain/.

pub mod api;
pub mod types;
pub mod ui;

pub use ui::{GeneralLedgerDetailsPage, GeneralLedgerPage, GeneralLedgerTurnoversPage};
