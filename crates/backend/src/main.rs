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
pub mod shared;
pub mod system;
pub mod usecases;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use axum::body::Body;
    use axum::http::{header, Method, Request};
    use axum::middleware::{self, Next};
    use axum::response::Response;
    use axum::{
        routing::{get, post},
        Router,
    };
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

    // Initialize database (loads config from config.toml)
    shared::data::db::initialize_database()
        .await
        .map_err(|e| anyhow::anyhow!("db init failed: {e}"))?;

    // Apply auth system migration
    system::initialization::apply_auth_migration().await?;

    // Ensure admin user exists
    system::initialization::ensure_admin_user_exists().await?;

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

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        // ========================================
        // SYSTEM AUTH ROUTES (PUBLIC)
        // ========================================
        .route(
            "/api/system/auth/login",
            post(system::handlers::auth::login),
        )
        .route(
            "/api/system/auth/refresh",
            post(system::handlers::auth::refresh),
        )
        .route(
            "/api/system/auth/logout",
            post(system::handlers::auth::logout),
        )
        // System auth routes (protected)
        .route(
            "/api/system/auth/me",
            get(system::handlers::auth::current_user)
                .layer(middleware::from_fn(system::auth::middleware::require_auth)),
        )
        // System users management (admin only)
        .route(
            "/api/system/users",
            get(system::handlers::users::list)
                .post(system::handlers::users::create)
                .layer(middleware::from_fn(system::auth::middleware::require_admin)),
        )
        .route(
            "/api/system/users/:id",
            get(system::handlers::users::get_by_id)
                .put(system::handlers::users::update)
                .delete(system::handlers::users::delete)
                .layer(middleware::from_fn(system::auth::middleware::require_admin)),
        )
        .route(
            "/api/system/users/:id/change-password",
            post(system::handlers::users::change_password)
                .layer(middleware::from_fn(system::auth::middleware::require_auth)),
        )
        // ========================================
        // BUSINESS ROUTES (existing, without auth for now)
        // ========================================
        .route(
            "/api/connection_1c",
            get(handlers::a001_connection_1c::list_all).post(handlers::a001_connection_1c::upsert),
        )
        .route(
            "/api/connection_1c/list",
            get(handlers::a001_connection_1c::list_paginated),
        )
        .route(
            "/api/connection_1c/:id",
            get(handlers::a001_connection_1c::get_by_id),
        )
        .route(
            "/api/connection_1c/test",
            post(handlers::a001_connection_1c::test_connection),
        )
        .route(
            "/api/connection_1c/testdata",
            post(handlers::a001_connection_1c::insert_test_data),
        )
        .route(
            "/api/organization",
            get(handlers::a002_organization::list_all).post(handlers::a002_organization::upsert),
        )
        .route(
            "/api/organization/:id",
            get(handlers::a002_organization::get_by_id).delete(handlers::a002_organization::delete),
        )
        .route(
            "/api/organization/testdata",
            post(handlers::a002_organization::insert_test_data),
        )
        // Counterparty handlers
        .route(
            "/api/counterparty",
            get(handlers::a003_counterparty::list_all).post(handlers::a003_counterparty::upsert),
        )
        .route(
            "/api/counterparty/:id",
            get(handlers::a003_counterparty::get_by_id).delete(handlers::a003_counterparty::delete),
        )
        // Nomenclature handlers
        .route(
            "/api/nomenclature",
            get(handlers::a004_nomenclature::list_all).post(handlers::a004_nomenclature::upsert),
        )
        .route(
            "/api/nomenclature/:id",
            get(handlers::a004_nomenclature::get_by_id).delete(handlers::a004_nomenclature::delete),
        )
        .route(
            "/api/nomenclature/import-excel",
            post(handlers::a004_nomenclature::import_excel),
        )
        .route(
            "/api/nomenclature/dimensions",
            get(handlers::a004_nomenclature::get_dimensions),
        )
        .route(
            "/api/nomenclature/search",
            get(handlers::a004_nomenclature::search_by_article),
        )
        // Marketplace handlers
        .route(
            "/api/marketplace",
            get(handlers::a005_marketplace::list_all).post(handlers::a005_marketplace::upsert),
        )
        .route(
            "/api/marketplace/:id",
            get(handlers::a005_marketplace::get_by_id).delete(handlers::a005_marketplace::delete),
        )
        .route(
            "/api/marketplace/testdata",
            post(handlers::a005_marketplace::insert_test_data),
        )
        // Connection MP handlers
        .route(
            "/api/connection_mp",
            get(handlers::a006_connection_mp::list_all).post(handlers::a006_connection_mp::upsert),
        )
        .route(
            "/api/connection_mp/:id",
            get(handlers::a006_connection_mp::get_by_id)
                .delete(handlers::a006_connection_mp::delete),
        )
        .route(
            "/api/connection_mp/test",
            post(handlers::a006_connection_mp::test_connection),
        )
        // Marketplace product handlers
        .route(
            "/api/marketplace_product",
            get(handlers::a007_marketplace_product::list_all)
                .post(handlers::a007_marketplace_product::upsert),
        )
        .route(
            "/api/marketplace_product/:id",
            get(handlers::a007_marketplace_product::get_by_id)
                .delete(handlers::a007_marketplace_product::delete),
        )
        .route(
            "/api/marketplace_product/testdata",
            post(handlers::a007_marketplace_product::insert_test_data),
        )
        // Marketplace sales handlers
        .route(
            "/api/marketplace_sales",
            get(handlers::a008_marketplace_sales::list_all)
                .post(handlers::a008_marketplace_sales::upsert),
        )
        .route(
            "/api/marketplace_sales/:id",
            get(handlers::a008_marketplace_sales::get_by_id)
                .delete(handlers::a008_marketplace_sales::delete),
        )
        // OZON Returns handlers
        .route(
            "/api/ozon_returns",
            get(handlers::a009_ozon_returns::list_all).post(handlers::a009_ozon_returns::upsert),
        )
        .route(
            "/api/ozon_returns/:id",
            get(handlers::a009_ozon_returns::get_by_id).delete(handlers::a009_ozon_returns::delete),
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
        .route(
            "/api/u501/import/start",
            post(handlers::usecases::u501_start_import),
        )
        .route(
            "/api/u501/import/:session_id/progress",
            get(handlers::usecases::u501_get_progress),
        )
        // UseCase u502: Import from OZON
        .route(
            "/api/u502/import/start",
            post(handlers::usecases::u502_start_import),
        )
        .route(
            "/api/u502/import/:session_id/progress",
            get(handlers::usecases::u502_get_progress),
        )
        // UseCase u503: Import from Yandex Market
        .route(
            "/api/u503/import/start",
            post(handlers::usecases::u503_start_import),
        )
        .route(
            "/api/u503/import/:session_id/progress",
            get(handlers::usecases::u503_get_progress),
        )
        // UseCase u504: Import from Wildberries
        .route(
            "/api/u504/import/start",
            post(handlers::usecases::u504_start_import),
        )
        .route(
            "/api/u504/import/:session_id/progress",
            get(handlers::usecases::u504_get_progress),
        )
        // UseCase u505: Match Nomenclature
        .route(
            "/api/u505/match/start",
            post(handlers::usecases::u505_start_matching),
        )
        .route(
            "/api/u505/match/:session_id/progress",
            get(handlers::usecases::u505_get_progress),
        )
        // UseCase u506: Import from LemanaPro
        .route(
            "/api/u506/import/start",
            post(handlers::usecases::u506_start_import),
        )
        .route(
            "/api/u506/import/:session_id/progress",
            get(handlers::usecases::u506_get_progress),
        )
        // Logs handlers
        .route(
            "/api/logs",
            get(handlers::logs::list_all)
                .post(handlers::logs::create)
                .delete(handlers::logs::clear_all),
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
        .route("/api/p904/sales-data", get(handlers::p904_sales_data::list))
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
            "/api/a013/ym-order/:id/projections",
            get(handlers::a013_ym_order::get_projections),
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
            "/api/a016/ym-returns/:id/projections",
            get(handlers::a016_ym_returns::get_projections),
        )
        .route(
            "/api/a016/ym-returns/post-period",
            post(handlers::a016_ym_returns::post_period),
        )
        .route(
            "/api/a016/ym-returns/batch-post",
            post(handlers::a016_ym_returns::batch_post_documents),
        )
        .route(
            "/api/a016/ym-returns/batch-unpost",
            post(handlers::a016_ym_returns::batch_unpost_documents),
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
