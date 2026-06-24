//! General Ledger — контракты (типы, DTO, счета, виды оборотов).
//!
//! Самостоятельный слой учёта: таблица sys_general_ledger, план счетов,
//! реестр видов оборотов, DTO для API.

pub mod account_view;
pub mod accounting;
pub mod dto;
pub mod entity;
pub mod layer;
pub mod metadata;
pub mod report;
pub mod resource_detail;
pub mod supplier_balance;
pub mod turnover;
pub mod weekly_reconciliation;
pub mod ym_revenue_reconciliation;

pub use account_view::{GlAccountViewQuery, GlAccountViewResponse, GlAccountViewRow};
pub use accounting::{AccountDef, AccountType, NormalBalance, StatementSection};
pub use dto::{GeneralLedgerEntryDto, GeneralLedgerTurnoverDto};
pub use entity::{
    get_entity_class, GlEntitiesResponse, GlEntity, GlEntityClassDef, GlEntityDto,
    GL_ENTITY_CLASSES,
};
pub use layer::{get_layer_class, GlLayerClassDef, GlLayerDto, GlLayersResponse, GL_LAYER_CLASSES};
pub use metadata::{ENTITY_METADATA, FIELDS};
pub use report::{
    AggregateRepresentation, GlDimensionCatalogItem, GlDimensionDef, GlDimensionUsageRef,
    GlDimensionsCatalogResponse, GlDimensionsResponse, GlDrilldownQuery, GlDrilldownResponse,
    GlDrilldownRow, GlDrilldownSessionCreate, GlDrilldownSessionCreateResponse,
    GlDrilldownSessionRecord, GlLayerTurnoverMatrixResponse, GlMatrixCell, GlMatrixDimension,
    GlMatrixLayer, GlMatrixProjection, GlMatrixTurnover, GlReportQuery, GlReportResponse,
    GlReportRow,
};
pub use resource_detail::{GlResourceDetailResponse, GlResourceDetailTotals};
pub use supplier_balance::{SupplierBalanceQuery, SupplierBalanceResponse};
pub use turnover::{
    AggKind, AmountColumn, DateSource, EventKind, KeySource, ReportGroup, SelectionRule,
    SignPolicy, SourceRefStrategy, TargetProjection, TurnoverClassDef, TurnoverLayer,
    TurnoverMappingRule, TurnoverScope, ValueKind,
};
pub use weekly_reconciliation::{
    WbWeeklyReconciliationQuery, WbWeeklyReconciliationResponse, WbWeeklyReconciliationRow,
};
pub use ym_revenue_reconciliation::{
    YmRevenueReconGroup, YmRevenueReconQuery, YmRevenueReconResponse, YmRevenueReconRow,
};
