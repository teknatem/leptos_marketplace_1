//! General Ledger: re-export из projections для обратной совместимости.
//! Реальная реализация временно остаётся в projections/general_ledger/repository.rs
//! и будет перенесена сюда в рамках cleanup.
pub use crate::projections::general_ledger::repository::*;
