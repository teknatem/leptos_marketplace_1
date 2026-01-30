//! API client for universal dashboard mechanism
//!
//! All API calls for schemas, configs, validation, and dashboard execution.

use contracts::shared::universal_dashboard::{
    DashboardConfig, DeleteDashboardConfigResponse, DistinctValuesResponse,
    ExecuteDashboardRequest, ExecuteDashboardResponse, GenerateSqlResponse,
    GetSchemaResponse, ListDashboardConfigsResponse, ListSchemasResponse,
    SaveDashboardConfigRequest, SaveDashboardConfigResponse, SavedDashboardConfig,
    SchemaValidationResult, UpdateDashboardConfigRequest, ValidateAllSchemasResponse,
};
use gloo_net::http::Request;

const BASE_URL: &str = "/api/universal-dashboard";

// ============================================================================
// Schema API
// ============================================================================

/// List all available schemas
pub async fn list_schemas() -> Result<ListSchemasResponse, String> {
    Request::get(&format!("{}/schemas", BASE_URL))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

/// Get schema details by ID
pub async fn get_schema(id: &str) -> Result<GetSchemaResponse, String> {
    Request::get(&format!("{}/schemas/{}", BASE_URL, id))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

/// Get distinct values for a field
pub async fn get_distinct_values(
    schema_id: &str,
    field_id: &str,
) -> Result<DistinctValuesResponse, String> {
    Request::get(&format!(
        "{}/schemas/{}/fields/{}/values",
        BASE_URL, schema_id, field_id
    ))
    .send()
    .await
    .map_err(|e| e.to_string())?
    .json()
    .await
    .map_err(|e| e.to_string())
}

// ============================================================================
// Validation API
// ============================================================================

/// Validate a single schema
pub async fn validate_schema(schema_id: &str) -> Result<SchemaValidationResult, String> {
    Request::post(&format!("{}/schemas/{}/validate", BASE_URL, schema_id))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

/// Validate all schemas
pub async fn validate_all_schemas() -> Result<ValidateAllSchemasResponse, String> {
    Request::post(&format!("{}/schemas/validate-all", BASE_URL))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// Dashboard Execution API
// ============================================================================

/// Execute a dashboard query
pub async fn execute_dashboard(
    request: ExecuteDashboardRequest,
) -> Result<ExecuteDashboardResponse, String> {
    Request::post(&format!("{}/execute", BASE_URL))
        .json(&request)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

/// Generate SQL query without executing
pub async fn generate_sql(config: DashboardConfig) -> Result<GenerateSqlResponse, String> {
    Request::post(&format!("{}/generate-sql", BASE_URL))
        .json(&ExecuteDashboardRequest { config })
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// Config API
// ============================================================================

/// List saved dashboard configurations for a specific schema
pub async fn list_configs(schema_id: Option<&str>) -> Result<ListDashboardConfigsResponse, String> {
    let url = match schema_id {
        Some(id) => format!("{}/configs?schema_id={}", BASE_URL, id),
        None => format!("{}/configs", BASE_URL),
    };

    Request::get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

/// Get a saved dashboard configuration
pub async fn get_config(id: &str) -> Result<SavedDashboardConfig, String> {
    Request::get(&format!("{}/configs/{}", BASE_URL, id))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

/// Save a new dashboard configuration
pub async fn save_config(
    request: SaveDashboardConfigRequest,
) -> Result<SaveDashboardConfigResponse, String> {
    Request::post(&format!("{}/configs", BASE_URL))
        .json(&request)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

/// Update a dashboard configuration
pub async fn update_config(
    id: &str,
    request: UpdateDashboardConfigRequest,
) -> Result<SaveDashboardConfigResponse, String> {
    Request::put(&format!("{}/configs/{}", BASE_URL, id))
        .json(&request)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

/// Delete a dashboard configuration
pub async fn delete_config(id: &str) -> Result<DeleteDashboardConfigResponse, String> {
    Request::delete(&format!("{}/configs/{}", BASE_URL, id))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}
