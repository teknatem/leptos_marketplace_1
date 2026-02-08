//! Tab components for Nomenclature details form
//!
//! Each tab is a separate file for better organization and maintainability.

mod barcodes;
mod dealer_prices;
mod dimensions;
mod general;

pub use barcodes::BarcodesTab;
pub use dealer_prices::DealerPricesTab;
pub use dimensions::DimensionsTab;
pub use general::GeneralTab;
