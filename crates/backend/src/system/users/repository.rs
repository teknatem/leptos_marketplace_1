use anyhow::{Context, Result};
use contracts::system::users::User;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

/// Create user with password hash
pub async fn create_with_password(user: &User, password_hash: &str) -> Result<()> {
    use crate::shared::data::db::get_connection;

    let conn = get_connection();

    conn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "INSERT INTO sys_users (id, username, email, password_hash, full_name, is_active, is_admin, created_at, updated_at, last_login_at, created_by) 
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        [
            user.id.clone().into(),
            user.username.clone().into(),
            user.email.clone().into(),
            password_hash.to_string().into(),
            user.full_name.clone().into(),
            (if user.is_active { 1 } else { 0 }).into(),
            (if user.is_admin { 1 } else { 0 }).into(),
            user.created_at.clone().into(),
            user.updated_at.clone().into(),
            user.last_login_at.clone().into(),
            user.created_by.clone().into(),
        ],
    ))
    .await
    .context("Failed to insert user")?;

    Ok(())
}

/// Get user by ID
pub async fn get_by_id(id: &str) -> Result<Option<User>> {
    use crate::shared::data::db::get_connection;

    let conn = get_connection();

    let result = conn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT id, username, email, full_name, is_active, is_admin, created_at, updated_at, last_login_at, created_by 
             FROM sys_users WHERE id = ?",
            [id.into()],
        ))
        .await?;

    match result {
        Some(row) => {
            let user = User {
                id: row.try_get("", "id")?,
                username: row.try_get("", "username")?,
                email: row.try_get("", "email")?,
                full_name: row.try_get("", "full_name")?,
                is_active: row.try_get::<i32>("", "is_active")? != 0,
                is_admin: row.try_get::<i32>("", "is_admin")? != 0,
                created_at: row.try_get("", "created_at")?,
                updated_at: row.try_get("", "updated_at")?,
                last_login_at: row.try_get("", "last_login_at")?,
                created_by: row.try_get("", "created_by")?,
            };
            Ok(Some(user))
        }
        None => Ok(None),
    }
}

/// Get user by username
pub async fn get_by_username(username: &str) -> Result<Option<User>> {
    use crate::shared::data::db::get_connection;

    let conn = get_connection();

    let result = conn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT id, username, email, full_name, is_active, is_admin, created_at, updated_at, last_login_at, created_by 
             FROM sys_users WHERE username = ?",
            [username.into()],
        ))
        .await?;

    match result {
        Some(row) => {
            let user = User {
                id: row.try_get("", "id")?,
                username: row.try_get("", "username")?,
                email: row.try_get("", "email")?,
                full_name: row.try_get("", "full_name")?,
                is_active: row.try_get::<i32>("", "is_active")? != 0,
                is_admin: row.try_get::<i32>("", "is_admin")? != 0,
                created_at: row.try_get("", "created_at")?,
                updated_at: row.try_get("", "updated_at")?,
                last_login_at: row.try_get("", "last_login_at")?,
                created_by: row.try_get("", "created_by")?,
            };
            Ok(Some(user))
        }
        None => Ok(None),
    }
}

/// Get password hash for user
pub async fn get_password_hash(user_id: &str) -> Result<Option<String>> {
    use crate::shared::data::db::get_connection;

    let conn = get_connection();

    let result = conn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT password_hash FROM sys_users WHERE id = ?",
            [user_id.into()],
        ))
        .await?;

    match result {
        Some(row) => {
            let hash: String = row.try_get("", "password_hash")?;
            Ok(Some(hash))
        }
        None => Ok(None),
    }
}

/// List all users
pub async fn list_all() -> Result<Vec<User>> {
    use crate::shared::data::db::get_connection;

    let conn = get_connection();

    let rows = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT id, username, email, full_name, is_active, is_admin, created_at, updated_at, last_login_at, created_by 
             FROM sys_users ORDER BY created_at DESC".to_string(),
        ))
        .await?;

    let mut users = Vec::new();
    for row in rows {
        let user = User {
            id: row.try_get("", "id")?,
            username: row.try_get("", "username")?,
            email: row.try_get("", "email")?,
            full_name: row.try_get("", "full_name")?,
            is_active: row.try_get::<i32>("", "is_active")? != 0,
            is_admin: row.try_get::<i32>("", "is_admin")? != 0,
            created_at: row.try_get("", "created_at")?,
            updated_at: row.try_get("", "updated_at")?,
            last_login_at: row.try_get("", "last_login_at")?,
            created_by: row.try_get("", "created_by")?,
        };
        users.push(user);
    }

    Ok(users)
}

/// Update user
pub async fn update(user: &User) -> Result<()> {
    use crate::shared::data::db::get_connection;

    let conn = get_connection();

    conn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "UPDATE sys_users 
         SET email = ?, full_name = ?, is_active = ?, is_admin = ?, updated_at = ? 
         WHERE id = ?",
        [
            user.email.clone().into(),
            user.full_name.clone().into(),
            (if user.is_active { 1 } else { 0 }).into(),
            (if user.is_admin { 1 } else { 0 }).into(),
            user.updated_at.clone().into(),
            user.id.clone().into(),
        ],
    ))
    .await
    .context("Failed to update user")?;

    Ok(())
}

/// Delete user (hard delete)
pub async fn delete(id: &str) -> Result<bool> {
    use crate::shared::data::db::get_connection;

    let conn = get_connection();

    let result = conn
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "DELETE FROM sys_users WHERE id = ?",
            [id.into()],
        ))
        .await
        .context("Failed to delete user")?;

    Ok(result.rows_affected() > 0)
}

/// Update last login timestamp
pub async fn update_last_login(id: &str) -> Result<()> {
    use crate::shared::data::db::get_connection;

    let now = chrono::Utc::now().to_rfc3339();
    let conn = get_connection();

    conn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "UPDATE sys_users SET last_login_at = ? WHERE id = ?",
        [now.into(), id.to_string().into()],
    ))
    .await
    .context("Failed to update last login")?;

    Ok(())
}

/// Count total users
pub async fn count_users() -> Result<usize> {
    use crate::shared::data::db::get_connection;

    let conn = get_connection();

    let result = conn
        .query_one(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT COUNT(*) as count FROM sys_users".to_string(),
        ))
        .await?;

    match result {
        Some(row) => {
            let count: i64 = row.try_get("", "count")?;
            Ok(count as usize)
        }
        None => Ok(0),
    }
}

/// Update user password
pub async fn update_password(id: &str, password_hash: &str) -> Result<()> {
    use crate::shared::data::db::get_connection;

    let conn = get_connection();

    conn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "UPDATE sys_users SET password_hash = ?, updated_at = ? WHERE id = ?",
        [
            password_hash.to_string().into(),
            chrono::Utc::now().to_rfc3339().into(),
            id.to_string().into(),
        ],
    ))
    .await
    .context("Failed to update password")?;

    Ok(())
}
