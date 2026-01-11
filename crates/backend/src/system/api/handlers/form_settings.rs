use axum::{extract::Path, http::StatusCode, Json};
use chrono::Utc;
use contracts::shared::form_settings::{FormSettings, SaveSettingsRequest, SaveSettingsResponse};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

use crate::shared::data::db::get_connection;

pub async fn get_settings(
    Path(form_key): Path<String>,
) -> Result<Json<Option<FormSettings>>, StatusCode> {
    let conn = get_connection();

    let query = r#"
        SELECT form_key, settings_json
        FROM user_form_settings
        WHERE form_key = ?
    "#;

    match conn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            query,
            vec![form_key.clone().into()],
        ))
        .await
    {
        Ok(Some(row)) => {
            let settings_json: String = row
                .try_get("", "settings_json")
                .map_err(|e| {
                    tracing::error!("Failed to get settings_json: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            Ok(Json(Some(FormSettings {
                form_key,
                settings_json,
            })))
        }
        Ok(None) => Ok(Json(None)),
        Err(e) => {
            tracing::error!("Failed to load form settings: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn save_settings(
    Json(request): Json<SaveSettingsRequest>,
) -> Result<Json<SaveSettingsResponse>, StatusCode> {
    let conn = get_connection();

    let settings_json = serde_json::to_string(&request.settings).map_err(|e| {
        tracing::error!("Failed to serialize settings: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    let updated_at = Utc::now().to_rfc3339();

    let query = r#"
        INSERT INTO user_form_settings (form_key, settings_json, updated_at)
        VALUES (?, ?, ?)
        ON CONFLICT(form_key) DO UPDATE SET
            settings_json = excluded.settings_json,
            updated_at = excluded.updated_at
    "#;

    match conn
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            query,
            vec![
                request.form_key.into(),
                settings_json.into(),
                updated_at.into(),
            ],
        ))
        .await
    {
        Ok(_) => Ok(Json(SaveSettingsResponse {
            success: true,
            message: "Settings saved successfully".to_string(),
        })),
        Err(e) => {
            tracing::error!("Failed to save form settings: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

