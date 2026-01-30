//! Universal Dashboard UI components
//!
//! ## Structure
//! - `schema_browser/` - Schema browser page
//! - `dashboard/` - Universal dashboard with tabs (result, settings, saved, sql)
//!   - `result_tab` - Query results display
//!   - `settings_tab` - Configuration interface
//!   - `saved_tab` - Saved configurations list
//!   - `sql_tab` - SQL query viewer
//!   - `tabs_container` - Tab bar and content container
//! - `picker/` - Schema picker dropdown
//! - `condition_editor/` - Filter condition editor with modal and tabs
//! - `config_panel` - Legacy config panel (deprecated, use SettingsTable)
//! - `pivot_table` - Result table renderer
//! - `saved_configs` - Saved configurations list and dialogs
//! - `settings_table` - Field configuration table
//! - `sql_viewer` - SQL query display

pub mod condition_editor;
pub mod schema_browser;
pub mod config_panel;
pub mod dashboard;
pub mod picker;
pub mod pivot_table;
pub mod saved_configs;
pub mod settings_table;
pub mod sql_viewer;

pub use condition_editor::*;
pub use schema_browser::SchemaBrowser;
pub use config_panel::*;
pub use dashboard::UniversalDashboard;
pub use picker::*;
pub use pivot_table::*;
pub use saved_configs::*;
pub use settings_table::*;
pub use sql_viewer::*;
