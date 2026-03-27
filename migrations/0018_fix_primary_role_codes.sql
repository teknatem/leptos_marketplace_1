-- =============================================================================
-- Migration 0018: Fix primary_role_code for existing users
-- =============================================================================
-- Migration 0017 added the column with DEFAULT 'viewer' and ran UPDATE,
-- but if users were created after that migration (e.g. via ensure_admin_user_exists),
-- they would still have 'viewer'. This migration corrects that.
-- =============================================================================

-- Set primary_role_code = 'admin' for users with is_admin = 1
-- Using COALESCE-safe update (no-op if already correct)
UPDATE sys_users
SET primary_role_code = 'admin'
WHERE is_admin = 1
  AND primary_role_code != 'admin';
