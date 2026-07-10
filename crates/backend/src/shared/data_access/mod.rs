pub mod catalog;
pub mod data_view;
pub mod raw_query;
pub mod row_json;
pub mod schema_query;
pub mod sql_guard;

pub use catalog::{list_sources, DataSourceCatalogItem, DataSourceKind, DataSourceRef};
pub use data_view::{
    run_data_view_drilldown, run_data_view_scalar, DataViewContextRequest,
    DataViewDrilldownRequest, DataViewScalarRequest,
};
pub use raw_query::{execute_raw_query, RawQueryRequest, SqlAccessProfile};
pub use schema_query::{
    query_schema, SchemaAggregate, SchemaFilter, SchemaFilterOperator, SchemaMetric,
    SchemaQueryRequest, SchemaSortDirection, SchemaSortRule,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabularResult {
    pub source: DataSourceRef,
    pub columns: Vec<String>,
    pub rows: Vec<serde_json::Value>,
    pub row_count: usize,
    pub truncated: bool,
    /// Готовый SQL, сгенерированный безопасной схемой (с реальными JOIN-ами и колонками,
    /// значения фильтров подставлены литералами). Это КАНОНИЧЕСКИЙ правильный запрос —
    /// модель может переиспользовать его для raw SQL/build_chart вместо угадывания колонок
    /// (поля схемы вроде `dim1` НЕ являются колонками таблицы). None для raw/DataView.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_sql: Option<String>,
}
