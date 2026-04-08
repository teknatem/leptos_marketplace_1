//! General Ledger — контракты (типы, DTO, счета, виды оборотов).
//!
//! Самостоятельный слой учёта: таблица sys_general_ledger, план счетов,
//! реестр видов оборотов, DTO для API.

pub mod account_view;
pub mod accounting;
pub mod dto;
pub mod metadata;
pub mod report;
pub mod turnover;
pub mod weekly_reconciliation;

pub use account_view::{GlAccountViewQuery, GlAccountViewResponse, GlAccountViewRow};
pub use accounting::{AccountDef, AccountType, NormalBalance, StatementSection};
pub use dto::{GeneralLedgerEntryDto, GeneralLedgerTurnoverDto};
pub use metadata::{ENTITY_METADATA, FIELDS};
pub use report::{
    GlDimensionDef, GlDimensionsResponse, GlDrilldownQuery, GlDrilldownResponse, GlDrilldownRow,
    GlDrilldownSessionCreate, GlDrilldownSessionCreateResponse, GlDrilldownSessionRecord,
    GlReportQuery, GlReportResponse, GlReportRow,
};
pub use turnover::{
    AggKind, AmountColumn, DateSource, EventKind, KeySource, ReportGroup, SelectionRule,
    SignPolicy, SourceRefStrategy, TargetProjection, TurnoverClassDef, TurnoverLayer,
    TurnoverMappingRule, TurnoverScope, ValueKind,
};
pub use weekly_reconciliation::{
    WbWeeklyReconciliationQuery, WbWeeklyReconciliationResponse, WbWeeklyReconciliationRow,
};
