//! Tab components for Nomenclature details form
//!
//! Each tab is a separate file for better organization and maintainability.

mod barcodes;
mod dimensions;
mod general;

pub use barcodes::BarcodesTab;
pub use dimensions::DimensionsTab;
pub use general::GeneralTab;
