use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::system::auth::{api, context::use_auth, storage};

#[component]
pub fn LoginPage() -> impl IntoView {
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error_message = RwSignal::new(Option::<String>::None);
    let is_loading = RwSignal::new(false);

    let (_, set_auth_state) = use_auth();

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();

        let username_val = username.get();
        let password_val = password.get();

        is_loading.set(true);
        error_message.set(None);

        spawn_local(async move {
            match api::login(username_val, password_val).await {
                Ok(response) => {
                    // Save tokens
                    storage::save_access_token(&response.access_token);
                    storage::save_refresh_token(&response.refresh_token);

                    // Update auth state - это автоматически переключит на MainLayout
                    set_auth_state.set(crate::system::auth::context::AuthState {
                        access_token: Some(response.access_token),
                        user_info: Some(response.user),
                    });

                    is_loading.set(false);
                }
                Err(e) => {
                    error_message.set(Some(format!("Login failed: {}", e)));
                    is_loading.set(false);
                }
            }
        });
    };

    view! {
        <div class="modern-login-container">
            // Canvas for particle animation
            <canvas id="particle-canvas" class="particle-canvas"></canvas>

            <div class="modern-login-box">
                // Logo with double circle and animated ring
                <div class="modern-login-icon">
                    <div class="modern-logo-wrapper">
                        <div class="modern-logo-outer">
                            <div class="modern-logo-inner">
                                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"
                                     stroke-width="1.5" stroke="currentColor">
                                    <path stroke-linecap="round" stroke-linejoin="round"
                                          d="M12 2L2 7L12 12L22 7L12 2Z" class="fill-white/20" />
                                    <path stroke-linecap="round" stroke-linejoin="round"
                                          d="M2 17L12 22L22 17" />
                                    <path stroke-linecap="round" stroke-linejoin="round"
                                          d="M2 12L12 17L22 12" />
                                </svg>
                            </div>
                        </div>
                        <div class="modern-logo-ring"></div>
                    </div>

                    <h2>"Вход в систему MPI"</h2>
                    <p class="subtitle">"Платформа интеграции маркетплейсов"</p>
                </div>

                <Show when=move || error_message.get().is_some()>
                    <div class="modern-error-message">
                        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"
                             stroke-width="2" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round"
                                  d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z" />
                        </svg>
                        <span>{move || error_message.get().unwrap_or_default()}</span>
                    </div>
                </Show>

                <form on:submit=on_submit>
                    <div class="form-group">
                        <label for="username">"Логин"</label>
                        <div class="modern-input-wrapper">
                            <input
                                type="text"
                                id="username"
                                placeholder="Введите ваш логин"
                                value=move || username.get()
                                on:input=move |ev| username.set(event_target_value(&ev))
                                required
                                disabled=move || is_loading.get()
                                autocomplete="username"
                            />
                            <svg class="modern-input-icon" xmlns="http://www.w3.org/2000/svg"
                                 fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor">
                                <path stroke-linecap="round" stroke-linejoin="round"
                                      d="M15.75 6a3.75 3.75 0 11-7.5 0 3.75 3.75 0 017.5 0zM4.501 20.118a7.5 7.5 0 0114.998 0A17.933 17.933 0 0112 21.75c-2.676 0-5.216-.584-7.499-1.632z" />
                            </svg>
                        </div>
                    </div>

                    <div class="form-group">
                        <label for="password">"Пароль"</label>
                        <div class="modern-input-wrapper">
                            <input
                                type="password"
                                id="password"
                                placeholder="Введите ваш пароль"
                                value=move || password.get()
                                on:input=move |ev| password.set(event_target_value(&ev))
                                required
                                disabled=move || is_loading.get()
                                autocomplete="current-password"
                            />
                            <svg class="modern-input-icon" xmlns="http://www.w3.org/2000/svg"
                                 fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor">
                                <path stroke-linecap="round" stroke-linejoin="round"
                                      d="M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" />
                            </svg>
                        </div>
                    </div>

                    <button
                        type="submit"
                        disabled=move || is_loading.get()
                    >
                        <Show
                            when=move || is_loading.get()
                            fallback=move || view! {
                                <span>"Войти в систему"</span>
                                <svg class="modern-btn-arrow" xmlns="http://www.w3.org/2000/svg"
                                     fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor">
                                    <path stroke-linecap="round" stroke-linejoin="round"
                                          d="M13.5 4.5L21 12m0 0l-7.5 7.5M21 12H3" />
                                </svg>
                            }
                        >
                            <div class="modern-spinner"></div>
                        </Show>
                    </button>
                </form>

                // Footer with "Forgot password" link
                <div class="modern-login-footer">
                    <p>
                        "Забыли пароль? "
                        <a href="#" onclick="event.preventDefault()">"Восстановить доступ"</a>
                    </p>
                </div>
            </div>

            // Version badge outside the card
            <div class="modern-version-badge">
                <span>"MPI v1.0.0"</span>
            </div>
        </div>
    }
}
