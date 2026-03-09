//! BI Dashboard aggregate contracts (a025)
//!
//! Дашборды BI. Каждый дашборд — именованная коллекция BI-индикаторов,
//! сгруппированных по категориям в дерево, с глобальными фильтрами и оценкой.

pub mod aggregate;
pub mod metadata_gen;

pub use aggregate::{
    BiDashboard, BiDashboardId, BiDashboardListItem, BiDashboardStatus, DashboardGroup,
    DashboardItem, DashboardLayout,
};
pub use metadata_gen::{ENTITY_METADATA, FIELDS};
