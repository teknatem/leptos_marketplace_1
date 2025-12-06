use anyhow::{Context, Result};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

/// Apply authentication system migration
pub async fn apply_auth_migration() -> Result<()> {
    use crate::shared::data::db::get_connection;

    // Read migration file (check both current dir and parent dir for workspace root)
    let migration_sql = std::fs::read_to_string("migrate_auth_system.sql")
        .or_else(|_| std::fs::read_to_string("../../migrate_auth_system.sql"))
        .context("Failed to read migrate_auth_system.sql. Make sure it's in the project root.")?;

    let conn = get_connection();

    // Execute each statement separately (SQLite doesn't support execute_batch in sea-orm)
    for (idx, statement) in migration_sql.split(';').enumerate() {
        // Remove comment lines and trim
        let cleaned: String = statement
            .lines()
            .filter(|line| {
                let trimmed_line = line.trim();
                !trimmed_line.is_empty() && !trimmed_line.starts_with("--")
            })
            .collect::<Vec<_>>()
            .join("\n");
        
        let trimmed = cleaned.trim();
        if !trimmed.is_empty() {
            // Log the statement for debugging (first 100 chars)
            let preview = trimmed.chars().take(100).collect::<String>().replace('\n', " ");
            tracing::info!("Executing migration statement #{}: {}...", idx, preview);
            
            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                format!("{};", trimmed),
            ))
            .await
            .with_context(|| format!("Failed to execute statement #{}: {}", idx, trimmed.lines().take(3).collect::<Vec<_>>().join(" ")))?;
        }
    }

    tracing::info!("Auth system migration applied successfully");

    Ok(())
}

/// Ensure admin user exists (create if table is empty)
pub async fn ensure_admin_user_exists() -> Result<()> {
    use crate::system::users::{repository, service};
    use contracts::system::users::CreateUserDto;

    // Check if any users exist
    let count = repository::count_users().await?;

    if count == 0 {
        tracing::info!("No users found. Creating default admin user...");

        let admin_dto = CreateUserDto {
            username: "admin".to_string(),
            password: "admin".to_string(),
            email: None,
            full_name: Some("Administrator".to_string()),
            is_admin: true,
        };

        let admin_id = service::create(admin_dto, None).await?;

        tracing::warn!("═══════════════════════════════════════════════");
        tracing::warn!("  Default admin user created!");
        tracing::warn!("  Username: admin");
        tracing::warn!("  Password: admin");
        tracing::warn!("  User ID: {}", admin_id);
        tracing::warn!("  ⚠️  PLEASE CHANGE THE PASSWORD IMMEDIATELY!");
        tracing::warn!("═══════════════════════════════════════════════");
    }

    Ok(())
}
