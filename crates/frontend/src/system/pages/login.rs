use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::system::auth::{api, storage, context::use_auth};

#[component]
pub fn LoginPage() -> impl IntoView {
    let (username, set_username) = create_signal(String::new());
    let (password, set_password) = create_signal(String::new());
    let (error_message, set_error_message) = create_signal(Option::<String>::None);
    let (is_loading, set_is_loading) = create_signal(false);

    let (_, set_auth_state) = use_auth();

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        
        let username_val = username.get();
        let password_val = password.get();

        set_is_loading.set(true);
        set_error_message.set(None);

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
                    
                    set_is_loading.set(false);
                }
                Err(e) => {
                    set_error_message.set(Some(format!("Login failed: {}", e)));
                    set_is_loading.set(false);
                }
            }
        });
    };

    view! {
        <div class="login-container">
            <div class="login-box">
                <h1>"Leptos Marketplace"</h1>
                <h2>"Вход в систему"</h2>

                <Show when=move || error_message.get().is_some()>
                    <div class="error-message">
                        {move || error_message.get().unwrap_or_default()}
                    </div>
                </Show>

                <form on:submit=on_submit>
                    <div class="form-group">
                        <label for="username">"Логин"</label>
                        <input
                            type="text"
                            id="username"
                            placeholder="admin"
                            value=move || username.get()
                            on:input=move |ev| set_username.set(event_target_value(&ev))
                            required
                            disabled=move || is_loading.get()
                        />
                    </div>

                    <div class="form-group">
                        <label for="password">"Пароль"</label>
                        <input
                            type="password"
                            id="password"
                            placeholder="admin"
                            value=move || password.get()
                            on:input=move |ev| set_password.set(event_target_value(&ev))
                            required
                            disabled=move || is_loading.get()
                        />
                    </div>

                    <button
                        type="submit"
                        class="btn-primary"
                        disabled=move || is_loading.get()
                    >
                        {move || if is_loading.get() { "Вход..." } else { "Войти" }}
                    </button>
                </form>

                <div class="login-info">
                    <p>"По умолчанию:"</p>
                    <p>"Логин: " <strong>"admin"</strong></p>
                    <p>"Пароль: " <strong>"admin"</strong></p>
                </div>
            </div>
        </div>
    }
}

