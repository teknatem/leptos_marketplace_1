pub mod api_utils;
pub mod auth_download;
pub mod bi_card;
pub mod clipboard;
pub mod code_format;
pub mod components;
pub mod data;
pub mod date_utils;
pub mod dom_validator;
pub mod drilldown_report;
pub mod excel_importer;
pub mod export;
pub mod filters;
pub mod icons;
pub mod indicator_format;
pub mod json_viewer;
pub mod list_utils;
pub mod modal_frame;
pub mod modal_stack;
pub mod page_frame;
pub mod page_standard;
pub mod picker_aggregate;
pub mod state;
pub mod table_utils;
pub mod theme;
pub mod universal_dashboard;

// Unified init function: no-op on client WASM
pub async fn init_data_layer(_db_path: Option<&str>) -> Result<(), ()> {
    // No database in WASM environment; do nothing
    Ok(())
}
