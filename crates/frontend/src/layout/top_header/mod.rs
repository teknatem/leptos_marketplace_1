//! TopHeader component - application top navigation bar.
//!
//! Contains:
//! - Toggle buttons for sidebar and right panel
//! - Application title
//! - User info and actions
//! - Theme selector
//! - Notifications and settings buttons

use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use crate::shared::theme::ThemeSelect;
use crate::system::auth::context::{do_logout, use_auth};
use leptos::prelude::*;
use leptos::task::spawn_local;

/// TopHeader component - main application top bar.
///
/// Uses AppGlobalContext for sidebar/panel visibility control.
#[component]
pub fn TopHeader() -> impl IntoView {
    // Get global context for sidebar/panel toggles
    let ctx =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    // Get auth context for user info
    let (auth_state, set_auth_state) = use_auth();

    let toggle_sidebar = move |_| {
        ctx.toggle_left();
    };

    let toggle_right_panel = move |_| {
        ctx.toggle_right();
    };

    let logout = move |_| {
        spawn_local(async move {
            let _ = do_logout(set_auth_state).await;
        });
    };

    // Derive visibility states from context
    let is_sidebar_visible = move || ctx.left_open.get();
    let is_right_panel_visible = move || ctx.right_open.get();

    view! {
        <div class="top-header">
            // Left section - sidebar toggle and brand
            <div class="top-header__brand">
                <span class="top-header__title">"Marketplace Integrator"</span>
            </div>

            // Right section - actions
            <div class="top-header__actions">
                // Left panel toggle
                <button
                    class="top-header__icon-btn"
                    on:click=toggle_sidebar
                    title=move || if is_sidebar_visible() { "Скрыть навигацию" } else { "Показать навигацию" }
                >
                    {move || if is_sidebar_visible() {
                        icon("panel-left-close")
                    } else {
                        icon("panel-left-open")
                    }}
                </button>

                // Right panel toggle
                <button
                    class="top-header__icon-btn"
                    on:click=toggle_right_panel
                    title=move || if is_right_panel_visible() { "Скрыть правую панель" } else { "Показать правую панель" }
                >
                    {move || if is_right_panel_visible() {
                        icon("panel-right-close")
                    } else {
                        icon("panel-right-open")
                    }}
                </button>

                // Notifications
                <button class="top-header__icon-btn" title="Уведомления">
                    {icon("bell")}
                </button>

                // Settings
                <button class="top-header__icon-btn" title="Настройки">
                    {icon("settings")}
                </button>

                // Theme selector
                <ThemeSelect />

                // User info
                <div class="top-header__user">
                    {icon("user")}
                    <span>
                        {move || auth_state.get().user_info
                            .map(|u| u.username.clone())
                            .unwrap_or_else(|| "Гость".to_string())}
                    </span>
                </div>

                // Logout
                <button class="top-header__icon-btn" on:click=logout title="Выход">
                    {icon("log-out")}
                </button>
            </div>
        </div>
    }
}
