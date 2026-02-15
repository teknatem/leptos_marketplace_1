//! YM Orders Details UI Module (MVVM Standard)
//!
//! Structure:
//! - model.rs: DTOs and API functions
//! - view_model.rs: YmOrderDetailsVm with reactive state and commands
//! - page.rs: main page (header, tabs, tab content)
//! - tabs/: tab components (general, lines, campaign, json, projections)

mod model;
mod page;
mod tabs;
mod view_model;

pub use page::YmOrderDetail;
pub use view_model::YmOrderDetailsVm;
