pub mod dashboards;
pub mod domain;
pub mod handlers;
pub mod projections;
pub mod shared;
pub mod usecases;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use axum::body::Body;
    use axum::http::{header, Method, Request};
    use axum::middleware::{self, Next};
    use axum::response::Response;
    use axum::{
        extract::{Path, Query},
        routing::{get, post},
        Json, Router,
    };
    use serde_json::json;
    use std::net::SocketAddr;
    use tokio::net::TcpListener;
    use tower_http::cors::{Any, CorsLayer};
    use tower_http::services::ServeDir;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    // Создаем директорию для логов
    let log_dir = std::path::Path::new("target").join("logs");
    std::fs::create_dir_all(&log_dir)?;

    let log_file_path = log_dir.join("backend.log");
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file_path)?;

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| {
                // Отключаем логи SQL запросов, но оставляем логи приложения
                "info,sqlx=warn,sea_orm=warn".into()
            }),
        ))
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::sync::Arc::new(log_file))
                .with_ansi(false),
        )
        .init();

    // Функция для форматирования чисел с разделителями триад
    fn format_number(n: usize) -> String {
        let s = n.to_string();
        let mut result = String::new();
        for (i, ch) in s.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 {
                result.push('.');
            }
            result.push(ch);
        }
        result.chars().rev().collect()
    }

    // Простой middleware для логирования запросов
    async fn request_logger(req: Request<Body>, next: Next) -> Response {
        use axum::body::to_bytes;
        use chrono::Utc;

        let start = std::time::Instant::now();
        let method = req.method().clone();
        let uri = req.uri().clone();

        let response = next.run(req).await;

        let (parts, body) = response.into_parts();

        // Читаем тело ответа, чтобы узнать реальный размер
        let bytes = match to_bytes(body, usize::MAX).await {
            Ok(b) => b,
            Err(_) => {
                let duration = start.elapsed();
                let timestamp = Utc::now() + chrono::Duration::hours(3);
                // Ошибка - используем коричневый цвет
                println!(
                    "\x1b[33m{}\x1b[0m | {:>5}ms | {:>12} | {} {:>6} {}",
                    timestamp.format("%H:%M:%S"),
                    duration.as_millis(),
                    "error",
                    parts.status.as_u16(),
                    method,
                    uri.path()
                );
                return Response::from_parts(parts, Body::default());
            }
        };

        let size = bytes.len();
        let duration = start.elapsed();
        let timestamp = Utc::now() + chrono::Duration::hours(3);

        // Выбираем цвет для времени: голубой для 200, коричневый для остальных
        let color_code = if parts.status.as_u16() == 200 {
            "36"
        } else {
            "33"
        };

        println!(
            "\x1b[{}m{}\x1b[0m | {:>5}ms | {:>12} | {} {:>6} {}",
            color_code,
            timestamp.format("%H:%M:%S"),
            duration.as_millis(),
            format!("{}", format_number(size)),
            parts.status.as_u16(),
            method,
            uri.path()
        );

        // Создаем новый ответ с прочитанным телом
        Response::from_parts(parts, Body::from(bytes))
    }

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

    // Nomenclature search handler
    #[derive(serde::Deserialize)]
    struct SearchNomenclatureQuery {
        article: String,
    }

    async fn search_nomenclature_by_article(
        Query(query): Query<SearchNomenclatureQuery>,
    ) -> Result<
        Json<Vec<contracts::domain::a004_nomenclature::aggregate::Nomenclature>>,
        axum::http::StatusCode,
    > {
        match domain::a004_nomenclature::repository::find_by_article(query.article.trim()).await {
            Ok(items) => Ok(Json(items)),
            Err(e) => {
                tracing::error!("Failed to search nomenclature by article: {}", e);
                Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            }
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

    // UseCase u502: Import from OZON handlers
    static OZON_IMPORT_EXECUTOR: Lazy<Arc<usecases::u502_import_from_ozon::ImportExecutor>> =
        Lazy::new(|| {
            let tracker = Arc::new(usecases::u502_import_from_ozon::ProgressTracker::new());
            Arc::new(usecases::u502_import_from_ozon::ImportExecutor::new(
                tracker,
            ))
        });

    async fn start_ozon_import_handler(
        Json(request): Json<contracts::usecases::u502_import_from_ozon::ImportRequest>,
    ) -> Result<
        Json<contracts::usecases::u502_import_from_ozon::ImportResponse>,
        axum::http::StatusCode,
    > {
        match OZON_IMPORT_EXECUTOR.start_import(request).await {
            Ok(response) => Ok(Json(response)),
            Err(e) => {
                tracing::error!("Failed to start OZON import: {}", e);
                Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    async fn get_ozon_import_progress_handler(
        Path(session_id): Path<String>,
    ) -> Result<
        Json<contracts::usecases::u502_import_from_ozon::progress::ImportProgress>,
        axum::http::StatusCode,
    > {
        match OZON_IMPORT_EXECUTOR.get_progress(&session_id) {
            Some(progress) => Ok(Json(progress)),
            None => Err(axum::http::StatusCode::NOT_FOUND),
        }
    }

    // UseCase u503: Import from Yandex Market handlers
    static YANDEX_IMPORT_EXECUTOR: Lazy<Arc<usecases::u503_import_from_yandex::ImportExecutor>> =
        Lazy::new(|| {
            let tracker = Arc::new(usecases::u503_import_from_yandex::ProgressTracker::new());
            Arc::new(usecases::u503_import_from_yandex::ImportExecutor::new(
                tracker,
            ))
        });

    async fn start_yandex_import_handler(
        Json(request): Json<contracts::usecases::u503_import_from_yandex::ImportRequest>,
    ) -> Result<
        Json<contracts::usecases::u503_import_from_yandex::ImportResponse>,
        axum::http::StatusCode,
    > {
        match YANDEX_IMPORT_EXECUTOR.start_import(request).await {
            Ok(response) => Ok(Json(response)),
            Err(e) => {
                tracing::error!("Failed to start Yandex Market import: {}", e);
                Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    async fn get_yandex_import_progress_handler(
        Path(session_id): Path<String>,
    ) -> Result<
        Json<contracts::usecases::u503_import_from_yandex::progress::ImportProgress>,
        axum::http::StatusCode,
    > {
        match YANDEX_IMPORT_EXECUTOR.get_progress(&session_id) {
            Some(progress) => Ok(Json(progress)),
            None => Err(axum::http::StatusCode::NOT_FOUND),
        }
    }

    // UseCase u504: Import from Wildberries handlers
    static WB_IMPORT_EXECUTOR: Lazy<Arc<usecases::u504_import_from_wildberries::ImportExecutor>> =
        Lazy::new(|| {
            let tracker = Arc::new(usecases::u504_import_from_wildberries::ProgressTracker::new());
            Arc::new(usecases::u504_import_from_wildberries::ImportExecutor::new(
                tracker,
            ))
        });

    async fn start_wildberries_import_handler(
        Json(request): Json<contracts::usecases::u504_import_from_wildberries::ImportRequest>,
    ) -> Result<
        Json<contracts::usecases::u504_import_from_wildberries::ImportResponse>,
        axum::http::StatusCode,
    > {
        match WB_IMPORT_EXECUTOR.start_import(request).await {
            Ok(response) => Ok(Json(response)),
            Err(e) => {
                tracing::error!("Failed to start Wildberries import: {}", e);
                Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    async fn get_wildberries_import_progress_handler(
        Path(session_id): Path<String>,
    ) -> Result<
        Json<contracts::usecases::u504_import_from_wildberries::progress::ImportProgress>,
        axum::http::StatusCode,
    > {
        match WB_IMPORT_EXECUTOR.get_progress(&session_id) {
            Some(progress) => Ok(Json(progress)),
            None => Err(axum::http::StatusCode::NOT_FOUND),
        }
    }

    // UseCase u505: Match Nomenclature handlers
    static MATCH_NOMENCLATURE_EXECUTOR: Lazy<
        Arc<usecases::u505_match_nomenclature::MatchExecutor>,
    > = Lazy::new(|| {
        let tracker = Arc::new(usecases::u505_match_nomenclature::ProgressTracker::new());
        Arc::new(usecases::u505_match_nomenclature::MatchExecutor::new(
            tracker,
        ))
    });

    async fn start_match_nomenclature_handler(
        Json(request): Json<contracts::usecases::u505_match_nomenclature::MatchRequest>,
    ) -> Result<
        Json<contracts::usecases::u505_match_nomenclature::MatchResponse>,
        axum::http::StatusCode,
    > {
        match MATCH_NOMENCLATURE_EXECUTOR.start_matching(request).await {
            Ok(response) => Ok(Json(response)),
            Err(e) => {
                tracing::error!("Failed to start nomenclature matching: {}", e);
                Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    async fn get_match_nomenclature_progress_handler(
        Path(session_id): Path<String>,
    ) -> Result<
        Json<contracts::usecases::u505_match_nomenclature::progress::MatchProgress>,
        axum::http::StatusCode,
    > {
        match MATCH_NOMENCLATURE_EXECUTOR.get_progress(&session_id) {
            Some(progress) => Ok(Json(progress)),
            None => Err(axum::http::StatusCode::NOT_FOUND),
        }
    }

    // UseCase u506: Import from LemanaPro handlers
    static LEMANAPRO_IMPORT_EXECUTOR: Lazy<
        Arc<usecases::u506_import_from_lemanapro::ImportExecutor>,
    > = Lazy::new(|| {
        let tracker = Arc::new(usecases::u506_import_from_lemanapro::ProgressTracker::new());
        Arc::new(usecases::u506_import_from_lemanapro::ImportExecutor::new(
            tracker,
        ))
    });

    async fn start_lemanapro_import_handler(
        Json(request): Json<contracts::usecases::u506_import_from_lemanapro::ImportRequest>,
    ) -> Result<
        Json<contracts::usecases::u506_import_from_lemanapro::ImportResponse>,
        axum::http::StatusCode,
    > {
        match LEMANAPRO_IMPORT_EXECUTOR.start_import(request).await {
            Ok(response) => Ok(Json(response)),
            Err(e) => {
                tracing::error!("Failed to start LemanaPro import: {}", e);
                Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    async fn get_lemanapro_import_progress_handler(
        Path(session_id): Path<String>,
    ) -> Result<
        Json<contracts::usecases::u506_import_from_lemanapro::progress::ImportProgress>,
        axum::http::StatusCode,
    > {
        match LEMANAPRO_IMPORT_EXECUTOR.get_progress(&session_id) {
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
        // Counterparty handlers
        .route(
            "/api/counterparty",
            get(|| async {
                match domain::a003_counterparty::service::list_all().await {
                    Ok(v) => Ok(Json(v)),
                    Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                }
            })
            .post(
                |Json(dto): Json<
                    contracts::domain::a003_counterparty::aggregate::CounterpartyDto,
                >| async move {
                    let result = if dto.id.is_some() {
                        domain::a003_counterparty::service::update(dto)
                            .await
                            .map(|_| uuid::Uuid::nil().to_string())
                    } else {
                        domain::a003_counterparty::service::create(dto)
                            .await
                            .map(|id| id.to_string())
                    };
                    match result {
                        Ok(id) => Ok(Json(json!({"id": id}))),
                        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                    }
                },
            ),
        )
        .route(
            "/api/counterparty/:id",
            get(|Path(id): Path<String>| async move {
                let uuid = match uuid::Uuid::parse_str(&id) {
                    Ok(uuid) => uuid,
                    Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
                };
                match domain::a003_counterparty::service::get_by_id(uuid).await {
                    Ok(Some(v)) => Ok(Json(v)),
                    Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
                    Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                }
            })
            .delete(|Path(id): Path<String>| async move {
                let uuid = match uuid::Uuid::parse_str(&id) {
                    Ok(uuid) => uuid,
                    Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
                };
                match domain::a003_counterparty::service::delete(uuid).await {
                    Ok(true) => Ok(()),
                    Ok(false) => Err(axum::http::StatusCode::NOT_FOUND),
                    Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                }
            }),
        )
        // Nomenclature handlers
        .route(
            "/api/nomenclature",
            get(|| async {
                match domain::a004_nomenclature::service::list_all().await {
                    Ok(v) => Ok(Json(v)),
                    Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                }
            })
            .post(
                |Json(dto): Json<
                    contracts::domain::a004_nomenclature::aggregate::NomenclatureDto,
                >| async move {
                    let result = if dto.id.is_some() {
                        domain::a004_nomenclature::service::update(dto)
                            .await
                            .map(|_| uuid::Uuid::nil().to_string())
                    } else {
                        domain::a004_nomenclature::service::create(dto)
                            .await
                            .map(|id| id.to_string())
                    };
                    match result {
                        Ok(id) => Ok(Json(json!({"id": id}))),
                        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                    }
                },
            ),
        )
        .route(
            "/api/nomenclature/:id",
            get(|Path(id): Path<String>| async move {
                let uuid = match uuid::Uuid::parse_str(&id) {
                    Ok(uuid) => uuid,
                    Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
                };
                match domain::a004_nomenclature::service::get_by_id(uuid).await {
                    Ok(Some(v)) => Ok(Json(v)),
                    Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
                    Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                }
            })
            .delete(|Path(id): Path<String>| async move {
                let uuid = match uuid::Uuid::parse_str(&id) {
                    Ok(uuid) => uuid,
                    Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
                };
                match domain::a004_nomenclature::service::delete(uuid).await {
                    Ok(true) => Ok(()),
                    Ok(false) => Err(axum::http::StatusCode::NOT_FOUND),
                    Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                }
            }),
        )
        .route(
            "/api/nomenclature/import-excel",
            post(|Json(excel_data): Json<domain::a004_nomenclature::excel_import::ExcelData>| async move {
                tracing::info!("Received Excel import request with {} rows", excel_data.metadata.row_count);

                // Импортируем данные из ExcelData (backend делает маппинг полей)
                let result = match domain::a004_nomenclature::excel_import::import_nomenclature_from_excel_data(excel_data).await {
                    Ok(result) => result,
                    Err(e) => {
                        tracing::error!("Excel import error: {}", e);
                        return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
                    }
                };

                Ok(Json(result))
            }),
        )
        .route(
            "/api/nomenclature/dimensions",
            get(|| async {
                match domain::a004_nomenclature::repository::get_distinct_dimension_values().await {
                    Ok(values) => Ok(Json(values)),
                    Err(e) => {
                        tracing::error!("Failed to get dimension values: {}", e);
                        Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                    }
                }
            }),
        )
        .route(
            "/api/nomenclature/search",
            get(search_nomenclature_by_article),
        )
        // Marketplace handlers
        .route(
            "/api/marketplace",
            get(|| async {
                match domain::a005_marketplace::service::list_all().await {
                    Ok(v) => Ok(Json(v)),
                    Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                }
            })
            .post(
                |Json(dto): Json<
                    contracts::domain::a005_marketplace::aggregate::MarketplaceDto,
                >| async move {
                    let result = if dto.id.is_some() {
                        domain::a005_marketplace::service::update(dto)
                            .await
                            .map(|_| uuid::Uuid::nil().to_string())
                    } else {
                        domain::a005_marketplace::service::create(dto)
                            .await
                            .map(|id| id.to_string())
                    };
                    match result {
                        Ok(id) => Ok(Json(json!({"id": id}))),
                        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                    }
                },
            ),
        )
        .route(
            "/api/marketplace/:id",
            get(|Path(id): Path<String>| async move {
                let uuid = match uuid::Uuid::parse_str(&id) {
                    Ok(uuid) => uuid,
                    Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
                };
                match domain::a005_marketplace::service::get_by_id(uuid).await {
                    Ok(Some(v)) => Ok(Json(v)),
                    Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
                    Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                }
            })
            .delete(|Path(id): Path<String>| async move {
                let uuid = match uuid::Uuid::parse_str(&id) {
                    Ok(uuid) => uuid,
                    Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
                };
                match domain::a005_marketplace::service::delete(uuid).await {
                    Ok(true) => Ok(()),
                    Ok(false) => Err(axum::http::StatusCode::NOT_FOUND),
                    Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                }
            }),
        )
        .route(
            "/api/marketplace/testdata",
            post(|| async {
                match domain::a005_marketplace::service::insert_test_data().await {
                    Ok(_) => axum::http::StatusCode::OK,
                    Err(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                }
            }),
        )
        // Connection MP handlers
        .route(
            "/api/connection_mp",
            get(|| async {
                match domain::a006_connection_mp::service::list_all().await {
                    Ok(v) => Ok(Json(v)),
                    Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                }
            })
            .post(
                |Json(dto): Json<
                    contracts::domain::a006_connection_mp::aggregate::ConnectionMPDto,
                >| async move {
                    let result = if dto.id.is_some() {
                        domain::a006_connection_mp::service::update(dto)
                            .await
                            .map(|_| uuid::Uuid::nil().to_string())
                    } else {
                        domain::a006_connection_mp::service::create(dto)
                            .await
                            .map(|id| id.to_string())
                    };
                    match result {
                        Ok(id) => Ok(Json(json!({"id": id}))),
                        Err(e) => {
                            tracing::error!("Failed to save connection_mp: {}", e);
                            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                        }
                    }
                },
            ),
        )
        .route(
            "/api/connection_mp/:id",
            get(|Path(id): Path<String>| async move {
                let uuid = match uuid::Uuid::parse_str(&id) {
                    Ok(uuid) => uuid,
                    Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
                };
                match domain::a006_connection_mp::service::get_by_id(uuid).await {
                    Ok(Some(v)) => Ok(Json(v)),
                    Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
                    Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                }
            })
            .delete(|Path(id): Path<String>| async move {
                let uuid = match uuid::Uuid::parse_str(&id) {
                    Ok(uuid) => uuid,
                    Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
                };
                match domain::a006_connection_mp::service::delete(uuid).await {
                    Ok(true) => Ok(()),
                    Ok(false) => Err(axum::http::StatusCode::NOT_FOUND),
                    Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                }
            }),
        )
        .route(
            "/api/connection_mp/test",
            post(
                |Json(dto): Json<
                    contracts::domain::a006_connection_mp::aggregate::ConnectionMPDto,
                >| async move {
                    match domain::a006_connection_mp::service::test_connection(dto).await {
                        Ok(result) => Ok(Json(result)),
                        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                    }
                },
            ),
        )
        // Marketplace product handlers
        .route(
            "/api/marketplace_product",
            get(|| async {
                match domain::a007_marketplace_product::service::list_all().await {
                    Ok(v) => Ok(Json(v)),
                    Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                }
            })
            .post(
                |Json(dto): Json<
                    contracts::domain::a007_marketplace_product::aggregate::MarketplaceProductDto,
                >| async move {
                    let result = if dto.id.is_some() {
                        domain::a007_marketplace_product::service::update(dto)
                            .await
                            .map(|_| uuid::Uuid::nil().to_string())
                    } else {
                        domain::a007_marketplace_product::service::create(dto)
                            .await
                            .map(|id| id.to_string())
                    };
                    match result {
                        Ok(id) => Ok(Json(json!({"id": id}))),
                        Err(e) => {
                            tracing::error!("Failed to save marketplace_product: {}", e);
                            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                        }
                    }
                },
            ),
        )
        .route(
            "/api/marketplace_product/:id",
            get(|Path(id): Path<String>| async move {
                let uuid = match uuid::Uuid::parse_str(&id) {
                    Ok(uuid) => uuid,
                    Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
                };
                match domain::a007_marketplace_product::service::get_by_id(uuid).await {
                    Ok(Some(v)) => Ok(Json(v)),
                    Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
                    Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                }
            })
            .delete(|Path(id): Path<String>| async move {
                let uuid = match uuid::Uuid::parse_str(&id) {
                    Ok(uuid) => uuid,
                    Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
                };
                match domain::a007_marketplace_product::service::delete(uuid).await {
                    Ok(true) => Ok(()),
                    Ok(false) => Err(axum::http::StatusCode::NOT_FOUND),
                    Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                }
            }),
        )
        .route(
            "/api/marketplace_product/testdata",
            post(|| async {
                match domain::a007_marketplace_product::service::insert_test_data().await {
                    Ok(_) => axum::http::StatusCode::OK,
                    Err(e) => {
                        tracing::error!("Failed to insert test data: {}", e);
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR
                    }
                }
            }),
        )
        // Marketplace sales handlers
        .route(
            "/api/marketplace_sales",
            get(|| async {
                match domain::a008_marketplace_sales::service::list_all().await {
                    Ok(v) => Ok(Json(v)),
                    Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                }
            })
            .post(
                |Json(dto): Json<
                    contracts::domain::a008_marketplace_sales::aggregate::MarketplaceSalesDto,
                >| async move {
                    let result = if dto.id.is_some() {
                        domain::a008_marketplace_sales::service::update(dto)
                            .await
                            .map(|_| uuid::Uuid::nil().to_string())
                    } else {
                        domain::a008_marketplace_sales::service::create(dto)
                            .await
                            .map(|id| id.to_string())
                    };
                    match result {
                        Ok(id) => Ok(Json(json!({"id": id}))),
                        Err(e) => {
                            tracing::error!("Failed to save marketplace_sales: {}", e);
                            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                        }
                    }
                },
            ),
        )
        .route(
            "/api/marketplace_sales/:id",
            get(|Path(id): Path<String>| async move {
                let uuid = match uuid::Uuid::parse_str(&id) {
                    Ok(uuid) => uuid,
                    Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
                };
                match domain::a008_marketplace_sales::service::get_by_id(uuid).await {
                    Ok(Some(v)) => Ok(Json(v)),
                    Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
                    Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                }
            })
            .delete(|Path(id): Path<String>| async move {
                let uuid = match uuid::Uuid::parse_str(&id) {
                    Ok(uuid) => uuid,
                    Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
                };
                match domain::a008_marketplace_sales::service::delete(uuid).await {
                    Ok(true) => Ok(()),
                    Ok(false) => Err(axum::http::StatusCode::NOT_FOUND),
                    Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                }
            }),
        )
        // OZON Returns handlers
        .route(
            "/api/ozon_returns",
            get(|| async {
                match domain::a009_ozon_returns::service::list_all().await {
                    Ok(aggregates) => {
                        let list_dtos: Vec<_> = aggregates
                            .into_iter()
                            .map(|agg| agg.to_list_dto())
                            .collect();
                        Ok(Json(list_dtos))
                    }
                    Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                }
            })
            .post(
                |Json(dto): Json<
                    contracts::domain::a009_ozon_returns::aggregate::OzonReturnsDto,
                >| async move {
                    let result = if dto.id.is_some() {
                        domain::a009_ozon_returns::service::update(dto)
                            .await
                            .map(|_| uuid::Uuid::nil().to_string())
                    } else {
                        domain::a009_ozon_returns::service::create(dto)
                            .await
                            .map(|id| id.to_string())
                    };
                    match result {
                        Ok(id) => Ok(Json(json!({"id": id}))),
                        Err(e) => {
                            tracing::error!("Failed to save ozon_returns: {}", e);
                            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                        }
                    }
                },
            ),
        )
        .route(
            "/api/ozon_returns/:id",
            get(|Path(id): Path<String>| async move {
                let uuid = match uuid::Uuid::parse_str(&id) {
                    Ok(uuid) => uuid,
                    Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
                };
                match domain::a009_ozon_returns::service::get_by_id(uuid).await {
                    Ok(Some(v)) => {
                        let detail_dto = v.to_detail_dto();
                        Ok(Json(detail_dto))
                    }
                    Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
                    Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                }
            })
            .delete(|Path(id): Path<String>| async move {
                let uuid = match uuid::Uuid::parse_str(&id) {
                    Ok(uuid) => uuid,
                    Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
                };
                match domain::a009_ozon_returns::service::delete(uuid).await {
                    Ok(true) => Ok(()),
                    Ok(false) => Err(axum::http::StatusCode::NOT_FOUND),
                    Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                }
            }),
        )
        // OZON Transactions handlers
        .route(
            "/api/ozon_transactions",
            get(handlers::a014_ozon_transactions::list_all),
        )
        .route(
            "/api/ozon_transactions/:id",
            get(handlers::a014_ozon_transactions::get_by_id)
                .delete(handlers::a014_ozon_transactions::delete),
        )
        .route(
            "/api/ozon_transactions/by-posting/:posting_number",
            get(handlers::a014_ozon_transactions::get_by_posting_number),
        )
        .route(
            "/api/a014/ozon-transactions/:id/post",
            post(handlers::a014_ozon_transactions::post_document),
        )
        .route(
            "/api/a014/ozon-transactions/:id/unpost",
            post(handlers::a014_ozon_transactions::unpost_document),
        )
        .route(
            "/api/a014/ozon-transactions/:id/projections",
            get(handlers::a014_ozon_transactions::get_projections),
        )
        // UseCase u501: Import from UT
        .route("/api/u501/import/start", post(start_import_handler))
        .route(
            "/api/u501/import/:session_id/progress",
            get(get_import_progress_handler),
        )
        // UseCase u502: Import from OZON
        .route("/api/u502/import/start", post(start_ozon_import_handler))
        .route(
            "/api/u502/import/:session_id/progress",
            get(get_ozon_import_progress_handler),
        )
        // UseCase u503: Import from Yandex Market
        .route("/api/u503/import/start", post(start_yandex_import_handler))
        .route(
            "/api/u503/import/:session_id/progress",
            get(get_yandex_import_progress_handler),
        )
        // UseCase u504: Import from Wildberries
        .route(
            "/api/u504/import/start",
            post(start_wildberries_import_handler),
        )
        .route(
            "/api/u504/import/:session_id/progress",
            get(get_wildberries_import_progress_handler),
        )
        // UseCase u505: Match Nomenclature
        .route(
            "/api/u505/match/start",
            post(start_match_nomenclature_handler),
        )
        .route(
            "/api/u505/match/:session_id/progress",
            get(get_match_nomenclature_progress_handler),
        )
        // UseCase u506: Import from LemanaPro
        .route(
            "/api/u506/import/start",
            post(start_lemanapro_import_handler),
        )
        .route(
            "/api/u506/import/:session_id/progress",
            get(get_lemanapro_import_progress_handler),
        )
        // Logs handlers
        .route(
            "/api/logs",
            get(|| async {
                match shared::logger::repository::get_all_logs().await {
                    Ok(logs) => Ok(Json(logs)),
                    Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                }
            })
            .post(
                |Json(req): Json<contracts::shared::logger::CreateLogRequest>| async move {
                    match shared::logger::repository::log_event(
                        &req.source,
                        &req.category,
                        &req.message,
                    )
                    .await
                    {
                        Ok(_) => axum::http::StatusCode::OK,
                        Err(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    }
                },
            )
            .delete(|| async {
                match shared::logger::repository::clear_all_logs().await {
                    Ok(_) => axum::http::StatusCode::OK,
                    Err(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                }
            }),
        )
        // P900 Sales Register handlers
        .route(
            "/api/p900/sales-register",
            get(handlers::p900_sales_register::list_sales),
        )
        .route(
            "/api/p900/sales-register/:marketplace/:document_no/:line_id",
            get(handlers::p900_sales_register::get_sale_detail),
        )
        .route(
            "/api/p900/stats/by-date",
            get(handlers::p900_sales_register::get_stats_by_date),
        )
        .route(
            "/api/p900/stats/by-marketplace",
            get(handlers::p900_sales_register::get_stats_by_marketplace),
        )
        .route(
            "/api/p900/backfill-product-refs",
            post(handlers::p900_sales_register::backfill_product_refs),
        )
        // P901 Nomenclature Barcodes handlers
        .route(
            "/api/p901/barcode/:barcode",
            get(handlers::p901_barcodes::get_by_barcode),
        )
        .route(
            "/api/p901/nomenclature/:nomenclature_ref/barcodes",
            get(handlers::p901_barcodes::get_barcodes_by_nomenclature),
        )
        .route(
            "/api/p901/barcodes",
            get(handlers::p901_barcodes::list_barcodes),
        )
        // P902 OZON Finance Realization handlers
        .route(
            "/api/p902/finance-realization",
            get(handlers::p902_ozon_finance_realization::list_finance_realization),
        )
        .route(
            "/api/p902/finance-realization/:posting_number/:sku/:operation_type",
            get(handlers::p902_ozon_finance_realization::get_finance_realization_detail),
        )
        .route(
            "/api/p902/stats",
            get(handlers::p902_ozon_finance_realization::get_stats),
        )
        // P903 WB Finance Report handlers
        .route(
            "/api/p903/finance-report",
            get(handlers::p903_wb_finance_report::list_reports),
        )
        .route(
            "/api/p903/finance-report/search-by-srid",
            get(handlers::p903_wb_finance_report::search_by_srid),
        )
        .route(
            "/api/p903/finance-report/:rr_dt/:rrd_id",
            get(handlers::p903_wb_finance_report::get_report_detail),
        )
        .route(
            "/api/p903/finance-report/:rr_dt/:rrd_id/raw",
            get(handlers::p903_wb_finance_report::get_raw_json),
        )
        // P904 Sales Data handlers
        .route(
            "/api/p904/sales-data",
            get(handlers::p904_sales_data::list),
        )
        // D400 Monthly Summary Dashboard
        .route(
            "/api/d400/monthly_summary",
            get(handlers::d400_monthly_summary::get_monthly_summary),
        )
        // P905 WB Commission History handlers
        .route(
            "/api/p905-commission/list",
            get(handlers::p905_wb_commission_history::list_commissions),
        )
        .route(
            "/api/p905-commission/sync",
            post(handlers::p905_wb_commission_history::sync_commissions),
        )
        .route(
            "/api/p905-commission/:id",
            get(handlers::p905_wb_commission_history::get_commission)
                .put(handlers::p905_wb_commission_history::save_commission)
                .delete(handlers::p905_wb_commission_history::delete_commission),
        )
        .route(
            "/api/p905-commission",
            post(handlers::p905_wb_commission_history::save_commission),
        )
        // P906 Nomenclature Prices handlers
        .route(
            "/api/p906/nomenclature-prices",
            get(handlers::p906_nomenclature_prices::list),
        )
        .route(
            "/api/p906/periods",
            get(handlers::p906_nomenclature_prices::get_periods),
        )
        // Form Settings handlers
        .route(
            "/api/form-settings/:form_key",
            get(handlers::form_settings::get_settings),
        )
        .route(
            "/api/form-settings",
            post(handlers::form_settings::save_settings),
        )
        // A009 OZON Returns handlers
        .route(
            "/api/a009/ozon-returns/:id/post",
            post(handlers::a009_ozon_returns::post_ozon_return),
        )
        .route(
            "/api/a009/ozon-returns/:id/unpost",
            post(handlers::a009_ozon_returns::unpost_ozon_return),
        )
        // A010 OZON FBS Posting handlers
        .route(
            "/api/a010/ozon-fbs-posting",
            get(handlers::a010_ozon_fbs_posting::list_postings),
        )
        .route(
            "/api/a010/ozon-fbs-posting/:id",
            get(handlers::a010_ozon_fbs_posting::get_posting_detail),
        )
        .route(
            "/api/a010/raw/:ref_id",
            get(handlers::a010_ozon_fbs_posting::get_raw_json),
        )
        .route(
            "/api/a010/ozon-fbs-posting/:id/post",
            post(handlers::a010_ozon_fbs_posting::post_document),
        )
        .route(
            "/api/a010/ozon-fbs-posting/:id/unpost",
            post(handlers::a010_ozon_fbs_posting::unpost_document),
        )
        .route(
            "/api/a010/ozon-fbs-posting/post-period",
            post(handlers::a010_ozon_fbs_posting::post_period),
        )
        // A011 OZON FBO Posting handlers
        .route(
            "/api/a011/ozon-fbo-posting",
            get(handlers::a011_ozon_fbo_posting::list_postings),
        )
        .route(
            "/api/a011/ozon-fbo-posting/:id",
            get(handlers::a011_ozon_fbo_posting::get_posting_detail),
        )
        .route(
            "/api/a011/ozon-fbo-posting/:id/post",
            post(handlers::a011_ozon_fbo_posting::post_document),
        )
        .route(
            "/api/a011/ozon-fbo-posting/:id/unpost",
            post(handlers::a011_ozon_fbo_posting::unpost_document),
        )
        .route(
            "/api/a011/ozon-fbo-posting/post-period",
            post(handlers::a011_ozon_fbo_posting::post_period),
        )
        // A012 WB Sales handlers
        .route(
            "/api/a012/wb-sales",
            get(handlers::a012_wb_sales::list_sales),
        )
        .route(
            "/api/a012/wb-sales/:id",
            get(handlers::a012_wb_sales::get_sale_detail),
        )
        .route(
            "/api/a012/wb-sales/search-by-srid",
            get(handlers::a012_wb_sales::search_by_srid),
        )
        .route(
            "/api/a012/raw/:ref_id",
            get(handlers::a012_wb_sales::get_raw_json),
        )
        .route(
            "/api/a012/wb-sales/:id/post",
            post(handlers::a012_wb_sales::post_document),
        )
        .route(
            "/api/a012/wb-sales/:id/unpost",
            post(handlers::a012_wb_sales::unpost_document),
        )
        .route(
            "/api/a012/wb-sales/post-period",
            post(handlers::a012_wb_sales::post_period),
        )
        .route(
            "/api/a012/wb-sales/batch-post",
            post(handlers::a012_wb_sales::batch_post_documents),
        )
        .route(
            "/api/a012/wb-sales/batch-unpost",
            post(handlers::a012_wb_sales::batch_unpost_documents),
        )
        .route(
            "/api/a012/wb-sales/:id/projections",
            get(handlers::a012_wb_sales::get_projections),
        )
        .route(
            "/api/a012/wb-sales/migrate-sale-id",
            post(handlers::a012_wb_sales::migrate_fill_sale_id),
        )
        // A013 YM Order handlers
        .route(
            "/api/a013/ym-order",
            get(handlers::a013_ym_order::list_orders),
        )
        .route(
            "/api/a013/ym-order/list",
            get(handlers::a013_ym_order::list_orders_fast),
        )
        .route(
            "/api/a013/ym-order/:id",
            get(handlers::a013_ym_order::get_order_detail),
        )
        .route(
            "/api/a013/raw/:ref_id",
            get(handlers::a013_ym_order::get_raw_json),
        )
        .route(
            "/api/a013/ym-order/:id/post",
            post(handlers::a013_ym_order::post_document),
        )
        .route(
            "/api/a013/ym-order/:id/unpost",
            post(handlers::a013_ym_order::unpost_document),
        )
        .route(
            "/api/a013/ym-order/post-period",
            post(handlers::a013_ym_order::post_period),
        )
        .route(
            "/api/a013/ym-order/batch-post",
            post(handlers::a013_ym_order::batch_post_documents),
        )
        .route(
            "/api/a013/ym-order/batch-unpost",
            post(handlers::a013_ym_order::batch_unpost_documents),
        )
        // A016 YM Returns handlers
        .route(
            "/api/a016/ym-returns",
            get(handlers::a016_ym_returns::list_returns),
        )
        .route(
            "/api/a016/ym-returns/:id",
            get(handlers::a016_ym_returns::get_return_detail),
        )
        .route(
            "/api/a016/raw/:ref_id",
            get(handlers::a016_ym_returns::get_raw_json),
        )
        .route(
            "/api/a016/ym-returns/:id/post",
            post(handlers::a016_ym_returns::post_document),
        )
        .route(
            "/api/a016/ym-returns/:id/unpost",
            post(handlers::a016_ym_returns::unpost_document),
        )
        .route(
            "/api/a016/ym-returns/post-period",
            post(handlers::a016_ym_returns::post_period),
        )
        // A015 WB Orders handlers
        .route(
            "/api/a015/wb-orders",
            get(handlers::a015_wb_orders::list_orders),
        )
        .route(
            "/api/a015/wb-orders/:id",
            get(handlers::a015_wb_orders::get_order_detail),
        )
        .route(
            "/api/a015/wb-orders/search-by-srid",
            get(handlers::a015_wb_orders::search_by_srid),
        )
        .route(
            "/api/a015/raw/:ref_id",
            get(handlers::a015_wb_orders::get_raw_json),
        )
        .route(
            "/api/a015/wb-orders/:id/delete",
            post(handlers::a015_wb_orders::delete_order),
        )
        .route(
            "/api/a015/wb-orders/:id/post",
            post(handlers::a015_wb_orders::post_order),
        )
        .route(
            "/api/a015/wb-orders/:id/unpost",
            post(handlers::a015_wb_orders::unpost_order),
        )
        // P900 Sales Register - get projections by registrator
        .route(
            "/api/projections/p900/:registrator_ref",
            get(handlers::p900_sales_register::get_by_registrator),
        )
        .fallback_service(ServeDir::new("dist"))
        .layer(middleware::from_fn(request_logger))
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
