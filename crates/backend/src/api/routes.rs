use axum::{
    body::Body,
    extract::Request,
    middleware::{self, Next},
    routing::{get, post, put},
    Router,
};

use super::handlers;
use crate::system::auth::middleware::{check_scope, check_scope_read, require_auth};

/// Business routes configuration.
/// Each aggregate group is wrapped with require_scope_auto for its scope.
/// Projections, usecases, and dashboards require authentication but no scope.
pub fn configure_business_routes() -> Router {
    Router::new()
        .merge(a001_routes())
        .merge(a002_routes())
        .merge(a003_routes())
        .merge(a004_routes())
        .merge(a005_routes())
        .merge(a006_routes())
        .merge(a007_routes())
        .merge(a008_routes())
        .merge(a009_routes())
        .merge(a010_routes())
        .merge(a011_routes())
        .merge(a012_routes())
        .merge(a013_routes())
        .merge(a014_routes())
        .merge(a015_routes())
        .merge(a016_routes())
        .merge(a017_routes())
        .merge(a018_routes())
        .merge(a019_routes())
        .merge(a020_routes())
        .merge(a021_routes())
        .merge(a022_routes())
        .merge(a023_routes())
        .merge(a024_routes())
        .merge(a025_routes())
        .merge(a026_routes())
        .merge(a027_routes())
        .merge(a028_routes())
        .merge(a029_routes())
        .merge(usecase_routes())
        .merge(projection_routes())
        .merge(dashboard_routes())
        .merge(data_view_routes())
        .merge(misc_routes())
}

// ============================================================================
// Aggregates A001–A025 (each wrapped with require_scope_auto)
// ============================================================================

fn a001_routes() -> Router {
    Router::new()
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
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a001_connection_1c", req, next).await
            },
        ))
}

fn a002_routes() -> Router {
    Router::new()
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
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a002_organization", req, next).await
            },
        ))
}

fn a003_routes() -> Router {
    Router::new()
        .route(
            "/api/counterparty",
            get(handlers::a003_counterparty::list_all).post(handlers::a003_counterparty::upsert),
        )
        .route(
            "/api/counterparty/:id",
            get(handlers::a003_counterparty::get_by_id).delete(handlers::a003_counterparty::delete),
        )
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a003_counterparty", req, next).await
            },
        ))
}

fn a004_routes() -> Router {
    Router::new()
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
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a004_nomenclature", req, next).await
            },
        ))
}

fn a005_routes() -> Router {
    Router::new()
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
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a005_marketplace", req, next).await
            },
        ))
}

fn a006_routes() -> Router {
    Router::new()
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
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a006_connection_mp", req, next).await
            },
        ))
}

fn a007_routes() -> Router {
    Router::new()
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
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a007_marketplace_product", req, next).await
            },
        ))
}

fn a008_routes() -> Router {
    Router::new()
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
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a008_marketplace_sales", req, next).await
            },
        ))
}

fn a009_routes() -> Router {
    Router::new()
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
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a009_ozon_returns", req, next).await
            },
        ))
}

fn a010_routes() -> Router {
    Router::new()
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
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a010_ozon_fbs_posting", req, next).await
            },
        ))
}

fn a011_routes() -> Router {
    Router::new()
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
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a011_ozon_fbo_posting", req, next).await
            },
        ))
}

fn a012_routes() -> Router {
    Router::new()
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
            "/api/a012/wb-sales/:id/journal",
            get(handlers::a012_wb_sales::get_general_ledger_entries),
        )
        .route(
            "/api/a012/wb-sales/:id/refresh-dealer-price",
            post(handlers::a012_wb_sales::refresh_dealer_price),
        )
        .route(
            "/api/a012/wb-sales/migrate-sale-id",
            post(handlers::a012_wb_sales::migrate_fill_sale_id),
        )
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a012_wb_sales", req, next).await
            },
        ))
}

fn a013_routes() -> Router {
    Router::new()
        .route(
            "/api/a013/ym-order",
            get(handlers::a013_ym_order::list_orders_fast),
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
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a013_ym_order", req, next).await
            },
        ))
}

fn a014_routes() -> Router {
    Router::new()
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
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a014_ozon_transactions", req, next).await
            },
        ))
}

fn a015_routes() -> Router {
    Router::new()
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
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a015_wb_orders", req, next).await
            },
        ))
}

fn a016_routes() -> Router {
    Router::new()
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
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a016_ym_returns", req, next).await
            },
        ))
}

fn a017_routes() -> Router {
    Router::new()
        .route(
            "/api/a017-llm-agent",
            get(handlers::a017_llm_agent::list_all).post(handlers::a017_llm_agent::upsert),
        )
        .route(
            "/api/a017-llm-agent/list",
            get(handlers::a017_llm_agent::list_paginated),
        )
        .route(
            "/api/a017-llm-agent/primary",
            get(handlers::a017_llm_agent::get_primary),
        )
        .route(
            "/api/a017-llm-agent/:id",
            get(handlers::a017_llm_agent::get_by_id).delete(handlers::a017_llm_agent::delete),
        )
        .route(
            "/api/a017-llm-agent/:id/test",
            post(handlers::a017_llm_agent::test_connection),
        )
        .route(
            "/api/a017-llm-agent/:id/fetch-models",
            post(handlers::a017_llm_agent::fetch_models),
        )
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a017_llm_agent", req, next).await
            },
        ))
}

fn a018_routes() -> Router {
    Router::new()
        .route(
            "/api/a018-llm-chat",
            get(handlers::a018_llm_chat::list_all).post(handlers::a018_llm_chat::upsert),
        )
        .route(
            "/api/a018-llm-chat/with-stats",
            get(handlers::a018_llm_chat::list_with_stats),
        )
        .route(
            "/api/a018-llm-chat/list",
            get(handlers::a018_llm_chat::list_paginated),
        )
        .route(
            "/api/a018-llm-chat/jobs/:job_id",
            get(handlers::a018_llm_chat::poll_job),
        )
        .route(
            "/api/a018-llm-chat/:id",
            get(handlers::a018_llm_chat::get_by_id).delete(handlers::a018_llm_chat::delete),
        )
        .route(
            "/api/a018-llm-chat/:id/messages",
            get(handlers::a018_llm_chat::get_messages).post(handlers::a018_llm_chat::send_message),
        )
        .route(
            "/api/a018-llm-chat/:id/upload",
            post(handlers::a018_llm_chat::upload_attachment),
        )
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a018_llm_chat", req, next).await
            },
        ))
}

fn a019_routes() -> Router {
    Router::new()
        .route(
            "/api/a019-llm-artifact",
            get(handlers::a019_llm_artifact::list_all).post(handlers::a019_llm_artifact::upsert),
        )
        .route(
            "/api/a019-llm-artifact/list",
            get(handlers::a019_llm_artifact::list_paginated),
        )
        .route(
            "/api/a019-llm-artifact/chat/:chat_id",
            get(handlers::a019_llm_artifact::list_by_chat),
        )
        .route(
            "/api/a019-llm-artifact/:id",
            get(handlers::a019_llm_artifact::get_by_id).delete(handlers::a019_llm_artifact::delete),
        )
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a019_llm_artifact", req, next).await
            },
        ))
}

fn a020_routes() -> Router {
    Router::new()
        .route(
            "/api/a020/wb-promotions",
            get(handlers::a020_wb_promotion::list_promotions),
        )
        .route(
            "/api/a020/wb-promotions/:id",
            get(handlers::a020_wb_promotion::get_promotion_detail),
        )
        .route(
            "/api/a020/wb-promotions/:id/post",
            post(handlers::a020_wb_promotion::post_promotion),
        )
        .route(
            "/api/a020/wb-promotions/:id/unpost",
            post(handlers::a020_wb_promotion::unpost_promotion),
        )
        .route(
            "/api/a020/raw/:ref_id",
            get(handlers::a020_wb_promotion::get_raw_json),
        )
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a020_wb_promotion", req, next).await
            },
        ))
}

fn a021_routes() -> Router {
    Router::new()
        .route(
            "/api/a021/production-output/list",
            get(handlers::a021_production_output::list_paginated),
        )
        .route(
            "/api/a021/production-output/:id",
            get(handlers::a021_production_output::get_by_id),
        )
        .route(
            "/api/a021/production-output/:id/post",
            post(handlers::a021_production_output::post_production_output),
        )
        .route(
            "/api/a021/production-output/:id/unpost",
            post(handlers::a021_production_output::unpost_production_output),
        )
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a021_production_output", req, next).await
            },
        ))
}

fn a022_routes() -> Router {
    Router::new()
        .route(
            "/api/a022/kit-variant/list",
            get(handlers::a022_kit_variant::list_paginated),
        )
        .route(
            "/api/a022/kit-variant/:id",
            get(handlers::a022_kit_variant::get_by_id),
        )
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a022_kit_variant", req, next).await
            },
        ))
}

fn a026_routes() -> Router {
    Router::new()
        .route(
            "/api/a026/wb-advert-daily/list",
            get(handlers::a026_wb_advert_daily::list_paginated),
        )
        .route(
            "/api/a026/wb-advert-daily/:id",
            get(handlers::a026_wb_advert_daily::get_by_id),
        )
        .route(
            "/api/a026/wb-advert-daily/:id/post",
            post(handlers::a026_wb_advert_daily::post_document),
        )
        .route(
            "/api/a026/wb-advert-daily/:id/unpost",
            post(handlers::a026_wb_advert_daily::unpost_document),
        )
        .route(
            "/api/a026/wb-advert-daily/:id/journal",
            get(handlers::a026_wb_advert_daily::get_general_ledger_entries),
        )
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a026_wb_advert_daily", req, next).await
            },
        ))
}

fn a023_routes() -> Router {
    Router::new()
        .route(
            "/api/a023/purchase-of-goods/list",
            get(handlers::a023_purchase_of_goods::list_paginated),
        )
        .route(
            "/api/a023/purchase-of-goods/:id",
            get(handlers::a023_purchase_of_goods::get_by_id),
        )
        .route(
            "/api/a023/purchase-of-goods/:id/post",
            post(handlers::a023_purchase_of_goods::post_purchase_of_goods),
        )
        .route(
            "/api/a023/purchase-of-goods/:id/unpost",
            post(handlers::a023_purchase_of_goods::unpost_purchase_of_goods),
        )
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a023_purchase_of_goods", req, next).await
            },
        ))
}

fn a024_routes() -> Router {
    // Write routes: upsert, delete, testdata, generate-view — require "all" access.
    let write_routes = Router::new()
        .route(
            "/api/a024-bi-indicator",
            get(handlers::a024_bi_indicator::list_all).post(handlers::a024_bi_indicator::upsert),
        )
        .route(
            "/api/a024-bi-indicator/upsert",
            post(handlers::a024_bi_indicator::upsert),
        )
        .route(
            "/api/a024-bi-indicator/list",
            get(handlers::a024_bi_indicator::list_paginated),
        )
        .route(
            "/api/a024-bi-indicator/public",
            get(handlers::a024_bi_indicator::list_public),
        )
        .route(
            "/api/a024-bi-indicator/owner/:user_id",
            get(handlers::a024_bi_indicator::list_by_owner),
        )
        .route(
            "/api/a024-bi-indicator/testdata",
            post(handlers::a024_bi_indicator::insert_test_data),
        )
        .route(
            "/api/a024-bi-indicator/generate-view",
            post(handlers::a024_bi_indicator::generate_view),
        )
        .route(
            "/api/a024-bi-indicator/:id",
            get(handlers::a024_bi_indicator::get_by_id).delete(handlers::a024_bi_indicator::delete),
        )
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a024_bi_indicator", req, next).await
            },
        ));

    // Read-only compute routes: these POST endpoints only compute/query data,
    // they never mutate state — "read" access is sufficient.
    let compute_routes = Router::new()
        .route(
            "/api/a024-bi-indicator/resolve-batch",
            post(handlers::a024_bi_indicator::resolve_batch),
        )
        .route(
            "/api/a024-bi-indicator/:id/compute",
            post(handlers::a024_bi_indicator::compute),
        )
        .route(
            "/api/a024-bi-indicator/compute-batch",
            post(handlers::a024_bi_indicator::compute_batch),
        )
        .route(
            "/api/a024-bi-indicator/:id/drilldown",
            get(handlers::a024_bi_indicator::drilldown),
        )
        .route(
            "/api/drilldown/execute",
            post(handlers::a024_bi_indicator::execute_drilldown),
        )
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope_read("a024_bi_indicator", req, next).await
            },
        ));

    write_routes.merge(compute_routes)
}

fn a025_routes() -> Router {
    Router::new()
        .route(
            "/api/a025-bi-dashboard",
            get(handlers::a025_bi_dashboard::list_all).post(handlers::a025_bi_dashboard::upsert),
        )
        .route(
            "/api/a025-bi-dashboard/upsert",
            post(handlers::a025_bi_dashboard::upsert),
        )
        .route(
            "/api/a025-bi-dashboard/list",
            get(handlers::a025_bi_dashboard::list_paginated),
        )
        .route(
            "/api/a025-bi-dashboard/public",
            get(handlers::a025_bi_dashboard::list_public),
        )
        .route(
            "/api/a025-bi-dashboard/testdata",
            post(handlers::a025_bi_dashboard::insert_test_data),
        )
        .route(
            "/api/a025-bi-dashboard/owner/:user_id",
            get(handlers::a025_bi_dashboard::list_by_owner),
        )
        .route(
            "/api/a025-bi-dashboard/:id",
            get(handlers::a025_bi_dashboard::get_by_id).delete(handlers::a025_bi_dashboard::delete),
        )
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a025_bi_dashboard", req, next).await
            },
        ))
}

fn a027_routes() -> Router {
    Router::new()
        .route(
            "/api/a027/wb-documents/list",
            get(handlers::a027_wb_documents::list_paginated),
        )
        .route(
            "/api/a027/wb-documents/:id",
            get(handlers::a027_wb_documents::get_by_id),
        )
        .route(
            "/api/a027/wb-documents/:id/manual",
            put(handlers::a027_wb_documents::update_manual_fields),
        )
        .route(
            "/api/a027/wb-documents/:id/download/:extension",
            get(handlers::a027_wb_documents::download_document),
        )
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a027_wb_documents", req, next).await
            },
        ))
}

fn a028_routes() -> Router {
    Router::new()
        .route(
            "/api/a028/missing-cost-registry/list",
            get(handlers::a028_missing_cost_registry::list_paginated),
        )
        .route(
            "/api/a028/missing-cost-registry/:id",
            get(handlers::a028_missing_cost_registry::get_by_id)
                .put(handlers::a028_missing_cost_registry::update_document),
        )
        .route(
            "/api/a028/missing-cost-registry/:id/post",
            post(handlers::a028_missing_cost_registry::post_document),
        )
        .route(
            "/api/a028/missing-cost-registry/:id/unpost",
            post(handlers::a028_missing_cost_registry::unpost_document),
        )
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a028_missing_cost_registry", req, next).await
            },
        ))
}

fn a029_routes() -> Router {
    Router::new()
        .route(
            "/api/a029/wb-supply",
            get(handlers::a029_wb_supply::list_supplies),
        )
        // Lookup by WB supply ID string (e.g. "WB-GI-32319994") — must be before :id
        .route(
            "/api/a029/wb-supply/by-wb-id/:wb_id",
            get(handlers::a029_wb_supply::get_supply_by_wb_id),
        )
        .route(
            "/api/a029/wb-supply/:id",
            get(handlers::a029_wb_supply::get_supply_detail),
        )
        .route(
            "/api/a029/wb-supply/:id/orders",
            get(handlers::a029_wb_supply::get_supply_orders),
        )
        .route(
            "/api/a029/wb-supply/:id/stickers",
            get(handlers::a029_wb_supply::get_supply_stickers),
        )
        .route(
            "/api/a029/raw/:ref_id",
            get(handlers::a029_wb_supply::get_raw_json),
        )
        .route(
            "/api/a029/wb-supply/:id/delete",
            post(handlers::a029_wb_supply::delete_supply),
        )
        .layer(middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                check_scope("a029_wb_supply", req, next).await
            },
        ))
}

// ============================================================================
// Use Cases (U501–U508) — require auth, no scope check in Phase 1
// ============================================================================

fn usecase_routes() -> Router {
    Router::new()
        .route(
            "/api/u501/import/start",
            post(handlers::usecases::u501_start_import),
        )
        .route(
            "/api/u501/import/:session_id/progress",
            get(handlers::usecases::u501_get_progress),
        )
        .route(
            "/api/u502/import/start",
            post(handlers::usecases::u502_start_import),
        )
        .route(
            "/api/u502/import/:session_id/progress",
            get(handlers::usecases::u502_get_progress),
        )
        .route(
            "/api/u503/import/start",
            post(handlers::usecases::u503_start_import),
        )
        .route(
            "/api/u503/import/:session_id/progress",
            get(handlers::usecases::u503_get_progress),
        )
        .route(
            "/api/u504/import/start",
            post(handlers::usecases::u504_start_import),
        )
        .route(
            "/api/u504/import/:session_id/progress",
            get(handlers::usecases::u504_get_progress),
        )
        .route(
            "/api/u505/match/start",
            post(handlers::usecases::u505_start_matching),
        )
        .route(
            "/api/u505/match/:session_id/progress",
            get(handlers::usecases::u505_get_progress),
        )
        .route(
            "/api/u506/import/start",
            post(handlers::usecases::u506_start_import),
        )
        .route(
            "/api/u506/import/:session_id/progress",
            get(handlers::usecases::u506_get_progress),
        )
        .route(
            "/api/u507/import/start",
            post(handlers::usecases::u507_start_import),
        )
        .route(
            "/api/u507/import/:session_id/progress",
            get(handlers::usecases::u507_get_progress),
        )
        .route(
            "/api/u508/repost/projections",
            get(handlers::usecases::u508_get_projections),
        )
        .route(
            "/api/u508/repost/aggregates",
            get(handlers::usecases::u508_get_aggregates),
        )
        .route(
            "/api/u508/repost/start",
            post(handlers::usecases::u508_start_repost),
        )
        .route(
            "/api/u508/repost/aggregate/start",
            post(handlers::usecases::u508_start_aggregate_repost),
        )
        .route(
            "/api/u508/repost/:session_id/progress",
            get(handlers::usecases::u508_get_progress),
        )
        .layer(middleware::from_fn(require_auth))
}

// ============================================================================
// Projections (P900–P908) — require auth, no scope check in Phase 1
// ============================================================================

fn projection_routes() -> Router {
    Router::new()
        // P900 Sales Register
        .route(
            "/api/p900/sales-register",
            get(handlers::p900_mp_sales_register::list_sales),
        )
        .route(
            "/api/p900/sales-register/:marketplace/:document_no/:line_id",
            get(handlers::p900_mp_sales_register::get_sale_detail),
        )
        .route(
            "/api/p900/stats/by-date",
            get(handlers::p900_mp_sales_register::get_stats_by_date),
        )
        .route(
            "/api/p900/stats/by-marketplace",
            get(handlers::p900_mp_sales_register::get_stats_by_marketplace),
        )
        .route(
            "/api/p900/backfill-product-refs",
            post(handlers::p900_mp_sales_register::backfill_product_refs),
        )
        .route(
            "/api/projections/p900/:registrator_ref",
            get(handlers::p900_mp_sales_register::get_by_registrator),
        )
        // P901 Nomenclature Barcodes
        .route(
            "/api/p901/barcode/:barcode",
            get(handlers::p901_nomenclature_barcodes::get_by_barcode),
        )
        .route(
            "/api/p901/nomenclature/:nomenclature_ref/barcodes",
            get(handlers::p901_nomenclature_barcodes::get_barcodes_by_nomenclature),
        )
        .route(
            "/api/p901/barcodes",
            get(handlers::p901_nomenclature_barcodes::list_barcodes),
        )
        // P902 OZON Finance Realization
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
        // P903 WB Finance Report
        .route(
            "/api/p903/finance-report",
            get(handlers::p903_wb_finance_report::list_reports),
        )
        .route(
            "/api/p903/finance-report/export",
            get(handlers::p903_wb_finance_report::export_reports),
        )
        .route(
            "/api/p903/finance-report/search-by-srid",
            get(handlers::p903_wb_finance_report::search_by_srid),
        )
        .route(
            "/api/p903/finance-report/operation-kinds",
            get(handlers::p903_wb_finance_report::list_operation_kinds),
        )
        .route(
            "/api/p903/finance-report/by-id/:id",
            get(handlers::p903_wb_finance_report::get_report_detail_by_id),
        )
        .route(
            "/api/p903/finance-report/by-id/:id/post",
            post(handlers::p903_wb_finance_report::post_report_by_id),
        )
        .route(
            "/api/p903/finance-report/by-id/:id/raw",
            get(handlers::p903_wb_finance_report::get_raw_json_by_id),
        )
        // P904 Sales Data
        .route("/api/p904/sales-data", get(handlers::p904_sales_data::list))
        // P905 WB Commission History
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
        // P906 Nomenclature Prices
        .route(
            "/api/p906/nomenclature-prices",
            get(handlers::p906_nomenclature_prices::list),
        )
        .route(
            "/api/p906/periods",
            get(handlers::p906_nomenclature_prices::get_periods),
        )
        .route(
            "/api/p906/import-excel",
            post(handlers::p906_nomenclature_prices::import_excel),
        )
        // P907 YM Payment Report
        .route(
            "/api/p907/payment-report",
            get(handlers::p907_ym_payment_report::list_reports),
        )
        .route(
            "/api/p907/payment-report/:record_key",
            get(handlers::p907_ym_payment_report::get_report),
        )
        // P908 WB Goods Prices
        .route(
            "/api/p908/goods-prices",
            get(handlers::p908_wb_goods_prices::list_goods_prices),
        )
        .route(
            "/api/p908/goods-prices/:nm_id",
            get(handlers::p908_wb_goods_prices::get_goods_price),
        )
        .route(
            "/api/p912/nomenclature-costs",
            get(handlers::p912_nomenclature_costs::list),
        )
        .layer(middleware::from_fn(require_auth))
}

// ============================================================================
// Indicators
// ============================================================================

// ============================================================================
// Dashboards (D400, DS01, DS02)
// ============================================================================

fn dashboard_routes() -> Router {
    Router::new()
        .route(
            "/api/d400/monthly_summary",
            get(handlers::d400_monthly_summary::get_monthly_summary),
        )
        .route(
            "/api/d400/periods",
            get(handlers::d400_monthly_summary::get_available_periods),
        )
        // Universal Dashboard API
        .route(
            "/api/universal-dashboard/execute",
            post(handlers::ds01_wb_finance_report::execute_dashboard),
        )
        .route(
            "/api/universal-dashboard/generate-sql",
            post(handlers::ds01_wb_finance_report::generate_sql),
        )
        .route(
            "/api/universal-dashboard/schemas",
            get(handlers::ds01_wb_finance_report::list_schemas),
        )
        .route(
            "/api/universal-dashboard/schemas/validate-all",
            post(handlers::ds01_wb_finance_report::validate_all_schemas),
        )
        .route(
            "/api/universal-dashboard/schemas/:id",
            get(handlers::ds01_wb_finance_report::get_schema),
        )
        .route(
            "/api/universal-dashboard/schemas/:id/validate",
            post(handlers::ds01_wb_finance_report::validate_schema),
        )
        .route(
            "/api/universal-dashboard/schemas/:schema_id/fields/:field_id/values",
            get(handlers::ds01_wb_finance_report::get_distinct_values),
        )
        .route(
            "/api/universal-dashboard/configs",
            get(handlers::ds01_wb_finance_report::list_configs)
                .post(handlers::ds01_wb_finance_report::save_config),
        )
        .route(
            "/api/universal-dashboard/configs/:id",
            get(handlers::ds01_wb_finance_report::get_config)
                .put(handlers::ds01_wb_finance_report::update_config)
                .delete(handlers::ds01_wb_finance_report::delete_config),
        )
        // DS01 WB Finance Report
        .route(
            "/api/ds01/execute",
            post(handlers::ds01_wb_finance_report::execute_dashboard),
        )
        .route(
            "/api/ds01/generate-sql",
            post(handlers::ds01_wb_finance_report::generate_sql),
        )
        .route(
            "/api/ds01/schemas",
            get(handlers::ds01_wb_finance_report::list_schemas),
        )
        .route(
            "/api/ds01/schemas/:id",
            get(handlers::ds01_wb_finance_report::get_schema),
        )
        .route(
            "/api/ds01/schemas/:schema_id/fields/:field_id/values",
            get(handlers::ds01_wb_finance_report::get_distinct_values),
        )
        .route(
            "/api/ds01/configs",
            get(handlers::ds01_wb_finance_report::list_configs)
                .post(handlers::ds01_wb_finance_report::save_config),
        )
        .route(
            "/api/ds01/configs/:id",
            get(handlers::ds01_wb_finance_report::get_config)
                .put(handlers::ds01_wb_finance_report::update_config)
                .delete(handlers::ds01_wb_finance_report::delete_config),
        )
        // Legacy D401 routes
        .route(
            "/api/d401/execute",
            post(handlers::ds01_wb_finance_report::execute_dashboard),
        )
        .route(
            "/api/d401/generate-sql",
            post(handlers::ds01_wb_finance_report::generate_sql),
        )
        .route(
            "/api/d401/schemas",
            get(handlers::ds01_wb_finance_report::list_schemas),
        )
        .route(
            "/api/d401/schemas/:id",
            get(handlers::ds01_wb_finance_report::get_schema),
        )
        .route(
            "/api/d401/schemas/:schema_id/fields/:field_id/values",
            get(handlers::ds01_wb_finance_report::get_distinct_values),
        )
        .route(
            "/api/d401/configs",
            get(handlers::ds01_wb_finance_report::list_configs)
                .post(handlers::ds01_wb_finance_report::save_config),
        )
        .route(
            "/api/d401/configs/:id",
            get(handlers::ds01_wb_finance_report::get_config)
                .put(handlers::ds01_wb_finance_report::update_config)
                .delete(handlers::ds01_wb_finance_report::delete_config),
        )
        // DS02 Sales Register routes
        .route(
            "/api/ds02/execute",
            post(handlers::ds02_mp_sales_register::execute_dashboard),
        )
        .route(
            "/api/ds02/generate-sql",
            post(handlers::ds02_mp_sales_register::generate_sql),
        )
        .route(
            "/api/ds02/schemas",
            get(handlers::ds02_mp_sales_register::list_schemas),
        )
        .route(
            "/api/ds02/schemas/:id",
            get(handlers::ds02_mp_sales_register::get_schema),
        )
        .route(
            "/api/ds02/schemas/:schema_id/fields/:field_id/values",
            get(handlers::ds02_mp_sales_register::get_distinct_values),
        )
        .route(
            "/api/ds02/configs",
            get(handlers::ds02_mp_sales_register::list_configs)
                .post(handlers::ds02_mp_sales_register::save_config),
        )
        .route(
            "/api/ds02/configs/:id",
            get(handlers::ds02_mp_sales_register::get_config)
                .put(handlers::ds02_mp_sales_register::update_config)
                .delete(handlers::ds02_mp_sales_register::delete_config),
        )
        // Legacy D402 routes
        .route(
            "/api/dashboards/d402/execute",
            post(handlers::ds02_mp_sales_register::execute_dashboard),
        )
        .route(
            "/api/dashboards/d402/generate-sql",
            post(handlers::ds02_mp_sales_register::generate_sql),
        )
        .route(
            "/api/dashboards/d402/schemas",
            get(handlers::ds02_mp_sales_register::list_schemas),
        )
        .route(
            "/api/dashboards/d402/schemas/:id",
            get(handlers::ds02_mp_sales_register::get_schema),
        )
        .route(
            "/api/dashboards/d402/schemas/:schema_id/fields/:field_id/values",
            get(handlers::ds02_mp_sales_register::get_distinct_values),
        )
        .route(
            "/api/dashboards/d402/configs",
            get(handlers::ds02_mp_sales_register::list_configs)
                .post(handlers::ds02_mp_sales_register::save_config),
        )
        .route(
            "/api/dashboards/d402/configs/:id",
            get(handlers::ds02_mp_sales_register::get_config)
                .put(handlers::ds02_mp_sales_register::update_config)
                .delete(handlers::ds02_mp_sales_register::delete_config),
        )
        .layer(middleware::from_fn(require_auth))
}

// ============================================================================
// DataView semantic layer + misc (sys-drilldown, debug)
// ============================================================================

fn data_view_routes() -> Router {
    Router::new()
        .route("/api/data-view", get(handlers::data_view::list))
        .route(
            "/api/data-view/filters",
            get(handlers::data_view::list_filters),
        )
        .route("/api/data-view/:id", get(handlers::data_view::get_by_id))
        .route(
            "/api/data-view/:id/filters",
            get(handlers::data_view::get_view_filters),
        )
        .route(
            "/api/data-view/:id/compute",
            axum::routing::post(handlers::data_view::compute),
        )
        .route(
            "/api/data-view/:id/drilldown",
            axum::routing::post(handlers::data_view::drilldown),
        )
        .route(
            "/api/data-view/:id/drilldown-capabilities",
            axum::routing::post(handlers::data_view::drilldown_capabilities),
        )
        .layer(middleware::from_fn(require_auth))
}

fn misc_routes() -> Router {
    Router::new()
        .route(
            "/api/llm-knowledge",
            axum::routing::get(handlers::llm_knowledge::list),
        )
        .route(
            "/api/llm-knowledge/:id",
            axum::routing::get(handlers::llm_knowledge::get_by_id),
        )
        // Общий журнал операций
        .route(
            "/api/general-ledger",
            axum::routing::get(handlers::general_ledger::list),
        )
        .route(
            "/api/general-ledger/turnovers",
            axum::routing::get(handlers::general_ledger::list_turnovers),
        )
        .route(
            "/api/general-ledger/report",
            axum::routing::post(handlers::general_ledger::report),
        )
        .route(
            "/api/general-ledger/account-view",
            axum::routing::post(handlers::general_ledger::account_view),
        )
        .route(
            "/api/reports/wb-weekly-reconciliation",
            axum::routing::get(handlers::general_ledger::wb_weekly_reconciliation),
        )
        .route(
            "/api/general-ledger/report/dimensions",
            axum::routing::get(handlers::general_ledger::report_dimensions),
        )
        .route(
            "/api/general-ledger/report/drilldown",
            axum::routing::post(handlers::general_ledger::report_drilldown),
        )
        .route(
            "/api/general-ledger/drilldown",
            axum::routing::post(handlers::general_ledger::create_drilldown_session),
        )
        .route(
            "/api/general-ledger/drilldown/:id",
            axum::routing::get(handlers::general_ledger::get_drilldown_session),
        )
        .route(
            "/api/general-ledger/drilldown/:id/data",
            axum::routing::get(handlers::general_ledger::get_drilldown_session_data),
        )
        .route(
            "/api/general-ledger/:id",
            axum::routing::get(handlers::general_ledger::get_by_id),
        )
        // Drilldown session store (sys_drilldown)
        .route(
            "/api/sys-drilldown",
            axum::routing::post(handlers::sys_drilldown::create),
        )
        .route(
            "/api/sys-drilldown/:id",
            axum::routing::get(handlers::sys_drilldown::get_by_id),
        )
        .route(
            "/api/sys-drilldown/:id/data",
            axum::routing::get(handlers::sys_drilldown::get_data),
        )
        .layer(middleware::from_fn(require_auth))
        // Debug endpoints (open, dev only)
        .route("/api/debug/tool-test", get(handlers::debug::tool_test))
}
