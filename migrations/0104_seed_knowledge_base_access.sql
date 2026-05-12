-- Grant manager role access to the read-only Knowledge Base workspace.

INSERT OR IGNORE INTO sys_role_scope_access (role_id, access_scope_id, access_mode)
SELECT id, 'knowledge_base', 'all'
FROM sys_roles
WHERE code = 'manager';
