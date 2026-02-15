//! Universal Dashboard UI components
//!
//! ## Structure
//! - `schema_browser/` - Schema browser page
//! - `schema_details/` - Schema details with tabs (fields, settings, sql, test)
//! - `dashboard/` - Universal dashboard with tabs (result, settings, saved, sql)
//!   - `result_tab` - Query results display
//!   - `settings_tab` - Configuration interface
//!   - `saved_tab` - Saved configurations list
//!   - `sql_tab` - SQL query viewer
//!   - `tabs_container` - Tab bar and content container
//! - `all_reports_list/` - All reports list page
//! - `all_reports_details/` - All reports details page (view/edit config)
//! - `picker/` - Schema picker dropdown
//! - `condition_editor/` - Filter condition editor with modal and tabs
//! - `config_panel` - Legacy config panel (deprecated, use SettingsTable)
//! - `pivot_table` - Result table renderer
//! - `saved_configs` - Saved configurations list and dialogs
//! - `settings_table` - Field configuration table

pub mod all_reports_details;
pub mod all_reports_list;
pub mod condition_editor;
pub mod config_panel;
pub mod dashboard;
pub mod picker;
pub mod pivot_table;
pub mod saved_configs;
pub mod schema_browser;
pub mod schema_details;
pub mod settings_table;

pub use all_reports_details::AllReportsDetails;
pub use all_reports_list::AllReportsList;
pub use condition_editor::*;
pub use config_panel::*;
pub use dashboard::UniversalDashboard;
pub use picker::*;
pub use pivot_table::*;
pub use saved_configs::*;
pub use schema_browser::SchemaBrowser;
pub use schema_details::SchemaDetails;
pub use settings_table::*;
