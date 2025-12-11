pub mod components;
pub mod data;
pub mod date_utils;
pub mod excel_importer;
pub mod export;
pub mod icons;
pub mod json_viewer;
pub mod list_utils;
pub mod modal;
pub mod picker_aggregate;
pub mod state;
pub mod table_utils;
pub mod theme;

// Unified init function: no-op on client WASM
pub async fn init_data_layer(_db_path: Option<&str>) -> Result<(), ()> {
    // No database in WASM environment; do nothing
    Ok(())
}
