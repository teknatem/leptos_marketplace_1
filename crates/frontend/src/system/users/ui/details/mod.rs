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
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let email = RwSignal::new(String::new());
    let full_name = RwSignal::new(String::new());
    let is_admin = RwSignal::new(false);
    let error_message = RwSignal::new(Option::<String>::None);
    let is_saving = RwSignal::new(false);

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

        is_saving.set(true);
        error_message.set(None);

        spawn_local(async move {
            match api::create_user(dto).await {
                Ok(_) => {
                    on_created();
                }
                Err(e) => {
                    error_message.set(Some(format!("Failed to create user: {}", e)));
                    is_saving.set(false);
                }
            }
        });
    };

    view! {
        <div class="modal-overlay" on:click=move |_| on_close()>
            <div class="modal-content" on:click=move |ev| ev.stop_propagation()>
                <div class="modal-header">
                    <h3>"Создать пользователя"</h3>
                    <button class="button--ghost" on:click=move |_| on_close()>"×"</button>
                </div>

                <div class="modal-body">
                    <Show when=move || error_message.get().is_some()>
                        <div class="error-message">
                            {move || error_message.get().unwrap_or_default()}
                        </div>
                    </Show>

                    <form id="create-user-form" on:submit=on_submit>
                    <div class="form__group">
                        <label for="username">"Логин *"</label>
                        <input
                            type="text"
                            id="username"
                            value=move || username.get()
                            on:input=move |ev| username.set(event_target_value(&ev))
                            required
                            disabled=move || is_saving.get()
                        />
                    </div>

                    <div class="form__group">
                        <label for="password">"Пароль *"</label>
                        <input
                            type="password"
                            id="password"
                            value=move || password.get()
                            on:input=move |ev| password.set(event_target_value(&ev))
                            required
                            disabled=move || is_saving.get()
                        />
                    </div>

                    <div class="form__group">
                        <label for="email">"Email"</label>
                        <input
                            type="email"
                            id="email"
                            value=move || email.get()
                            on:input=move |ev| email.set(event_target_value(&ev))
                            disabled=move || is_saving.get()
                        />
                    </div>

                    <div class="form__group">
                        <label for="full_name">"ФИО"</label>
                        <input
                            type="text"
                            id="full_name"
                            value=move || full_name.get()
                            on:input=move |ev| full_name.set(event_target_value(&ev))
                            disabled=move || is_saving.get()
                        />
                    </div>

                    <div class="form__group">
                        <label>
                            <input
                                type="checkbox"
                                checked=move || is_admin.get()
                                on:change=move |ev| is_admin.set(event_target_checked(&ev))
                                disabled=move || is_saving.get()
                            />
                            " Администратор"
                        </label>
                    </div>
                    </form>
                </div>

                <div class="form-actions">
                    <button
                        type="button"
                        class="button--secondary"
                        on:click=move |_| on_close()
                        disabled=move || is_saving.get()
                    >
                        "Отмена"
                    </button>
                    <button
                        type="submit"
                        form="create-user-form"
                        class="button button--primary"
                        disabled=move || is_saving.get()
                    >
                        {move || if is_saving.get() { "Сохранение..." } else { "Создать" }}
                    </button>
                </div>
            </div>
        </div>
    }
}
