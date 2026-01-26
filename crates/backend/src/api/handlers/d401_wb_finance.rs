use axum::{
    extract::Path,
    http::StatusCode,
    Json,
};
use contracts::shared::pivot::{
    DeleteDashboardConfigResponse, DistinctValuesResponse, ExecuteDashboardRequest,
    ExecuteDashboardResponse, GenerateSqlResponse, GetSchemaResponse,
    ListDashboardConfigsResponse, ListSchemasResponse, SaveDashboardConfigRequest,
    SaveDashboardConfigResponse, SavedDashboardConfig, SchemaInfo, UpdateDashboardConfigRequest,
};

use crate::dashboards::d401_wb_finance::{schema::P903_SCHEMA, service};

/// POST /api/d401/execute
/// Execute a dashboard query
pub async fn execute_dashboard(
    Json(request): Json<ExecuteDashboardRequest>,
) -> Result<Json<ExecuteDashboardResponse>, StatusCode> {
    tracing::info!(
        "D401 Dashboard: Executing query for data source: {}",
        request.config.data_source
    );

    match service::execute_dashboard(request.config).await {
        Ok(response) => {
            tracing::info!(
                "D401 Dashboard: Returning {} rows, {} columns",
                response.rows.len(),
                response.columns.len()
            );
            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("D401 Dashboard: Failed to execute query: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /api/d401/generate-sql
/// Generate SQL query without executing
pub async fn generate_sql(
    Json(request): Json<ExecuteDashboardRequest>,
) -> Result<Json<GenerateSqlResponse>, StatusCode> {
    tracing::info!(
        "D401 Dashboard: Generating SQL for data source: {}",
        request.config.data_source
    );

    match service::generate_sql(request.config).await {
        Ok(response) => {
            tracing::info!("D401 Dashboard: Generated SQL query successfully");
            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("D401 Dashboard: Failed to generate SQL: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/d401/schemas
/// List available data source schemas
pub async fn list_schemas() -> Result<Json<ListSchemasResponse>, StatusCode> {
    tracing::info!("D401 Dashboard: Listing available schemas");

    let schemas = vec![SchemaInfo {
        id: P903_SCHEMA.id.to_string(),
        name: P903_SCHEMA.name.to_string(),
    }];

    Ok(Json(ListSchemasResponse { schemas }))
}

/// GET /api/d401/schemas/:id
/// Get schema details
pub async fn get_schema(
    Path(id): Path<String>,
) -> Result<Json<GetSchemaResponse>, StatusCode> {
    tracing::info!("D401 Dashboard: Getting schema: {}", id);

    if id == P903_SCHEMA.id {
        Ok(Json(GetSchemaResponse {
            schema: (&P903_SCHEMA).into(),
        }))
    } else {
        tracing::warn!("D401 Dashboard: Schema not found: {}", id);
        Err(StatusCode::NOT_FOUND)
    }
}

/// GET /api/d401/configs
/// List saved dashboard configurations
pub async fn list_configs() -> Result<Json<ListDashboardConfigsResponse>, StatusCode> {
    tracing::info!("D401 Dashboard: Listing saved configurations");

    match service::list_dashboard_configs(Some(P903_SCHEMA.id)).await {
        Ok(configs) => {
            tracing::info!(
                "D401 Dashboard: Returning {} saved configurations",
                configs.len()
            );
            Ok(Json(ListDashboardConfigsResponse { configs }))
        }
        Err(e) => {
            tracing::error!("D401 Dashboard: Failed to list configs: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/d401/configs/:id
/// Get a saved dashboard configuration
pub async fn get_config(
    Path(id): Path<String>,
) -> Result<Json<SavedDashboardConfig>, StatusCode> {
    tracing::info!("D401 Dashboard: Getting configuration: {}", id);

    match service::get_dashboard_config(&id).await {
        Ok(config) => Ok(Json(config)),
        Err(e) => {
            tracing::error!("D401 Dashboard: Failed to get config: {}", e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// POST /api/d401/configs
/// Save a new dashboard configuration
pub async fn save_config(
    Json(request): Json<SaveDashboardConfigRequest>,
) -> Result<Json<SaveDashboardConfigResponse>, StatusCode> {
    tracing::info!("D401 Dashboard: Saving configuration: {}", request.name);

    match service::save_dashboard_config(request).await {
        Ok(config) => {
            tracing::info!("D401 Dashboard: Saved configuration with ID: {}", config.id);
            Ok(Json(SaveDashboardConfigResponse {
                id: config.id,
                message: "Configuration saved successfully".to_string(),
            }))
        }
        Err(e) => {
            tracing::error!("D401 Dashboard: Failed to save config: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// PUT /api/d401/configs/:id
/// Update a dashboard configuration
pub async fn update_config(
    Path(id): Path<String>,
    Json(mut request): Json<UpdateDashboardConfigRequest>,
) -> Result<Json<SaveDashboardConfigResponse>, StatusCode> {
    tracing::info!("D401 Dashboard: Updating configuration: {}", id);

    request.id = id.clone();

    match service::update_dashboard_config(request).await {
        Ok(_) => {
            tracing::info!("D401 Dashboard: Updated configuration: {}", id);
            Ok(Json(SaveDashboardConfigResponse {
                id,
                message: "Configuration updated successfully".to_string(),
            }))
        }
        Err(e) => {
            tracing::error!("D401 Dashboard: Failed to update config: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// DELETE /api/d401/configs/:id
/// Delete a dashboard configuration
pub async fn delete_config(
    Path(id): Path<String>,
) -> Result<Json<DeleteDashboardConfigResponse>, StatusCode> {
    tracing::info!("D401 Dashboard: Deleting configuration: {}", id);

    match service::delete_dashboard_config(&id).await {
        Ok(_) => {
            tracing::info!("D401 Dashboard: Deleted configuration: {}", id);
            Ok(Json(DeleteDashboardConfigResponse {
                success: true,
                message: "Configuration deleted successfully".to_string(),
            }))
        }
        Err(e) => {
            tracing::error!("D401 Dashboard: Failed to delete config: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/d401/schemas/:schema_id/fields/:field_id/values
/// Get distinct values for a field
pub async fn get_distinct_values(
    Path((schema_id, field_id)): Path<(String, String)>,
) -> Result<Json<DistinctValuesResponse>, StatusCode> {
    tracing::info!(
        "D401 Dashboard: Getting distinct values for field {} in schema {}",
        field_id,
        schema_id
    );

    // Validate schema
    if schema_id != P903_SCHEMA.id {
        tracing::warn!("D401 Dashboard: Schema not found: {}", schema_id);
        return Err(StatusCode::NOT_FOUND);
    }

    match service::get_distinct_values(&field_id, Some(100)).await {
        Ok(values) => {
            tracing::info!(
                "D401 Dashboard: Returning {} distinct values for field {}",
                values.len(),
                field_id
            );
            Ok(Json(DistinctValuesResponse { field_id, values }))
        }
        Err(e) => {
            tracing::error!("D401 Dashboard: Failed to get distinct values: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
