pub mod catalog;
pub mod data_view;
pub mod raw_query;
pub mod schema_query;
pub mod sql_guard;

pub use catalog::{list_sources, DataSourceCatalogItem, DataSourceKind, DataSourceRef};
pub use data_view::{
    run_data_view_drilldown, run_data_view_scalar, DataViewDrilldownRequest, DataViewScalarRequest,
};
pub use raw_query::{execute_raw_query, RawQueryRequest, SqlAccessProfile};
pub use schema_query::{query_schema, SchemaQueryRequest};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabularResult {
    pub source: DataSourceRef,
    pub columns: Vec<String>,
    pub rows: Vec<serde_json::Value>,
    pub row_count: usize,
    pub truncated: bool,
}
