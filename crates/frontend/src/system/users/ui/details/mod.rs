use contracts::system::users::CreateUserDto;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::system::users::api;

#[component]
pub fn CreateUserForm<F1, F2>(on_close: F1, on_created: F2) -> impl IntoView
where
    F1: Fn() + 'static + Copy,
    F2: Fn() + 'static + Copy,
{
    let (username, set_username) = create_signal(String::new());
    let (password, set_password) = create_signal(String::new());
    let (email, set_email) = create_signal(String::new());
    let (full_name, set_full_name) = create_signal(String::new());
    let (is_admin, set_is_admin) = create_signal(false);
    let (error_message, set_error_message) = create_signal(Option::<String>::None);
    let (is_saving, set_is_saving) = create_signal(false);

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();

        let dto = CreateUserDto {
            username: username.get(),
            password: password.get(),
            email: if email.get().is_empty() {
                None
            } else {
                Some(email.get())
            },
            full_name: if full_name.get().is_empty() {
                None
            } else {
                Some(full_name.get())
            },
            is_admin: is_admin.get(),
        };

        set_is_saving.set(true);
        set_error_message.set(None);

        spawn_local(async move {
            match api::create_user(dto).await {
                Ok(_) => {
                    on_created();
                }
                Err(e) => {
                    set_error_message.set(Some(format!("Failed to create user: {}", e)));
                    set_is_saving.set(false);
                }
            }
        });
    };

    view! {
        <div class="modal-overlay" on:click=move |_| on_close()>
            <div class="modal-content" on:click=move |ev| ev.stop_propagation()>
                <div class="modal-header">
                    <h3>"Создать пользователя"</h3>
                    <button class="btn-close" on:click=move |_| on_close()>"×"</button>
                </div>

                <Show when=move || error_message.get().is_some()>
                    <div class="error-message">
                        {move || error_message.get().unwrap_or_default()}
                    </div>
                </Show>

                <form on:submit=on_submit>
                    <div class="form-group">
                        <label for="username">"Username *"</label>
                        <input
                            type="text"
                            id="username"
                            value=move || username.get()
                            on:input=move |ev| set_username.set(event_target_value(&ev))
                            required
                            disabled=move || is_saving.get()
                        />
                    </div>

                    <div class="form-group">
                        <label for="password">"Password *"</label>
                        <input
                            type="password"
                            id="password"
                            value=move || password.get()
                            on:input=move |ev| set_password.set(event_target_value(&ev))
                            required
                            disabled=move || is_saving.get()
                        />
                    </div>

                    <div class="form-group">
                        <label for="email">"Email"</label>
                        <input
                            type="email"
                            id="email"
                            value=move || email.get()
                            on:input=move |ev| set_email.set(event_target_value(&ev))
                            disabled=move || is_saving.get()
                        />
                    </div>

                    <div class="form-group">
                        <label for="full_name">"Full Name"</label>
                        <input
                            type="text"
                            id="full_name"
                            value=move || full_name.get()
                            on:input=move |ev| set_full_name.set(event_target_value(&ev))
                            disabled=move || is_saving.get()
                        />
                    </div>

                    <div class="form-group">
                        <label>
                            <input
                                type="checkbox"
                                checked=move || is_admin.get()
                                on:change=move |ev| set_is_admin.set(event_target_checked(&ev))
                                disabled=move || is_saving.get()
                            />
                            " Administrator"
                        </label>
                    </div>

                    <div class="form-actions">
                        <button
                            type="button"
                            class="btn-secondary"
                            on:click=move |_| on_close()
                            disabled=move || is_saving.get()
                        >
                            "Cancel"
                        </button>
                        <button
                            type="submit"
                            class="btn-primary"
                            disabled=move || is_saving.get()
                        >
                            {move || if is_saving.get() { "Saving..." } else { "Create User" }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}

