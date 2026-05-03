use anyhow::{Context, Result};
use contracts::system::roles::{CreateRoleDto, Role, RoleScopeAccess, UpdateRoleDto};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait};

fn row_to_role(row: &sea_orm::QueryResult) -> Result<Role> {
    // sys_roles uses INTEGER PRIMARY KEY — cast to TEXT in SQL for uniform string API
    Ok(Role {
        id: row.try_get::<i64>("", "id")?.to_string(),
        code: row.try_get("", "code")?,
        name: row.try_get("", "name")?,
        is_system: row.try_get::<i32>("", "is_system")? != 0,
    })
}

pub async fn list_all() -> Result<Vec<Role>> {
    use crate::shared::data::db::get_connection;

    let conn = get_connection();

    let rows = conn
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT id, code, name, is_system FROM sys_roles ORDER BY is_system DESC, code ASC"
                .to_string(),
        ))
        .await?;

    let mut roles = Vec::new();
    for row in rows {
        roles.push(row_to_role(&row)?);
    }

    Ok(roles)
}

pub async fn get_by_id(id: &str) -> Result<Option<Role>> {
    use crate::shared::data::db::get_connection;

    let id_i64: i64 = id.parse().unwrap_or(0);
    let conn = get_connection();

    let result = conn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT id, code, name, is_system FROM sys_roles WHERE id = ?",
            [id_i64.into()],
        ))
        .await?;

    match result {
        Some(row) => Ok(Some(row_to_role(&row)?)),
        None => Ok(None),
    }
}

pub async fn get_by_code(code: &str) -> Result<Option<Role>> {
    use crate::shared::data::db::get_connection;

    let conn = get_connection();

    let result = conn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT id, code, name, is_system FROM sys_roles WHERE code = ?",
            [code.into()],
        ))
        .await?;

    match result {
        Some(row) => Ok(Some(row_to_role(&row)?)),
        None => Ok(None),
    }
}

pub async fn create(dto: &CreateRoleDto) -> Result<String> {
    use crate::shared::data::db::get_connection;

    let conn = get_connection();

    // Use AUTOINCREMENT — do not pass id
    conn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "INSERT INTO sys_roles (code, name, is_system) VALUES (?, ?, 0)",
        [dto.code.clone().into(), dto.name.clone().into()],
    ))
    .await
    .context("Failed to create role")?;

    // Get the auto-generated id
    let result = conn
        .query_one(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT last_insert_rowid() AS id".to_string(),
        ))
        .await?;

    let id: i64 = result
        .ok_or_else(|| anyhow::anyhow!("No last insert rowid"))?
        .try_get("", "id")?;

    Ok(id.to_string())
}

pub async fn update(dto: &UpdateRoleDto) -> Result<()> {
    use crate::shared::data::db::get_connection;

    let id_i64: i64 = dto.id.parse().unwrap_or(0);
    let conn = get_connection();

    conn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "UPDATE sys_roles SET name = ? WHERE id = ?",
        [dto.name.clone().into(), id_i64.into()],
    ))
    .await
    .context("Failed to update role")?;

    Ok(())
}

pub async fn delete(id: &str) -> Result<bool> {
    use crate::shared::data::db::get_connection;

    let id_i64: i64 = id.parse().unwrap_or(0);
    let conn = get_connection();

    let result = conn
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "DELETE FROM sys_roles WHERE id = ?",
            [id_i64.into()],
        ))
        .await
        .context("Failed to delete role")?;

    Ok(result.rows_affected() > 0)
}

pub async fn get_scope_access(role_id: &str) -> Result<Vec<RoleScopeAccess>> {
    use crate::shared::data::db::get_connection;

    let id_i64: i64 = role_id.parse().unwrap_or(0);
    let conn = get_connection();

    let rows = conn
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT access_scope_id, access_mode FROM sys_role_scope_access WHERE role_id = ? ORDER BY access_scope_id ASC",
            [id_i64.into()],
        ))
        .await?;

    let mut result = Vec::new();
    for row in rows {
        result.push(RoleScopeAccess {
            scope_id: row.try_get("", "access_scope_id")?,
            access_mode: row.try_get("", "access_mode")?,
        });
    }

    Ok(result)
}

/// Replace all scope access grants for a role atomically (DELETE + INSERT in a transaction).
pub async fn replace_all_scope_access(role_id: &str, grants: &[RoleScopeAccess]) -> Result<()> {
    use crate::shared::data::db::get_connection;

    let id_i64: i64 = role_id.parse().unwrap_or(0);
    let conn = get_connection();
    let txn = conn.begin().await?;

    txn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "DELETE FROM sys_role_scope_access WHERE role_id = ?",
        [id_i64.into()],
    ))
    .await
    .context("Failed to clear scope access")?;

    for grant in grants {
        txn.execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "INSERT INTO sys_role_scope_access (role_id, access_scope_id, access_mode) VALUES (?, ?, ?)",
            [id_i64.into(), grant.scope_id.clone().into(), grant.access_mode.clone().into()],
        ))
        .await
        .context("Failed to insert scope access")?;
    }

    txn.commit()
        .await
        .context("Failed to commit scope access transaction")?;
    Ok(())
}
