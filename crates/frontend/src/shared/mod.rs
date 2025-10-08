pub mod aggregate_picker;
pub mod data;
pub mod icons;

pub use aggregate_picker::{AggregatePickerResult, GenericAggregatePicker, TableDisplayable};

// Unified init function: no-op on client WASM
pub async fn init_data_layer(_db_path: Option<&str>) -> Result<(), ()> {
    // No database in WASM environment; do nothing
    Ok(())
}
