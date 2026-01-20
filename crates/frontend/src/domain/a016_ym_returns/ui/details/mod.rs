//! YM Returns Details UI Module (Standard Tab Structure)
//!
//! Structure:
//! - model.rs: DTOs and constants
//! - page.rs: Main component with loading logic and tab navigation
//! - tabs/: Tab components (general, lines, projections, json)

mod model;
mod page;
mod tabs;

pub use page::YmReturnDetail;
