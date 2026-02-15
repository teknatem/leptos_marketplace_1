//! WB Orders Details UI Module (MVVM Standard)
//!
//! Structure:
//! - model.rs: DTOs and API functions
//! - view_model.rs: WbOrdersDetailsVm with reactive state and commands
//! - page.rs: main page, header, tab bar, tab routing
//! - tabs/: tab components (general, line, json, links)

mod model;
mod page;
mod tabs;
mod view_model;

pub use page::WbOrdersDetail;
pub use view_model::WbOrdersDetailsVm;
