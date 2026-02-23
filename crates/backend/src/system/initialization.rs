use anyhow::Result;

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
