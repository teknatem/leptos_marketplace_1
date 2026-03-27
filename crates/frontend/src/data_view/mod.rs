//! DataView — Семантический слой (frontend)
//!
//! Типы, API-клиент и UI-страницы для каталога DataView.

pub mod api;
pub mod types;
pub mod ui;

pub use types::{
    DataViewMeta, DimensionMeta, FilterDef, FilterKind, FilterRef, GlobalFiltersResponse,
    ResourceMeta, SelectOption, ViewFiltersResponse,
};
