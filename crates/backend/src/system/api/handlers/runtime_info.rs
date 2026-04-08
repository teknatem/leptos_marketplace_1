use axum::{http::StatusCode, Json};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct RuntimeInfoResponse {
    pub config_path: String,
    pub database_path: String,
}

pub async fn get_runtime_info() -> Result<Json<RuntimeInfoResponse>, (StatusCode, String)> {
    let config = crate::shared::config::load_config().map_err(internal_error)?;
    let config_path = crate::shared::config::get_config_path().map_err(internal_error)?;
    let database_path =
        crate::shared::config::get_database_path(&config).map_err(internal_error)?;

    Ok(Json(RuntimeInfoResponse {
        config_path: config_path.display().to_string(),
        database_path: database_path.display().to_string(),
    }))
}

fn internal_error(error: anyhow::Error) -> (StatusCode, String) {
    tracing::error!("runtime-info failed: {}", error);
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}
