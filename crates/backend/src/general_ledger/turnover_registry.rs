//! Реестр видов оборотов General Ledger.
//! Re-export из shared/analytics для обратной совместимости.
pub use crate::shared::analytics::turnover_registry::{get_turnover_class, TURNOVER_CLASSES};
