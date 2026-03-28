//! General Ledger — контракты (типы, DTO, счета, виды оборотов).
//!
//! Самостоятельный слой учёта: таблица sys_general_ledger, план счетов,
//! реестр видов оборотов, DTO для API.

pub mod accounting;
pub mod dto;
pub mod metadata;
pub mod turnover;

pub use accounting::{AccountDef, AccountType, NormalBalance, StatementSection};
pub use dto::{GeneralLedgerEntryDto, GeneralLedgerTurnoverDto};
pub use metadata::{ENTITY_METADATA, FIELDS};
pub use turnover::{
    AggKind, AmountColumn, DateSource, EventKind, KeySource, ReportGroup, SelectionRule,
    SignPolicy, SourceRefStrategy, TargetProjection, TurnoverClassDef, TurnoverLayer,
    TurnoverMappingRule, TurnoverScope, ValueKind,
};
