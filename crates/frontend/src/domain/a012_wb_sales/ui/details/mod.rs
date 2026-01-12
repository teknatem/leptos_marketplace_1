//! WB Sales Details UI Module (MVVM Standard)
//!
//! Structure:
//! - model.rs: DTOs and API functions
//! - view_model.rs: WbSalesDetailsVm with RwSignals
//! - page.rs: Main component with Header, TabBar, TabContent
//! - tabs/: Tab components (general, line, json, links, projections)

mod model;
mod page;
mod tabs;
mod view_model;

pub use page::WbSalesDetail;
pub use view_model::WbSalesDetailsVm;
