-- System settings table (for JWT_SECRET and other system-wide configs)
CREATE TABLE IF NOT EXISTS sys_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Users table
CREATE TABLE IF NOT EXISTS sys_users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    email TEXT UNIQUE,
    password_hash TEXT NOT NULL,
    full_name TEXT,
    is_active INTEGER DEFAULT 1 NOT NULL,
    is_admin INTEGER DEFAULT 0 NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_login_at TEXT,
    created_by TEXT,
    FOREIGN KEY (created_by) REFERENCES sys_users(id)
);

CREATE INDEX IF NOT EXISTS idx_sys_users_username ON sys_users(username);
CREATE INDEX IF NOT EXISTS idx_sys_users_email ON sys_users(email);
CREATE INDEX IF NOT EXISTS idx_sys_users_active ON sys_users(is_active);

-- Refresh tokens table
CREATE TABLE IF NOT EXISTS sys_refresh_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    token_hash TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    revoked_at TEXT,
    ip_address TEXT,
    user_agent TEXT,
    FOREIGN KEY (user_id) REFERENCES sys_users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user ON sys_refresh_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_expires ON sys_refresh_tokens(expires_at);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_hash ON sys_refresh_tokens(token_hash);

-- Audit log table
CREATE TABLE IF NOT EXISTS sys_audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT,
    action TEXT NOT NULL,
    entity_type TEXT,
    entity_id TEXT,
    details TEXT,
    ip_address TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES sys_users(id)
);

CREATE INDEX IF NOT EXISTS idx_audit_log_user ON sys_audit_log(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_created ON sys_audit_log(created_at);
CREATE INDEX IF NOT EXISTS idx_audit_log_action ON sys_audit_log(action);

