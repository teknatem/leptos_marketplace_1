use contracts::system::users::CreateUserDto;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

use crate::shared::icons::icon;
use crate::system::users::api;

#[component]
pub fn CreateUserForm<F1, F2>(on_close: F1, on_created: F2) -> impl IntoView
where
    F1: Fn() + 'static + Copy + Send + Sync,
    F2: Fn() + 'static + Copy + Send + Sync,
{
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let email = RwSignal::new(String::new());
    let full_name = RwSignal::new(String::new());
    let is_admin = RwSignal::new(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (is_saving, set_is_saving) = signal(false);

    let on_save = move |_| {
        if username.get().trim().is_empty() {
            set_error.set(Some("Логин обязателен".to_string()));
            return;
        }
        if password.get().trim().is_empty() {
            set_error.set(Some("Пароль обязателен".to_string()));
            return;
        }

        let dto = CreateUserDto {
            username: username.get(),
            password: password.get(),
            email: if email.get().is_empty() { None } else { Some(email.get()) },
            full_name: if full_name.get().is_empty() { None } else { Some(full_name.get()) },
            is_admin: is_admin.get(),
        };

        set_is_saving.set(true);
        set_error.set(None);

        spawn_local(async move {
            match api::create_user(dto).await {
                Ok(_) => on_created(),
                Err(e) => {
                    set_error.set(Some(format!("Ошибка создания: {}", e)));
                    set_is_saving.set(false);
                }
            }
        });
    };

    view! {
        <div class="modal-overlay" on:click=move |_| on_close()>
            <div class="modal" on:click=move |ev| ev.stop_propagation()>
                <div class="modal-header">
                    <h2 class="modal-title">"Новый пользователь"</h2>
                    <Button
                        appearance=ButtonAppearance::Subtle
                        on_click=move |_| on_close()
                    >
                        {icon("x")}
                    </Button>
                </div>

                <div class="modal-body">
                    {move || error.get().map(|e| view! { <div class="alert alert--error">{e}</div> })}

                    <div class="form__group">
                        <Label>
                            "Логин "
                            <span style="color: var(--color-error);">"*"</span>
                        </Label>
                        <Input
                            value=username
                            placeholder="Введите логин"
                            disabled=Signal::derive(move || is_saving.get())
                        />
                    </div>

                    <div class="form__group">
                        <Label>
                            "Пароль "
                            <span style="color: var(--color-error);">"*"</span>
                        </Label>
                        <Input
                            value=password
                            input_type=InputType::Password
                            placeholder="Введите пароль"
                            disabled=Signal::derive(move || is_saving.get())
                        />
                    </div>

                    <div class="form__group">
                        <Label>"Email"</Label>
                        <Input
                            value=email
                            input_type=InputType::Email
                            placeholder="user@example.com"
                            disabled=Signal::derive(move || is_saving.get())
                        />
                    </div>

                    <div class="form__group">
                        <Label>"ФИО"</Label>
                        <Input
                            value=full_name
                            placeholder="Полное имя"
                            disabled=Signal::derive(move || is_saving.get())
                        />
                    </div>

                    <div class="form__group">
                        <Checkbox checked=is_admin label="Администратор" />
                    </div>
                </div>

                <div class="modal-footer">
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| on_close()
                        disabled=Signal::derive(move || is_saving.get())
                    >
                        "Отмена"
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=on_save
                        disabled=Signal::derive(move || is_saving.get())
                    >
                        {move || if is_saving.get() { "Сохранение..." } else { "Создать" }}
                    </Button>
                </div>
            </div>
        </div>
    }
}
