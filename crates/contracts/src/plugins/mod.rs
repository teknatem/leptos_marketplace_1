//! Shared contracts for the plugin subsystem.
//!
//! A plugin is a self-contained interpreted artifact (`PluginBundle`) with a
//! manifest, parameters, data binding, client/server JavaScript modules, view
//! metadata, styles, and assets. It is stored and executed without rebuilding
//! the application.

pub mod bundle;
pub mod runs;

pub use bundle::{
    is_read_only_sql, DataBinding, ParamSpec, ParamType, PluginBundle, PluginDefinition,
    PluginError, PluginInvokeRequest, PluginListItem, PluginManifest, PluginRunContext,
    PluginRuntime, PluginStatus, PluginUpsert, PluginValidateReport, ViewSpec, Widget, WidgetKind,
};
pub use runs::{
    PluginHealth, PluginRunBrief, PluginRunRecord, PluginRunSummary, PluginStats, StageCount,
};
