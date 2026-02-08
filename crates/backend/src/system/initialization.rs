use anyhow::{Context, Result};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

/// Apply authentication system migration
pub async fn apply_auth_migration() -> Result<()> {
    use crate::shared::data::db::get_connection;

    // Try to find migration file in multiple locations
    let mut migration_paths = Vec::new();

    // 1. Next to executable (production)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            migration_paths.push(exe_dir.join("migrate_auth_system.sql"));
        }
    }

    // 2. Current directory (development)
    migration_paths.push(std::path::PathBuf::from("migrate_auth_system.sql"));

    // 3. Project root (when running from target/)
    migration_paths.push(std::path::PathBuf::from("../../migrate_auth_system.sql"));

    // Try to read migration file from any of these locations
    let migration_sql = {
        let mut found = None;
        for path in &migration_paths {
            if let Ok(content) = std::fs::read_to_string(path) {
                tracing::info!("Found migration file: {}", path.display());
                found = Some(content);
                break;
            }
        }

        match found {
            Some(content) => content,
            None => {
                // Migration file not found - just warn and continue
                println!("\n⚠  WARNING: Migration file not found!");
                println!("   Searched in:");
                for path in &migration_paths {
                    println!("   - {}", path.display());
                }
                println!("\n   This is OK if database is already migrated.");
                println!("   If you need to run migrations, place 'migrate_auth_system.sql'");
                println!("   next to the executable.\n");

                tracing::warn!(
                    "Migration file 'migrate_auth_system.sql' not found, skipping migration"
                );
                tracing::warn!("This is normal if the database is already up to date");

                return Ok(()); // Continue without error
            }
        }
    };

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
            let preview = trimmed
                .chars()
                .take(100)
                .collect::<String>()
                .replace('\n', " ");
            tracing::info!("Executing migration statement #{}: {}...", idx, preview);

            conn.execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                format!("{};", trimmed),
            ))
            .await
            .with_context(|| {
                format!(
                    "Failed to execute statement #{}: {}",
                    idx,
                    trimmed.lines().take(3).collect::<Vec<_>>().join(" ")
                )
            })?;
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
