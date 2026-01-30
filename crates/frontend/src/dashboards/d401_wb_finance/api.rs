use contracts::shared::universal_dashboard::{
    DashboardConfig, DeleteDashboardConfigResponse, DistinctValuesResponse,
    ExecuteDashboardRequest, ExecuteDashboardResponse, GenerateSqlResponse, GetSchemaResponse,
    ListDashboardConfigsResponse, ListSchemasResponse, SaveDashboardConfigRequest,
    SaveDashboardConfigResponse, SavedDashboardConfig, UpdateDashboardConfigRequest,
};
use gloo_net::http::Request;

const BASE_URL: &str = "/api/d401";

/// Execute a dashboard query
pub async fn execute_dashboard(
    request: ExecuteDashboardRequest,
) -> Result<ExecuteDashboardResponse, String> {
    Request::post(&format!("{}/execute", BASE_URL))
        .json(&request)
        .map_err(|e| format!("Failed to serialize request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Generate SQL preview
pub async fn generate_sql(config: DashboardConfig) -> Result<GenerateSqlResponse, String> {
    Request::post(&format!("{}/generate-sql", BASE_URL))
        .json(&ExecuteDashboardRequest { config })
        .map_err(|e| format!("Failed to serialize request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// List available data source schemas
pub async fn list_schemas() -> Result<ListSchemasResponse, String> {
    Request::get(&format!("{}/schemas", BASE_URL))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Get schema details
pub async fn get_schema(id: &str) -> Result<GetSchemaResponse, String> {
    Request::get(&format!("{}/schemas/{}", BASE_URL, id))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// List saved dashboard configurations
pub async fn list_configs() -> Result<ListDashboardConfigsResponse, String> {
    Request::get(&format!("{}/configs", BASE_URL))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Get a saved dashboard configuration
pub async fn get_config(id: &str) -> Result<SavedDashboardConfig, String> {
    Request::get(&format!("{}/configs/{}", BASE_URL, id))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Save a new dashboard configuration
pub async fn save_config(
    request: SaveDashboardConfigRequest,
) -> Result<SaveDashboardConfigResponse, String> {
    Request::post(&format!("{}/configs", BASE_URL))
        .json(&request)
        .map_err(|e| format!("Failed to serialize request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Update a dashboard configuration
pub async fn update_config(
    id: &str,
    request: UpdateDashboardConfigRequest,
) -> Result<SaveDashboardConfigResponse, String> {
    Request::put(&format!("{}/configs/{}", BASE_URL, id))
        .json(&request)
        .map_err(|e| format!("Failed to serialize request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Delete a dashboard configuration
pub async fn delete_config(id: &str) -> Result<DeleteDashboardConfigResponse, String> {
    Request::delete(&format!("{}/configs/{}", BASE_URL, id))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
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
    .map_err(|e| format!("Request failed: {}", e))?
    .json()
    .await
    .map_err(|e| format!("Failed to parse response: {}", e))
}
