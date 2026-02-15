//! Tab components for WB Orders details

mod general;
mod json;
mod line;
mod links;
mod sales;

pub use general::GeneralTab;
pub use json::JsonTab;
pub use line::LineTab;
pub use links::LinksTab;
pub use sales::SalesTab;