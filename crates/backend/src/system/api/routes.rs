use axum::{
    middleware,
    routing::{get, post},
    Router,
};

use super::handlers;
use crate::system::auth;

/// Конфигурация системных роутов приложения
pub fn configure_system_routes() -> Router {
    Router::new()
        // ========================================
        // HEALTH CHECK
        // ========================================
        .route("/health", get(|| async { "ok" }))
        // ========================================
        // SYSTEM AUTH ROUTES (PUBLIC)
        // ========================================
        .route(
            "/api/system/auth/login",
            post(handlers::auth::login),
        )
        .route(
            "/api/system/auth/refresh",
            post(handlers::auth::refresh),
        )
        .route(
            "/api/system/auth/logout",
            post(handlers::auth::logout),
        )
        // System auth routes (protected)
        .route(
            "/api/system/auth/me",
            get(handlers::auth::current_user)
                .layer(middleware::from_fn(auth::middleware::require_auth)),
        )
        // ========================================
        // SYSTEM USERS MANAGEMENT (admin only)
        // ========================================
        .route(
            "/api/system/users",
            get(handlers::users::list)
                .post(handlers::users::create)
                .layer(middleware::from_fn(auth::middleware::require_admin)),
        )
        .route(
            "/api/system/users/:id",
            get(handlers::users::get_by_id)
                .put(handlers::users::update)
                .delete(handlers::users::delete)
                .layer(middleware::from_fn(auth::middleware::require_admin)),
        )
        .route(
            "/api/system/users/:id/change-password",
            post(handlers::users::change_password)
                .layer(middleware::from_fn(auth::middleware::require_auth)),
        )
        // ========================================
        // SYSTEM SCHEDULED TASKS ROUTES
        // ========================================
        .route(
            "/api/sys/scheduled_tasks",
            get(handlers::tasks::list_scheduled_tasks)
                .post(handlers::tasks::create_scheduled_task)
                .layer(middleware::from_fn(auth::middleware::require_admin)),
        )
        .route(
            "/api/sys/scheduled_tasks/:id",
            get(handlers::tasks::get_scheduled_task)
                .put(handlers::tasks::update_scheduled_task)
                .delete(handlers::tasks::delete_scheduled_task)
                .layer(middleware::from_fn(auth::middleware::require_admin)),
        )
        .route(
            "/api/sys/scheduled_tasks/:id/toggle_enabled",
            post(handlers::tasks::toggle_scheduled_task_enabled)
                .layer(middleware::from_fn(auth::middleware::require_admin)),
        )
        .route(
            "/api/sys/scheduled_tasks/:id/progress/:session_id",
            get(handlers::tasks::get_task_progress)
                .layer(middleware::from_fn(auth::middleware::require_auth)),
        )
        .route(
            "/api/sys/scheduled_tasks/:id/log/:session_id",
            get(handlers::tasks::get_task_log)
                .layer(middleware::from_fn(auth::middleware::require_auth)),
        )
        // ========================================
        // UTILITIES
        // ========================================
        // Logs handlers
        .route(
            "/api/logs",
            get(handlers::logs::list_all)
                .post(handlers::logs::create)
                .delete(handlers::logs::clear_all),
        )
        // Form Settings handlers
        .route(
            "/api/form-settings/:form_key",
            get(handlers::form_settings::get_settings),
        )
        .route(
            "/api/form-settings",
            post(handlers::form_settings::save_settings),
        )
}
