INSERT OR IGNORE INTO sys_role_scope_access (role_id, access_scope_id, access_mode)
SELECT id, 'a027_wb_documents', 'all'
FROM sys_roles
WHERE code = 'manager';

INSERT OR IGNORE INTO sys_role_scope_access (role_id, access_scope_id, access_mode)
SELECT id, 'a027_wb_documents', 'all'
FROM sys_roles
WHERE code = 'operator';

INSERT OR IGNORE INTO sys_role_scope_access (role_id, access_scope_id, access_mode)
SELECT id, 'a027_wb_documents', 'read'
FROM sys_roles
WHERE code = 'viewer';
