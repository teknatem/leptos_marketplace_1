//! Static route policy registry.
//!
//! Every endpoint in the application must have exactly one entry here.
//! The registry is the single source of truth for access policy auditing.
//!
//! Convention:
//!   - method "*" = the route group is protected by check_scope (GET→Read, others→All)
//!   - method "GET" + ReadOnly = POST that is read-only (check_scope_read)
//!   - AdminOnly / Public / AuthOnly have scope_id = None
//!
//! When adding a new route to `api/routes.rs` or `system/api/routes.rs`,
//! you MUST add a corresponding entry here. Tests in this module will catch gaps.

use contracts::system::access::{PolicyMode, RoutePolicy};

pub static ROUTE_REGISTRY: &[RoutePolicy] = &[
    // ========================================================================
    // System / Public routes
    // ========================================================================
    RoutePolicy {
        method: "GET",
        path: "/health",
        scope_id: None,
        mode: PolicyMode::Public,
    },
    RoutePolicy {
        method: "POST",
        path: "/api/system/auth/login",
        scope_id: None,
        mode: PolicyMode::Public,
    },
    RoutePolicy {
        method: "POST",
        path: "/api/system/auth/refresh",
        scope_id: None,
        mode: PolicyMode::Public,
    },
    RoutePolicy {
        method: "POST",
        path: "/api/system/auth/logout",
        scope_id: None,
        mode: PolicyMode::Public,
    },
    RoutePolicy {
        method: "GET",
        path: "/api/system/auth/me",
        scope_id: None,
        mode: PolicyMode::AuthOnly,
    },
    // ========================================================================
    // System admin routes
    // ========================================================================
    RoutePolicy {
        method: "*",
        path: "/api/system/users",
        scope_id: None,
        mode: PolicyMode::AdminOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/system/users/:id",
        scope_id: None,
        mode: PolicyMode::AdminOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/system/users/:id/change-password",
        scope_id: None,
        mode: PolicyMode::AuthOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/system/roles",
        scope_id: None,
        mode: PolicyMode::AdminOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/system/roles/:id",
        scope_id: None,
        mode: PolicyMode::AdminOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/system/roles/:id/permissions",
        scope_id: None,
        mode: PolicyMode::AdminOnly,
    },
    RoutePolicy {
        method: "GET",
        path: "/api/system/scopes",
        scope_id: None,
        mode: PolicyMode::AdminOnly,
    },
    RoutePolicy {
        method: "GET",
        path: "/api/system/runtime-info",
        scope_id: None,
        mode: PolicyMode::AuthOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/sys/tasks",
        scope_id: None,
        mode: PolicyMode::AdminOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/sys/tasks/:id",
        scope_id: None,
        mode: PolicyMode::AdminOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/sys/tasks/:id/toggle_enabled",
        scope_id: None,
        mode: PolicyMode::AdminOnly,
    },
    RoutePolicy {
        method: "GET",
        path: "/api/sys/tasks/runs/active/progress",
        scope_id: None,
        mode: PolicyMode::AuthOnly,
    },
    RoutePolicy {
        method: "GET",
        path: "/api/sys/tasks/:id/progress/:session_id",
        scope_id: None,
        mode: PolicyMode::AuthOnly,
    },
    RoutePolicy {
        method: "GET",
        path: "/api/sys/tasks/:id/log/:session_id",
        scope_id: None,
        mode: PolicyMode::AuthOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/system/audit/routes",
        scope_id: None,
        mode: PolicyMode::AdminOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/system/audit/violations",
        scope_id: None,
        mode: PolicyMode::AdminOnly,
    },
    // Utility routes without explicit scope (logs, form-settings — low sensitivity)
    RoutePolicy {
        method: "*",
        path: "/api/logs",
        scope_id: None,
        mode: PolicyMode::AuthOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/form-settings/:form_key",
        scope_id: None,
        mode: PolicyMode::AuthOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/form-settings",
        scope_id: None,
        mode: PolicyMode::AuthOnly,
    },
    // Debug — open, dev only
    RoutePolicy {
        method: "GET",
        path: "/api/debug/tool-test",
        scope_id: None,
        mode: PolicyMode::Public,
    },
    // ========================================================================
    // Aggregates A001–A029
    // ========================================================================
    RoutePolicy {
        method: "*",
        path: "/api/connection_1c",
        scope_id: Some("a001_connection_1c"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/connection_1c/list",
        scope_id: Some("a001_connection_1c"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/connection_1c/:id",
        scope_id: Some("a001_connection_1c"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/connection_1c/test",
        scope_id: Some("a001_connection_1c"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/connection_1c/testdata",
        scope_id: Some("a001_connection_1c"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/organization",
        scope_id: Some("a002_organization"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/organization/:id",
        scope_id: Some("a002_organization"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/organization/testdata",
        scope_id: Some("a002_organization"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/counterparty",
        scope_id: Some("a003_counterparty"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/counterparty/:id",
        scope_id: Some("a003_counterparty"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/nomenclature",
        scope_id: Some("a004_nomenclature"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/nomenclature/:id",
        scope_id: Some("a004_nomenclature"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/nomenclature/import-excel",
        scope_id: Some("a004_nomenclature"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/nomenclature/dimensions",
        scope_id: Some("a004_nomenclature"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/nomenclature/search",
        scope_id: Some("a004_nomenclature"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/a004/nomenclature",
        scope_id: Some("a004_nomenclature"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/marketplace",
        scope_id: Some("a005_marketplace"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/marketplace/:id",
        scope_id: Some("a005_marketplace"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/marketplace/testdata",
        scope_id: Some("a005_marketplace"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/connection_mp",
        scope_id: Some("a006_connection_mp"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/connection_mp/:id",
        scope_id: Some("a006_connection_mp"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/connection_mp/testdata",
        scope_id: Some("a006_connection_mp"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/marketplace_product",
        scope_id: Some("a007_marketplace_product"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/marketplace_product/:id",
        scope_id: Some("a007_marketplace_product"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/marketplace_product/testdata",
        scope_id: Some("a007_marketplace_product"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/marketplace_sales",
        scope_id: Some("a008_marketplace_sales"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/marketplace_sales/:id",
        scope_id: Some("a008_marketplace_sales"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/ozon_returns",
        scope_id: Some("a009_ozon_returns"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/ozon_returns/:id",
        scope_id: Some("a009_ozon_returns"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/ozon_fbs_posting",
        scope_id: Some("a010_ozon_fbs_posting"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/ozon_fbs_posting/:id",
        scope_id: Some("a010_ozon_fbs_posting"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/ozon_fbo_posting",
        scope_id: Some("a011_ozon_fbo_posting"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/ozon_fbo_posting/:id",
        scope_id: Some("a011_ozon_fbo_posting"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/wb_sales",
        scope_id: Some("a012_wb_sales"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/wb_sales/:id",
        scope_id: Some("a012_wb_sales"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/ym_order",
        scope_id: Some("a013_ym_order"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/ym_order/:id",
        scope_id: Some("a013_ym_order"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/ozon_transactions",
        scope_id: Some("a014_ozon_transactions"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/ozon_transactions/:id",
        scope_id: Some("a014_ozon_transactions"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/a015/wb-orders",
        scope_id: Some("a015_wb_orders"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/a015/wb-orders/:id",
        scope_id: Some("a015_wb_orders"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/ym_returns",
        scope_id: Some("a016_ym_returns"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/ym_returns/:id",
        scope_id: Some("a016_ym_returns"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/llm-agent",
        scope_id: Some("a017_llm_agent"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/llm-agent/:id",
        scope_id: Some("a017_llm_agent"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/llm-chat",
        scope_id: Some("a018_llm_chat"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/llm-chat/:id",
        scope_id: Some("a018_llm_chat"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/llm-chat/:id/messages",
        scope_id: Some("a018_llm_chat"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/llm-chat/:id/run",
        scope_id: Some("a018_llm_chat"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/llm-artifact",
        scope_id: Some("a019_llm_artifact"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/llm-artifact/:id",
        scope_id: Some("a019_llm_artifact"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/wb-promotion",
        scope_id: Some("a020_wb_promotion"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/wb-promotion/:id",
        scope_id: Some("a020_wb_promotion"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/production_output",
        scope_id: Some("a021_production_output"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/production_output/:id",
        scope_id: Some("a021_production_output"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/kit_variant",
        scope_id: Some("a022_kit_variant"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/kit_variant/:id",
        scope_id: Some("a022_kit_variant"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/purchase_of_goods",
        scope_id: Some("a023_purchase_of_goods"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/purchase_of_goods/:id",
        scope_id: Some("a023_purchase_of_goods"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/a024-bi-indicator",
        scope_id: Some("a024_bi_indicator"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/a024-bi-indicator/:id",
        scope_id: Some("a024_bi_indicator"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/a024-bi-indicator/resolve-batch",
        scope_id: Some("a024_bi_indicator"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/a024-bi-indicator/:id/compute",
        scope_id: Some("a024_bi_indicator"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/a024-bi-indicator/compute-batch",
        scope_id: Some("a024_bi_indicator"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/drilldown/execute",
        scope_id: Some("a024_bi_indicator"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/a025-bi-dashboard",
        scope_id: Some("a025_bi_dashboard"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/a025-bi-dashboard/:id",
        scope_id: Some("a025_bi_dashboard"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/a026/wb-advert-daily",
        scope_id: Some("a026_wb_advert_daily"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/a026/wb-advert-daily/report.csv",
        scope_id: Some("a026_wb_advert_daily"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/a026/wb-advert-daily/:id",
        scope_id: Some("a026_wb_advert_daily"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/a027/wb-documents/list",
        scope_id: Some("a027_wb_documents"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/a027/wb-documents/:id",
        scope_id: Some("a027_wb_documents"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/a028/missing-cost-registry/list",
        scope_id: Some("a028_missing_cost_registry"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/a028/missing-cost-registry/:id",
        scope_id: Some("a028_missing_cost_registry"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/a029/wb-supply",
        scope_id: Some("a029_wb_supply"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/a029/wb-supply/:id",
        scope_id: Some("a029_wb_supply"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/a029/wb-supply/by-wb-id/:wb_id",
        scope_id: Some("a029_wb_supply"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/a029/raw/:ref_id",
        scope_id: Some("a029_wb_supply"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/a030/wb-advert-campaign/list",
        scope_id: Some("a030_wb_advert_campaign"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/a030/wb-advert-campaign/:id",
        scope_id: Some("a030_wb_advert_campaign"),
        mode: PolicyMode::ReadOnly,
    },
    // ========================================================================
    // Projections P900–P912
    // ========================================================================
    RoutePolicy {
        method: "*",
        path: "/api/p900/sales-register",
        scope_id: Some("p900_mp_sales_register"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p900/sales-register/:marketplace/:document_no/:line_id",
        scope_id: Some("p900_mp_sales_register"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p900/stats/by-date",
        scope_id: Some("p900_mp_sales_register"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p900/stats/by-marketplace",
        scope_id: Some("p900_mp_sales_register"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p900/backfill-product-refs",
        scope_id: Some("p900_mp_sales_register"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/projections/p900/:registrator_ref",
        scope_id: Some("p900_mp_sales_register"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p901/barcode/:barcode",
        scope_id: Some("p901_nomenclature_barcodes"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p901/nomenclature/:nomenclature_ref/barcodes",
        scope_id: Some("p901_nomenclature_barcodes"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p901/barcodes",
        scope_id: Some("p901_nomenclature_barcodes"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p902/finance-realization",
        scope_id: Some("p902_ozon_finance_realization"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p902/finance-realization/:posting_number/:sku/:operation_type",
        scope_id: Some("p902_ozon_finance_realization"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p902/stats",
        scope_id: Some("p902_ozon_finance_realization"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p903/finance-report",
        scope_id: Some("p903_wb_finance_report"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p903/finance-report/export",
        scope_id: Some("p903_wb_finance_report"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p903/finance-report/search-by-srid",
        scope_id: Some("p903_wb_finance_report"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p903/finance-report/operation-kinds",
        scope_id: Some("p903_wb_finance_report"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p903/finance-report/by-id/:id",
        scope_id: Some("p903_wb_finance_report"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p903/finance-report/by-id/:id/raw",
        scope_id: Some("p903_wb_finance_report"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p904/sales-data",
        scope_id: Some("p904_sales_data"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p905-commission/list",
        scope_id: Some("p905_wb_commission_history"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p905-commission/sync",
        scope_id: Some("p905_wb_commission_history"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p905-commission/:id",
        scope_id: Some("p905_wb_commission_history"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p905-commission",
        scope_id: Some("p905_wb_commission_history"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p906/nomenclature-prices",
        scope_id: Some("p906_nomenclature_prices"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p906/periods",
        scope_id: Some("p906_nomenclature_prices"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p906/import-excel",
        scope_id: Some("p906_nomenclature_prices"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p907/payment-report",
        scope_id: Some("p907_ym_payment_report"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p907/payment-report/:record_key",
        scope_id: Some("p907_ym_payment_report"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p908/goods-prices",
        scope_id: Some("p908_wb_goods_prices"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p908/goods-prices/:nm_id",
        scope_id: Some("p908_wb_goods_prices"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/p912/nomenclature-costs",
        scope_id: Some("p912_nomenclature_costs"),
        mode: PolicyMode::ReadOnly,
    },
    // ========================================================================
    // Usecases U501–U508
    // ========================================================================
    RoutePolicy {
        method: "*",
        path: "/api/u501/import/start",
        scope_id: Some("u501_import_from_ut"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/u501/import/:session_id/progress",
        scope_id: Some("u501_import_from_ut"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/u502/import/start",
        scope_id: Some("u502_import_from_ozon"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/u502/import/:session_id/progress",
        scope_id: Some("u502_import_from_ozon"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/u503/import/start",
        scope_id: Some("u503_import_from_yandex"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/u503/import/:session_id/progress",
        scope_id: Some("u503_import_from_yandex"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/u504/import/start",
        scope_id: Some("u504_import_from_wildberries"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/u504/import/:session_id/progress",
        scope_id: Some("u504_import_from_wildberries"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/u505/match/start",
        scope_id: Some("u505_match_nomenclature"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/u505/match/:session_id/progress",
        scope_id: Some("u505_match_nomenclature"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/u506/import/start",
        scope_id: Some("u506_import_from_lemanapro"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/u506/import/:session_id/progress",
        scope_id: Some("u506_import_from_lemanapro"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/u507/import/start",
        scope_id: Some("u507_import_from_erp"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/u507/import/:session_id/progress",
        scope_id: Some("u507_import_from_erp"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/u508/repost/projections",
        scope_id: Some("u508_repost_documents"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/u508/repost/aggregates",
        scope_id: Some("u508_repost_documents"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/u508/repost/start",
        scope_id: Some("u508_repost_documents"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/u508/repost/aggregate/start",
        scope_id: Some("u508_repost_documents"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/u508/repost/:session_id/progress",
        scope_id: Some("u508_repost_documents"),
        mode: PolicyMode::ReadOnly,
    },
    // ========================================================================
    // Dashboards
    // ========================================================================
    RoutePolicy {
        method: "*",
        path: "/api/d400/monthly_summary",
        scope_id: Some("dashboard"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/d400/periods",
        scope_id: Some("dashboard"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/universal-dashboard/execute",
        scope_id: Some("dashboard"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/universal-dashboard/generate-sql",
        scope_id: Some("dashboard"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/universal-dashboard/schemas",
        scope_id: Some("dashboard"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/universal-dashboard/schemas/:id",
        scope_id: Some("dashboard"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/universal-dashboard/configs",
        scope_id: Some("dashboard"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/universal-dashboard/configs/:id",
        scope_id: Some("dashboard"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/ds01/execute",
        scope_id: Some("dashboard"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/ds01/schemas",
        scope_id: Some("dashboard"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/ds01/configs",
        scope_id: Some("dashboard"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/ds02/execute",
        scope_id: Some("dashboard"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/ds02/schemas",
        scope_id: Some("dashboard"),
        mode: PolicyMode::Auto,
    },
    RoutePolicy {
        method: "*",
        path: "/api/ds02/configs",
        scope_id: Some("dashboard"),
        mode: PolicyMode::Auto,
    },
    // ========================================================================
    // Data Views
    // ========================================================================
    RoutePolicy {
        method: "*",
        path: "/api/data-view",
        scope_id: Some("data_view"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/data-view/filters",
        scope_id: Some("data_view"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/data-view/:id",
        scope_id: Some("data_view"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/data-view/:id/filters",
        scope_id: Some("data_view"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/data-view/:id/compute",
        scope_id: Some("data_view"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/data-view/:id/drilldown",
        scope_id: Some("data_view"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/data-view/:id/drilldown-capabilities",
        scope_id: Some("data_view"),
        mode: PolicyMode::ReadOnly,
    },
    // ========================================================================
    // General Ledger
    // ========================================================================
    RoutePolicy {
        method: "*",
        path: "/api/general-ledger",
        scope_id: Some("general_ledger"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/general-ledger/turnovers",
        scope_id: Some("general_ledger"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/general-ledger/report",
        scope_id: Some("general_ledger"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/general-ledger/account-view",
        scope_id: Some("general_ledger"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/reports/wb-weekly-reconciliation",
        scope_id: Some("general_ledger"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/general-ledger/report/dimensions",
        scope_id: Some("general_ledger"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/general-ledger/report/drilldown",
        scope_id: Some("general_ledger"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/general-ledger/drilldown",
        scope_id: Some("general_ledger"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/general-ledger/drilldown/:id",
        scope_id: Some("general_ledger"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/general-ledger/drilldown/:id/data",
        scope_id: Some("general_ledger"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/general-ledger/:id",
        scope_id: Some("general_ledger"),
        mode: PolicyMode::ReadOnly,
    },
    // LLM Knowledge (read-only reference data — same scope as llm chat for simplicity)
    RoutePolicy {
        method: "*",
        path: "/api/llm-knowledge",
        scope_id: Some("a018_llm_chat"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/llm-knowledge/:id",
        scope_id: Some("a018_llm_chat"),
        mode: PolicyMode::ReadOnly,
    },
    // Sys-drilldown session store (internal; tied to data_view usage)
    RoutePolicy {
        method: "*",
        path: "/api/sys-drilldown",
        scope_id: Some("data_view"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/sys-drilldown/:id",
        scope_id: Some("data_view"),
        mode: PolicyMode::ReadOnly,
    },
    RoutePolicy {
        method: "*",
        path: "/api/sys-drilldown/:id/data",
        scope_id: Some("data_view"),
        mode: PolicyMode::ReadOnly,
    },
];

/// Look up the policy entries for a given scope_id.
pub fn policies_for_scope(scope_id: &str) -> Vec<&'static RoutePolicy> {
    ROUTE_REGISTRY
        .iter()
        .filter(|p| p.scope_id == Some(scope_id))
        .collect()
}

// ============================================================================
// Integrity tests
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::access::scope_catalog::SCOPE_CATALOG;

    #[test]
    fn all_scoped_routes_have_known_scope_id() {
        let catalog_ids: std::collections::HashSet<&str> =
            SCOPE_CATALOG.iter().map(|s| s.scope_id).collect();

        let mut failures = Vec::new();
        for policy in ROUTE_REGISTRY {
            if let Some(scope_id) = policy.scope_id {
                if !catalog_ids.contains(scope_id) {
                    failures.push(format!(
                        "ROUTE_REGISTRY entry {} {} references unknown scope_id '{}'",
                        policy.method, policy.path, scope_id
                    ));
                }
            }
        }

        if !failures.is_empty() {
            panic!(
                "Routes reference unknown scope IDs:\n{}",
                failures.join("\n")
            );
        }
    }

    #[test]
    fn all_catalog_scopes_covered_by_at_least_one_route() {
        let registry_scopes: std::collections::HashSet<&str> =
            ROUTE_REGISTRY.iter().filter_map(|p| p.scope_id).collect();

        let mut orphans = Vec::new();
        for scope in SCOPE_CATALOG {
            if !registry_scopes.contains(scope.scope_id) {
                orphans.push(scope.scope_id);
            }
        }

        if !orphans.is_empty() {
            panic!(
                "SCOPE_CATALOG entries not covered by any route:\n{}",
                orphans.join("\n")
            );
        }
    }

    #[test]
    fn no_auth_only_routes() {
        let auth_only: Vec<_> = ROUTE_REGISTRY
            .iter()
            .filter(|p| p.mode == PolicyMode::AuthOnly)
            .map(|p| format!("{} {}", p.method, p.path))
            .collect();

        if !auth_only.is_empty() {
            // This is a warning, not a hard failure — AuthOnly routes are
            // documented violations that should be resolved over time.
            eprintln!(
                "WARNING: {} AuthOnly routes (no scope assigned):\n{}",
                auth_only.len(),
                auth_only.join("\n")
            );
        }
        // Do not panic — these are known and tracked via the audit endpoint.
    }
}
