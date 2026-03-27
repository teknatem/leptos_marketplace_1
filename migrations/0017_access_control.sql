-- =============================================================================
-- Migration 0017: Access Control System
-- =============================================================================
-- Adds: sys_roles, sys_user_roles, sys_role_scope_access
--       primary_role_code column in sys_users
-- =============================================================================

-- Add primary_role_code to sys_users (default 'viewer' for existing users)
ALTER TABLE sys_users ADD COLUMN primary_role_code TEXT NOT NULL DEFAULT 'viewer';

-- Set primary_role_code = 'admin' for users with is_admin = 1
UPDATE sys_users SET primary_role_code = 'admin' WHERE is_admin = 1;

-- Roles table (includes built-in is_system=1 roles)
CREATE TABLE sys_roles (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    code      TEXT    NOT NULL UNIQUE,
    name      TEXT    NOT NULL,
    is_system INTEGER NOT NULL DEFAULT 0
);

-- Seed built-in primary roles
INSERT INTO sys_roles (code, name, is_system) VALUES
    ('admin',    'Администратор', 1),
    ('manager',  'Менеджер',      1),
    ('operator', 'Оператор',      1),
    ('viewer',   'Наблюдатель',   1);

-- Additional roles assigned to users (from DB, beyond primary_role)
CREATE TABLE sys_user_roles (
    user_id INTEGER NOT NULL REFERENCES sys_users(id) ON DELETE CASCADE,
    role_id INTEGER NOT NULL REFERENCES sys_roles(id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, role_id)
);

-- Access matrix: role x scope -> mode
CREATE TABLE sys_role_scope_access (
    role_id         INTEGER NOT NULL REFERENCES sys_roles(id) ON DELETE CASCADE,
    access_scope_id TEXT    NOT NULL,
    access_mode     TEXT    NOT NULL CHECK(access_mode IN ('read', 'all')),
    PRIMARY KEY (role_id, access_scope_id)
);

-- Default rights for built-in roles:
-- admin gets all via is_admin bypass, no rows needed
-- manager: full access to most aggregates
INSERT INTO sys_role_scope_access (role_id, access_scope_id, access_mode)
SELECT id, scope, 'all' FROM sys_roles, (
    SELECT 'a001_connection_1c'       AS scope UNION ALL
    SELECT 'a002_organization'        UNION ALL
    SELECT 'a003_counterparty'        UNION ALL
    SELECT 'a004_nomenclature'        UNION ALL
    SELECT 'a005_marketplace'         UNION ALL
    SELECT 'a006_connection_mp'       UNION ALL
    SELECT 'a007_marketplace_product' UNION ALL
    SELECT 'a008_marketplace_sales'   UNION ALL
    SELECT 'a009_ozon_returns'        UNION ALL
    SELECT 'a010_ozon_fbs_posting'    UNION ALL
    SELECT 'a011_ozon_fbo_posting'    UNION ALL
    SELECT 'a012_wb_sales'            UNION ALL
    SELECT 'a013_ym_order'            UNION ALL
    SELECT 'a014_ozon_transactions'   UNION ALL
    SELECT 'a015_wb_orders'           UNION ALL
    SELECT 'a016_ym_returns'          UNION ALL
    SELECT 'a017_llm_agent'           UNION ALL
    SELECT 'a018_llm_chat'            UNION ALL
    SELECT 'a019_llm_artifact'        UNION ALL
    SELECT 'a020_wb_promotion'        UNION ALL
    SELECT 'a021_production_output'   UNION ALL
    SELECT 'a022_kit_variant'         UNION ALL
    SELECT 'a023_purchase_of_goods'   UNION ALL
    SELECT 'a024_bi_indicator'        UNION ALL
    SELECT 'a025_bi_dashboard'
) AS scopes
WHERE sys_roles.code = 'manager';

-- operator: read-only access to most + full access to documents/imports
INSERT INTO sys_role_scope_access (role_id, access_scope_id, access_mode)
SELECT id, scope, mode FROM sys_roles, (
    SELECT 'a001_connection_1c'       AS scope, 'read' AS mode UNION ALL
    SELECT 'a002_organization',       'read'  UNION ALL
    SELECT 'a003_counterparty',       'read'  UNION ALL
    SELECT 'a004_nomenclature',       'read'  UNION ALL
    SELECT 'a005_marketplace',        'read'  UNION ALL
    SELECT 'a006_connection_mp',      'read'  UNION ALL
    SELECT 'a007_marketplace_product','read'  UNION ALL
    SELECT 'a008_marketplace_sales',  'all'   UNION ALL
    SELECT 'a009_ozon_returns',       'all'   UNION ALL
    SELECT 'a010_ozon_fbs_posting',   'all'   UNION ALL
    SELECT 'a011_ozon_fbo_posting',   'all'   UNION ALL
    SELECT 'a012_wb_sales',           'all'   UNION ALL
    SELECT 'a013_ym_order',           'all'   UNION ALL
    SELECT 'a014_ozon_transactions',  'all'   UNION ALL
    SELECT 'a015_wb_orders',          'all'   UNION ALL
    SELECT 'a016_ym_returns',         'all'   UNION ALL
    SELECT 'a018_llm_chat',           'read'  UNION ALL
    SELECT 'a020_wb_promotion',       'all'   UNION ALL
    SELECT 'a021_production_output',  'all'   UNION ALL
    SELECT 'a022_kit_variant',        'all'   UNION ALL
    SELECT 'a023_purchase_of_goods',  'all'   UNION ALL
    SELECT 'a024_bi_indicator',       'read'  UNION ALL
    SELECT 'a025_bi_dashboard',       'read'
) AS scopes
WHERE sys_roles.code = 'operator';

-- viewer: read-only access to analytics/reports
INSERT INTO sys_role_scope_access (role_id, access_scope_id, access_mode)
SELECT id, scope, 'read' FROM sys_roles, (
    SELECT 'a002_organization'        AS scope UNION ALL
    SELECT 'a004_nomenclature'        UNION ALL
    SELECT 'a005_marketplace'         UNION ALL
    SELECT 'a007_marketplace_product' UNION ALL
    SELECT 'a008_marketplace_sales'   UNION ALL
    SELECT 'a012_wb_sales'            UNION ALL
    SELECT 'a013_ym_order'            UNION ALL
    SELECT 'a024_bi_indicator'        UNION ALL
    SELECT 'a025_bi_dashboard'
) AS scopes
WHERE sys_roles.code = 'viewer';
