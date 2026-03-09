/// DS03 p904_sales_data service
///
/// Execution is handled by ds02's execute_dashboard (which supports multiple schemas).
/// This module re-exports the shared functions for consistency.
pub use crate::data_schemes::ds02_mp_sales_register::service::{
    execute_dashboard, generate_sql, get_dashboard_config, get_distinct_values,
    list_dashboard_configs, save_dashboard_config,
};
