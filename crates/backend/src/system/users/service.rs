use anyhow::Result;
use chrono::Utc;
use contracts::system::users::{ChangePasswordDto, CreateUserDto, UpdateUserDto, User};

use super::repository;
use crate::system::auth::password;

/// Create a new user
pub async fn create(dto: CreateUserDto, created_by: Option<String>) -> Result<String> {
    // Validate username
    if dto.username.trim().is_empty() {
        return Err(anyhow::anyhow!("Username cannot be empty"));
    }

    // Check if username already exists
    if let Some(_) = repository::get_by_username(&dto.username).await? {
        return Err(anyhow::anyhow!("Username already exists"));
    }

    // Validate email if provided
    if let Some(ref email) = dto.email {
        if !email.trim().is_empty() {
            // Basic email validation
            if !email.contains('@') {
                return Err(anyhow::anyhow!("Invalid email format"));
            }
        }
    }

    // Validate password strength
    password::validate_password_strength(&dto.password)?;

    // Hash password
    let password_hash = password::hash_password(&dto.password)?;

    // Create user
    let user_id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let user = User {
        id: user_id.clone(),
        username: dto.username,
        email: dto.email,
        full_name: dto.full_name,
        is_active: true,
        is_admin: dto.is_admin,
        created_at: now.clone(),
        updated_at: now,
        last_login_at: None,
        created_by,
    };

    repository::create_with_password(&user, &password_hash).await?;

    Ok(user_id)
}

/// Update user
pub async fn update(dto: UpdateUserDto) -> Result<()> {
    // Get existing user
    let mut _user = repository::get_by_id(&dto.id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("User not found"))?;

    // Validate email if provided
    if let Some(ref email) = dto.email {
        if !email.trim().is_empty() && !email.contains('@') {
            return Err(anyhow::anyhow!("Invalid email format"));
        }
    }

    // Update fields
    _user.email = dto.email;
    _user.full_name = dto.full_name;
    _user.is_active = dto.is_active;
    _user.is_admin = dto.is_admin;
    _user.updated_at = Utc::now().to_rfc3339();

    repository::update(&_user).await?;

    Ok(())
}

/// Delete user
pub async fn delete(id: &str) -> Result<bool> {
    repository::delete(id).await
}

/// Get user by ID
pub async fn get_by_id(id: &str) -> Result<Option<User>> {
    repository::get_by_id(id).await
}

/// List all users
pub async fn list_all() -> Result<Vec<User>> {
    repository::list_all().await
}

/// Change user password
pub async fn change_password(dto: ChangePasswordDto, requester_id: &str) -> Result<()> {
    // Get user
    let user = repository::get_by_id(&dto.user_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("User not found"))?;

    // Get requester
    let requester = repository::get_by_id(requester_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Requester not found"))?;

    // Check permissions
    if dto.user_id != requester_id {
        // Changing someone else's password - must be admin
        if !requester.is_admin {
            return Err(anyhow::anyhow!("Permission denied"));
        }

        // Admin can change without old password
    } else {
        // Changing own password - verify old password if provided
        if let Some(ref old_password) = dto.old_password {
            let current_hash = repository::get_password_hash(&dto.user_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Password hash not found"))?;

            if !password::verify_password(old_password, &current_hash)? {
                return Err(anyhow::anyhow!("Invalid old password"));
            }
        }
    }

    // Validate new password strength
    password::validate_password_strength(&dto.new_password)?;

    // Hash new password
    let new_hash = password::hash_password(&dto.new_password)?;

    // Update password
    repository::update_password(&dto.user_id, &new_hash).await?;

    Ok(())
}

/// Verify user credentials (for login)
pub async fn verify_credentials(username: &str, password: &str) -> Result<Option<User>> {
    // Get user by username
    let user = match repository::get_by_username(username).await? {
        Some(u) => u,
        None => return Ok(None),
    };

    // Check if user is active
    if !user.is_active {
        return Err(anyhow::anyhow!("User account is inactive"));
    }

    // Get password hash
    let password_hash = repository::get_password_hash(&user.id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Password hash not found"))?;

    // Verify password
    if !password::verify_password(password, &password_hash)? {
        return Ok(None);
    }

    // Update last login
    let _ = repository::update_last_login(&user.id).await;

    Ok(Some(user))
}
