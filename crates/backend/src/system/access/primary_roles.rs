//! Built-in primary role catalog with default scope grants.
//!
//! Primary roles are defined in code, not in the DB (they are seeded with is_system=1
//! but their grants are the authoritative source here, not sys_role_scope_access).

/// Default scope grants for the `manager` role.
/// Managers get full access to all aggregates.
pub const MANAGER_GRANTS: &[(&str, &str)] = &[
    ("a001_connection_1c", "all"),
    ("a002_organization", "all"),
    ("a003_counterparty", "all"),
    ("a004_nomenclature", "all"),
    ("a005_marketplace", "all"),
    ("a006_connection_mp", "all"),
    ("a007_marketplace_product", "all"),
    ("a008_marketplace_sales", "all"),
    ("a009_ozon_returns", "all"),
    ("a010_ozon_fbs_posting", "all"),
    ("a011_ozon_fbo_posting", "all"),
    ("a012_wb_sales", "all"),
    ("a013_ym_order", "all"),
    ("a014_ozon_transactions", "all"),
    ("a015_wb_orders", "all"),
    ("a016_ym_returns", "all"),
    ("a017_llm_agent", "all"),
    ("a018_llm_chat", "all"),
    ("a019_llm_artifact", "all"),
    ("a020_wb_promotion", "all"),
    ("a021_production_output", "all"),
    ("a022_kit_variant", "all"),
    ("a023_purchase_of_goods", "all"),
    ("a024_bi_indicator", "all"),
    ("a025_bi_dashboard", "all"),
    ("a026_wb_advert_daily", "all"),
    ("a027_wb_documents", "all"),
    ("a028_missing_cost_registry", "all"),
];

/// Default scope grants for the `operator` role.
/// Operators get read access to references, full access to operational data.
pub const OPERATOR_GRANTS: &[(&str, &str)] = &[
    ("a001_connection_1c", "read"),
    ("a002_organization", "read"),
    ("a003_counterparty", "read"),
    ("a004_nomenclature", "read"),
    ("a005_marketplace", "read"),
    ("a006_connection_mp", "read"),
    ("a007_marketplace_product", "read"),
    ("a008_marketplace_sales", "all"),
    ("a009_ozon_returns", "all"),
    ("a010_ozon_fbs_posting", "all"),
    ("a011_ozon_fbo_posting", "all"),
    ("a012_wb_sales", "all"),
    ("a013_ym_order", "all"),
    ("a014_ozon_transactions", "all"),
    ("a015_wb_orders", "all"),
    ("a016_ym_returns", "all"),
    ("a018_llm_chat", "read"),
    ("a020_wb_promotion", "all"),
    ("a021_production_output", "all"),
    ("a022_kit_variant", "all"),
    ("a023_purchase_of_goods", "all"),
    ("a024_bi_indicator", "read"),
    ("a025_bi_dashboard", "all"),
    ("a026_wb_advert_daily", "all"),
    ("a027_wb_documents", "all"),
    ("a028_missing_cost_registry", "all"),
];

/// Default scope grants for the `viewer` role.
/// Viewers get read-only access to analytics and core references.
pub const VIEWER_GRANTS: &[(&str, &str)] = &[
    ("a002_organization", "read"),
    ("a004_nomenclature", "read"),
    ("a005_marketplace", "read"),
    ("a007_marketplace_product", "read"),
    ("a008_marketplace_sales", "read"),
    ("a012_wb_sales", "read"),
    ("a013_ym_order", "read"),
    ("a024_bi_indicator", "read"),
    ("a025_bi_dashboard", "read"),
    ("a026_wb_advert_daily", "read"),
    ("a027_wb_documents", "read"),
    ("a028_missing_cost_registry", "read"),
];

/// `admin` primary role: all access is granted via `is_admin=true` bypass.
/// No explicit grants needed.
pub const ADMIN_GRANTS: &[(&str, &str)] = &[];

/// Get built-in grants for a primary role code.
/// Returns empty slice for unknown roles (default deny).
pub fn grants_for_role(role_code: &str) -> &'static [(&'static str, &'static str)] {
    match role_code {
        "admin" => ADMIN_GRANTS,
        "manager" => MANAGER_GRANTS,
        "operator" => OPERATOR_GRANTS,
        "viewer" => VIEWER_GRANTS,
        _ => &[],
    }
}
