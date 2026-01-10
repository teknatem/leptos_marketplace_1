use axum::{
    middleware,
    routing::{get, post},
    Router,
};

use crate::{handlers, system};

/// Конфигурация всех роутов приложения
pub fn configure_routes() -> Router {
    Router::new()
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
        // A001 Connection 1C handlers
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
            get(handlers::a001_connection_1c::get_by_id)
                .delete(handlers::a001_connection_1c::delete),
        )
        .route(
            "/api/connection_1c/test",
            post(handlers::a001_connection_1c::test_connection),
        )
        .route(
            "/api/connection_1c/testdata",
            post(handlers::a001_connection_1c::insert_test_data),
        )
        // A002 Organization handlers
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
        // A003 Counterparty handlers
        .route(
            "/api/counterparty",
            get(handlers::a003_counterparty::list_all).post(handlers::a003_counterparty::upsert),
        )
        .route(
            "/api/counterparty/:id",
            get(handlers::a003_counterparty::get_by_id).delete(handlers::a003_counterparty::delete),
        )
        // A004 Nomenclature handlers
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
        .route(
            "/api/a004/nomenclature",
            get(handlers::a004_nomenclature::list_paginated),
        )
        // A005 Marketplace handlers
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
        // A006 Connection MP handlers
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
        // A007 Marketplace product handlers
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
        .route(
            "/api/a007/marketplace-product",
            get(handlers::a007_marketplace_product::list_paginated),
        )
        // A008 Marketplace sales handlers
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
        // A009 OZON Returns handlers
        .route(
            "/api/ozon_returns",
            get(handlers::a009_ozon_returns::list_all).post(handlers::a009_ozon_returns::upsert),
        )
        .route(
            "/api/ozon_returns/:id",
            get(handlers::a009_ozon_returns::get_by_id).delete(handlers::a009_ozon_returns::delete),
        )
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
        // A014 OZON Transactions handlers
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
        // ========================================
        // USECASES
        // ========================================
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
        // ========================================
        // SYSTEM SCHEDULED TASKS ROUTES
        // ========================================
        .route(
            "/api/sys/scheduled_tasks",
            get(handlers::sys_scheduled_task::list_scheduled_tasks)
                .post(handlers::sys_scheduled_task::create_scheduled_task)
                .layer(middleware::from_fn(system::auth::middleware::require_admin)),
        )
        .route(
            "/api/sys/scheduled_tasks/:id",
            get(handlers::sys_scheduled_task::get_scheduled_task)
                .put(handlers::sys_scheduled_task::update_scheduled_task)
                .delete(handlers::sys_scheduled_task::delete_scheduled_task)
                .layer(middleware::from_fn(system::auth::middleware::require_admin)),
        )
        .route(
            "/api/sys/scheduled_tasks/:id/toggle_enabled",
            post(handlers::sys_scheduled_task::toggle_scheduled_task_enabled)
                .layer(middleware::from_fn(system::auth::middleware::require_admin)),
        )
        .route(
            "/api/sys/scheduled_tasks/:id/progress/:session_id",
            get(handlers::sys_scheduled_task::get_task_progress)
                .layer(middleware::from_fn(system::auth::middleware::require_auth)),
        )
        .route(
            "/api/sys/scheduled_tasks/:id/log/:session_id",
            get(handlers::sys_scheduled_task::get_task_log)
                .layer(middleware::from_fn(system::auth::middleware::require_auth)),
        )
        // ========================================
        // PROJECTIONS
        // ========================================
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
        .route(
            "/api/projections/p900/:registrator_ref",
            get(handlers::p900_sales_register::get_by_registrator),
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
        // ========================================
        // DASHBOARDS
        // ========================================
        // D400 Monthly Summary Dashboard
        .route(
            "/api/d400/monthly_summary",
            get(handlers::d400_monthly_summary::get_monthly_summary),
        )
        // ========================================
        // UTILITIES
        // ========================================
        // Logs handlers
        .route(
            "/api/logs",
            get(handlers::logs::list_all)
                .post(handlers::logs::create)
                .delete(handlers::logs::clear_all),
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
}
