pub mod domain;
pub mod shared;
pub mod usecases;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use axum::http::{header, Method};
    use axum::{
        extract::Path,
        routing::{delete, get, post},
        Json, Router,
    };
    use serde_json::json;
    use std::net::SocketAddr;
    use tokio::net::TcpListener;
    use tower_http::cors::{Any, CorsLayer};
    use tower_http::services::ServeDir;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Define a database path in the `target` directory in a platform-agnostic way
    let db_path = std::path::Path::new("target").join("db").join("app.db");

    let db_path_str = db_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid database path string"))?;

    // Initialize database (create DB file, ensure tables/columns)
    shared::data::db::initialize_database(Some(db_path_str))
        .await
        .map_err(|e| anyhow::anyhow!("db init failed: {e}"))?;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT]);

    // Minimal JSON endpoints for the aggregate to enable quick testing without server_fn
    async fn list_connection_1c_handler() -> Result<
        Json<Vec<contracts::domain::a001_connection_1c::aggregate::Connection1CDatabase>>,
        axum::http::StatusCode,
    > {
        match domain::a001_connection_1c::service::list_all().await {
            Ok(v) => Ok(Json(v)),
            Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
    async fn insert_test_data_handler() -> axum::http::StatusCode {
        match domain::a001_connection_1c::service::insert_test_data().await {
            Ok(_) => axum::http::StatusCode::OK,
            Err(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    async fn get_connection_1c_by_id_handler(
        Path(id): Path<String>,
    ) -> Result<
        Json<contracts::domain::a001_connection_1c::aggregate::Connection1CDatabase>,
        axum::http::StatusCode,
    > {
        let uuid = match uuid::Uuid::parse_str(&id) {
            Ok(uuid) => uuid,
            Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
        };
        match domain::a001_connection_1c::service::get_by_id(uuid).await {
            Ok(Some(v)) => Ok(Json(v)),
            Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
            Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    async fn upsert_connection_1c_handler(
        Json(dto): Json<contracts::domain::a001_connection_1c::aggregate::Connection1CDatabaseDto>,
    ) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
        // Определяем операцию: create или update
        let result = if dto.id.is_some() {
            domain::a001_connection_1c::service::update(dto)
                .await
                .map(|_| uuid::Uuid::nil().to_string())
        } else {
            domain::a001_connection_1c::service::create(dto)
                .await
                .map(|id| id.to_string())
        };

        match result {
            Ok(id) => Ok(Json(json!({"id": id}))),
            Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    async fn test_connection_1c_handler(
        Json(dto): Json<contracts::domain::a001_connection_1c::aggregate::Connection1CDatabaseDto>,
    ) -> Result<
        Json<contracts::domain::a001_connection_1c::aggregate::ConnectionTestResult>,
        axum::http::StatusCode,
    > {
        match domain::a001_connection_1c::service::test_connection(dto).await {
            Ok(result) => Ok(Json(result)),
            Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    // Organization handlers
    async fn list_organization_handler() -> Result<
        Json<Vec<contracts::domain::a002_organization::aggregate::Organization>>,
        axum::http::StatusCode,
    > {
        match domain::a002_organization::service::list_all().await {
            Ok(v) => Ok(Json(v)),
            Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    async fn get_organization_by_id_handler(
        Path(id): Path<String>,
    ) -> Result<
        Json<contracts::domain::a002_organization::aggregate::Organization>,
        axum::http::StatusCode,
    > {
        let uuid = match uuid::Uuid::parse_str(&id) {
            Ok(uuid) => uuid,
            Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
        };
        match domain::a002_organization::service::get_by_id(uuid).await {
            Ok(Some(v)) => Ok(Json(v)),
            Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
            Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    async fn upsert_organization_handler(
        Json(dto): Json<contracts::domain::a002_organization::aggregate::OrganizationDto>,
    ) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
        let result = if dto.id.is_some() {
            domain::a002_organization::service::update(dto)
                .await
                .map(|_| uuid::Uuid::nil().to_string())
        } else {
            domain::a002_organization::service::create(dto)
                .await
                .map(|id| id.to_string())
        };

        match result {
            Ok(id) => Ok(Json(json!({"id": id}))),
            Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    async fn delete_organization_handler(
        Path(id): Path<String>,
    ) -> Result<(), axum::http::StatusCode> {
        let uuid = match uuid::Uuid::parse_str(&id) {
            Ok(uuid) => uuid,
            Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
        };
        match domain::a002_organization::service::delete(uuid).await {
            Ok(true) => Ok(()),
            Ok(false) => Err(axum::http::StatusCode::NOT_FOUND),
            Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    async fn insert_organization_test_data_handler() -> axum::http::StatusCode {
        match domain::a002_organization::service::insert_test_data().await {
            Ok(_) => axum::http::StatusCode::OK,
            Err(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    // UseCase u501: Import from UT handlers
    use once_cell::sync::Lazy;
    use std::sync::Arc;
    static IMPORT_EXECUTOR: Lazy<Arc<usecases::u501_import_from_ut::ImportExecutor>> =
        Lazy::new(|| {
            let tracker = Arc::new(usecases::u501_import_from_ut::ProgressTracker::new());
            Arc::new(usecases::u501_import_from_ut::ImportExecutor::new(tracker))
        });

    async fn start_import_handler(
        Json(request): Json<contracts::usecases::u501_import_from_ut::ImportRequest>,
    ) -> Result<
        Json<contracts::usecases::u501_import_from_ut::ImportResponse>,
        axum::http::StatusCode,
    > {
        match IMPORT_EXECUTOR.start_import(request).await {
            Ok(response) => Ok(Json(response)),
            Err(e) => {
                tracing::error!("Failed to start import: {}", e);
                Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    async fn get_import_progress_handler(
        Path(session_id): Path<String>,
    ) -> Result<
        Json<contracts::usecases::u501_import_from_ut::progress::ImportProgress>,
        axum::http::StatusCode,
    > {
        match IMPORT_EXECUTOR.get_progress(&session_id) {
            Some(progress) => Ok(Json(progress)),
            None => Err(axum::http::StatusCode::NOT_FOUND),
        }
    }

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route(
            "/api/connection_1c",
            get(list_connection_1c_handler).post(upsert_connection_1c_handler),
        )
        .route(
            "/api/connection_1c/:id",
            get(get_connection_1c_by_id_handler),
        )
        .route("/api/connection_1c/test", post(test_connection_1c_handler))
        .route(
            "/api/connection_1c/testdata",
            post(insert_test_data_handler),
        )
        .route(
            "/api/organization",
            get(list_organization_handler).post(upsert_organization_handler),
        )
        .route(
            "/api/organization/:id",
            get(get_organization_by_id_handler).delete(delete_organization_handler),
        )
        .route(
            "/api/organization/testdata",
            post(insert_organization_test_data_handler),
        )
        // UseCase u501: Import from UT
        .route("/api/u501/import/start", post(start_import_handler))
        .route(
            "/api/u501/import/:session_id/progress",
            get(get_import_progress_handler),
        )
        .fallback_service(ServeDir::new("dist"))
        .layer(cors);

    let addr: SocketAddr = ([0, 0, 0, 0], 3000).into();

    tracing::info!("Attempting to bind server to http://{}", addr);
    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => {
            tracing::info!("Server successfully bound to {}", addr);
            listener
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::AddrInUse {
                tracing::error!(
                    "Error: Port 3000 is already in use. Please ensure no other process is using this port."
                );
            } else {
                tracing::error!("Failed to bind to port 3000. Error: {}", e);
            }
            // Propagate the error to stop the application
            return Err(e.into());
        }
    };

    axum::serve(listener, app).await?;

    Ok(())
}
