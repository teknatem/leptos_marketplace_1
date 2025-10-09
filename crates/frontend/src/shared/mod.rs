pub mod data;
pub mod icons;
pub mod picker_aggregate;

// Unified init function: no-op on client WASM
pub async fn init_data_layer(_db_path: Option<&str>) -> Result<(), ()> {
    // No database in WASM environment; do nothing
    Ok(())
}
