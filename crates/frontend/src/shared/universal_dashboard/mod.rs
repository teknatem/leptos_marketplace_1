//! Universal Dashboard - schema-based dynamic reports
//!
//! ## Architecture
//! The universal dashboard provides a flexible system for creating dynamic reports from any schema.
//!
//! ## Structure
//! - `api` - API client for backend communication (execute, save, load configs)
//! - `ui/` - UI components
//!   - `dashboard/` - Universal dashboard with tab-based UI
//!   - `schema_browser/` - Schema browser and validation page
//!   - `picker/` - Schema picker dropdown component
//!   - Individual components: pivot_table, settings_table, saved_configs, sql_viewer
//!
//! ## Usage
//! ```rust
//! // Universal dashboard (any schema)
//! <UniversalDashboard />
//!
//! // Fixed schema dashboard
//! <UniversalDashboard
//!     initial_schema_id="p903_wb_finance_report".to_string()
//!     fixed_schema=true
//!     title="Custom Title".to_string()
//! />
//! ```

pub mod api;
pub mod ui;

// Re-export main components for convenience
pub use ui::AllReportsDetails;
pub use ui::AllReportsList;
pub use ui::SchemaBrowser;
pub use ui::UniversalDashboard;
pub use ui::{
    ConfigPanel, PivotTable, SaveConfigDialog, SavedConfigsList, SchemaPicker, SettingsTable,
};

// Re-export SqlViewer from shared components
pub use crate::shared::components::sql_viewer::SqlViewer;
