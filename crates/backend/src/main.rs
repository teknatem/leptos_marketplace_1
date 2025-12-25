#![allow(
    clippy::useless_format,
    clippy::unnecessary_map_or,
    clippy::type_complexity,
    clippy::manual_div_ceil,
    clippy::unused_enumerate_index,
    clippy::unnecessary_lazy_evaluations,
    clippy::too_many_arguments,
    clippy::if_same_then_else,
    clippy::unnecessary_cast,
    clippy::redundant_pattern_matching,
    clippy::option_as_ref_deref,
    clippy::derivable_impls
)]

pub mod dashboards;
pub mod domain;
pub mod handlers;
pub mod projections;
pub mod routes;
pub mod shared;
pub mod system;
pub mod usecases;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use axum::http::{header, Method};
    use axum::middleware;
    use std::net::SocketAddr;
    use tokio::net::TcpListener;
    use tower_http::cors::{Any, CorsLayer};
    use tower_http::services::ServeDir;

    // 1. Initialize tracing (системное логирование)
    system::tracing::initialize()?;

    // 2. Initialize database (loads config from config.toml)
    shared::data::db::initialize_database()
        .await
        .map_err(|e| anyhow::anyhow!("db init failed: {e}"))?;

    // 3. Apply auth system migration
    system::initialization::apply_auth_migration().await?;

    // 4. Ensure admin user exists
    system::initialization::ensure_admin_user_exists().await?;

    // 4.1. Initialize scheduled tasks
    let worker = system::sys_scheduled_task::initialization::initialize_scheduled_tasks().await?;

    // 4.2. Start background worker for scheduled tasks
    tokio::spawn(async move {
        worker.run_loop().await;
    });

    // 5. Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT, header::AUTHORIZATION]);

    // 6. Build app with routes
    let app = routes::configure_routes()
        .fallback_service(ServeDir::new("dist"))
        .layer(middleware::from_fn(
            system::middleware::request_logger::request_logger,
        ))
        .layer(cors);

    // 7. Start server
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
            return Err(e.into());
        }
    };

    axum::serve(listener, app).await?;

    Ok(())
}
