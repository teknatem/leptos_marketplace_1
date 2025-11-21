pub mod components;
pub mod data;
pub mod excel_importer;
pub mod export;
pub mod icons;
pub mod json_viewer;
pub mod list_utils;
pub mod picker_aggregate;
pub mod state;

// Unified init function: no-op on client WASM
pub async fn init_data_layer(_db_path: Option<&str>) -> Result<(), ()> {
    // No database in WASM environment; do nothing
    Ok(())
}
