use contracts::system::users::{ChangePasswordDto, CreateUserDto, UpdateUserDto};
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

use crate::shared::components::card_animated::CardAnimated;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_SYSTEM;
use crate::system::auth::guard::RequireAdmin;
use crate::system::users::api;

#[component]
pub fn UserDetailsPage(user_id: String) -> impl IntoView {
    let user_id = StoredValue::new(user_id);
    view! {
        <RequireAdmin>
            <UserDetailsInner user_id=user_id.get_value() />
        </RequireAdmin>
    }
}

#[component]
fn UserDetailsInner(user_id: String) -> impl IntoView {
    let (loading, set_loading) = signal(true);
    let (saving, set_saving) = signal(false);
    let (pwd_saving, set_pwd_saving) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (success, set_success) = signal::<Option<String>>(None);
    let (pwd_error, set_pwd_error) = signal::<Option<String>>(None);
    let (pwd_success, set_pwd_success) = signal::<Option<String>>(None);

    let username_display = RwSignal::new(String::new());
    let email = RwSignal::new(String::new());
    let full_name = RwSignal::new(String::new());
    let is_active = RwSignal::new(true);
    let is_admin = RwSignal::new(false);
    let primary_role_code = RwSignal::new("viewer".to_string());

    let new_password = RwSignal::new(String::new());
    let confirm_password = RwSignal::new(String::new());

    let id_for_load = user_id.clone();
    Effect::new(move |_| {
        let uid = id_for_load.clone();
        spawn_local(async move {
            set_loading.set(true);
            match api::get_user(&uid).await {
                Ok(user) => {
                    username_display.set(user.username.clone());
                    email.set(user.email.unwrap_or_default());
                    full_name.set(user.full_name.unwrap_or_default());
                    is_active.set(user.is_active);
                    is_admin.set(user.is_admin);
                    primary_role_code.set(user.primary_role_code);
                }
                Err(e) => set_error.set(Some(format!("Не удалось загрузить пользователя: {}", e))),
            }
            set_loading.set(false);
        });
    });

    let id_for_save = user_id.clone();
    let on_save = move |_| {
        set_saving.set(true);
        set_error.set(None);
        set_success.set(None);
        let uid = id_for_save.clone();
        let dto = UpdateUserDto {
            id: uid.clone(),
            email: {
                let v = email.get();
                if v.trim().is_empty() {
                    None
                } else {
                    Some(v)
                }
            },
            full_name: {
                let v = full_name.get();
                if v.trim().is_empty() {
                    None
                } else {
                    Some(v)
                }
            },
            is_active: is_active.get(),
            is_admin: is_admin.get(),
            primary_role_code: primary_role_code.get(),
        };
        spawn_local(async move {
            match api::update_user(dto).await {
                Ok(_) => set_success.set(Some("Изменения сохранены".to_string())),
                Err(e) => set_error.set(Some(format!("Ошибка сохранения: {}", e))),
            }
            set_saving.set(false);
        });
    };

    let id_for_pwd = user_id.clone();
    let on_change_password = move |_| {
        let np = new_password.get();
        let cp = confirm_password.get();
        if np.is_empty() {
            set_pwd_error.set(Some("Введите новый пароль".to_string()));
            return;
        }
        if np != cp {
            set_pwd_error.set(Some("Пароли не совпадают".to_string()));
            return;
        }
        set_pwd_saving.set(true);
        set_pwd_error.set(None);
        set_pwd_success.set(None);
        let uid = id_for_pwd.clone();
        spawn_local(async move {
            let dto = ChangePasswordDto {
                user_id: uid,
                old_password: None,
                new_password: np,
            };
            match api::change_password(dto).await {
                Ok(_) => {
                    new_password.set(String::new());
                    confirm_password.set(String::new());
                    set_pwd_success.set(Some("Пароль изменён".to_string()));
                }
                Err(e) => set_pwd_error.set(Some(format!("Ошибка: {}", e))),
            }
            set_pwd_saving.set(false);
        });
    };

    view! {
        <PageFrame page_id="sys_users--detail" category=PAGE_CAT_SYSTEM>
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">
                        {move || if loading.get() {
                            "Пользователь".to_string()
                        } else {
                            format!("Пользователь: {}", username_display.get())
                        }}
                    </h1>
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=on_save
                        disabled=Signal::derive(move || saving.get() || loading.get())
                    >
                        {icon("save")}
                        {move || if saving.get() { " Сохранение..." } else { " Сохранить" }}
                    </Button>
                </div>
            </div>

            {move || error.get().map(|e| view! { <div class="alert alert--error" style="margin: var(--spacing-sm) var(--spacing-md);">{e}</div> })}
            {move || success.get().map(|s| view! { <div class="alert alert--success" style="margin: var(--spacing-sm) var(--spacing-md);">{s}</div> })}

            <div class="detail-grid">
                // ── Левая колонка ──────────────────────────────────────────
                <div class="detail-grid__col">
                    <CardAnimated delay_ms=0 nav_id="sys_user_details_main">
                        <h4 class="details-section__title">"Основные данные"</h4>

                        <div class="form__group">
                            <label class="form__label">"Логин"</label>
                            <Input
                                value=username_display
                                disabled=Signal::derive(move || true)
                            />
                        </div>

                        <div class="form__group">
                            <label class="form__label">"ФИО"</label>
                            <Input
                                value=full_name
                                placeholder="Полное имя"
                                disabled=Signal::derive(move || saving.get() || loading.get())
                            />
                        </div>

                        <div class="form__group">
                            <label class="form__label">"Email"</label>
                            <Input
                                value=email
                                input_type=InputType::Email
                                placeholder="user@example.com"
                                disabled=Signal::derive(move || saving.get() || loading.get())
                            />
                        </div>

                        <div class="form__group">
                            <label class="form__label">"Основная роль"</label>
                            <select
                                class="thaw-input"
                                style="width: 100%;"
                                prop:value=move || primary_role_code.get()
                                on:change=move |ev| primary_role_code.set(event_target_value(&ev))
                                disabled=move || saving.get() || loading.get()
                            >
                                <option value="viewer">"viewer — только просмотр"</option>
                                <option value="operator">"operator — операционная работа"</option>
                                <option value="manager">"manager — полный доступ"</option>
                                <option value="admin">"admin — администратор системы"</option>
                            </select>
                        </div>

                        <div style="display: flex; gap: var(--spacing-lg); margin-top: var(--spacing-sm);">
                            <Checkbox checked=is_active label="Активен" />
                            <Checkbox checked=is_admin label="Суперадмин (is_admin bypass)" />
                        </div>
                    </CardAnimated>
                </div>

                // ── Правая колонка ─────────────────────────────────────────
                <div class="detail-grid__col">
                    <CardAnimated delay_ms=80 nav_id="sys_user_details_password">
                        <h4 class="details-section__title">"Смена пароля"</h4>

                        {move || pwd_error.get().map(|e| view! { <div class="alert alert--error">{e}</div> })}
                        {move || pwd_success.get().map(|s| view! { <div class="alert alert--success">{s}</div> })}

                        <div class="form__group">
                            <label class="form__label">"Новый пароль"</label>
                            <Input
                                value=new_password
                                input_type=InputType::Password
                                placeholder="Новый пароль"
                                disabled=Signal::derive(move || pwd_saving.get())
                            />
                        </div>

                        <div class="form__group">
                            <label class="form__label">"Подтверждение пароля"</label>
                            <Input
                                value=confirm_password
                                input_type=InputType::Password
                                placeholder="Повторите пароль"
                                disabled=Signal::derive(move || pwd_saving.get())
                            />
                        </div>
                        <div style="display: flex; margin-top: var(--spacing-lg); justify-content: center; align-items: center; width: 100%;">
                        <Button
                            appearance=ButtonAppearance::Primary
                            on_click=on_change_password
                            disabled=Signal::derive(move || pwd_saving.get())
                        >
                            {icon("key")}
                            {move || if pwd_saving.get() { " Сохранение..." } else { " Изменить пароль" }}
                        </Button>
                        </div>
                    </CardAnimated>
                </div>
            </div>
        </PageFrame>
    }
}

#[component]
pub fn CreateUserPage(on_close: Callback<()>) -> impl IntoView {
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let confirm_password = RwSignal::new(String::new());
    let email = RwSignal::new(String::new());
    let full_name = RwSignal::new(String::new());
    let is_admin = RwSignal::new(false);
    let primary_role_code = RwSignal::new("viewer".to_string());
    let (saving, set_saving) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);

    let on_save = move |_| {
        let u = username.get();
        let p = password.get();
        let cp = confirm_password.get();

        if u.trim().is_empty() {
            set_error.set(Some("Логин обязателен".to_string()));
            return;
        }
        if p.trim().is_empty() {
            set_error.set(Some("Пароль обязателен".to_string()));
            return;
        }
        if p != cp {
            set_error.set(Some("Пароли не совпадают".to_string()));
            return;
        }

        let dto = CreateUserDto {
            username: u,
            password: p,
            email: {
                let v = email.get();
                if v.trim().is_empty() {
                    None
                } else {
                    Some(v)
                }
            },
            full_name: {
                let v = full_name.get();
                if v.trim().is_empty() {
                    None
                } else {
                    Some(v)
                }
            },
            is_admin: is_admin.get(),
            primary_role_code: primary_role_code.get(),
        };

        set_saving.set(true);
        set_error.set(None);

        spawn_local(async move {
            match api::create_user(dto).await {
                Ok(_) => on_close.run(()),
                Err(e) => {
                    set_error.set(Some(format!("Ошибка создания: {}", e)));
                    set_saving.set(false);
                }
            }
        });
    };

    view! {
        <RequireAdmin>
            <PageFrame page_id="sys_user_details" category=PAGE_CAT_SYSTEM>
                <div class="page__header">
                    <div class="page__header-left">
                        <h1 class="page__title">"Новый пользователь"</h1>
                    </div>
                    <div class="page__header-right">
                        <Button
                            appearance=ButtonAppearance::Primary
                            on_click=on_save
                            disabled=Signal::derive(move || saving.get())
                        >
                            {icon("save")}
                            {move || if saving.get() { " Сохранение..." } else { " Создать" }}
                        </Button>
                    </div>
                </div>

                {move || error.get().map(|e| view! {
                    <div class="alert alert--error" style="margin: var(--spacing-sm) var(--spacing-md);">{e}</div>
                })}

                <div class="detail-grid">
                    // ── Левая колонка ──────────────────────────────────────────
                    <div class="detail-grid__col">
                        <CardAnimated delay_ms=0 nav_id="sys_user_details_main">
                            <h4 class="details-section__title">"Основные данные"</h4>

                            <div class="form__group">
                                <label class="form__label">
                                    "Логин "
                                    <span style="color: var(--color-error);">"*"</span>
                                </label>
                                <Input
                                    value=username
                                    placeholder="Введите логин"
                                    disabled=Signal::derive(move || saving.get())
                                />
                            </div>

                            <div class="form__group">
                                <label class="form__label">"ФИО"</label>
                                <Input
                                    value=full_name
                                    placeholder="Полное имя"
                                    disabled=Signal::derive(move || saving.get())
                                />
                            </div>

                            <div class="form__group">
                                <label class="form__label">"Email"</label>
                                <Input
                                    value=email
                                    input_type=InputType::Email
                                    placeholder="user@example.com"
                                    disabled=Signal::derive(move || saving.get())
                                />
                            </div>

                            <div class="form__group">
                                <label class="form__label">"Основная роль"</label>
                                <select
                                    class="thaw-input"
                                    style="width: 100%;"
                                    prop:value=move || primary_role_code.get()
                                    on:change=move |ev| primary_role_code.set(event_target_value(&ev))
                                    disabled=move || saving.get()
                                >
                                    <option value="viewer">"viewer — только просмотр"</option>
                                    <option value="operator">"operator — операционная работа"</option>
                                    <option value="manager">"manager — полный доступ"</option>
                                    <option value="admin">"admin — администратор системы"</option>
                                </select>
                            </div>

                            <div style="display: flex; gap: var(--spacing-lg); margin-top: var(--spacing-sm);">
                                <Checkbox checked=is_admin label="Суперадмин (is_admin bypass)" />
                            </div>
                        </CardAnimated>
                    </div>

                    // ── Правая колонка ─────────────────────────────────────────
                    <div class="detail-grid__col">
                        <CardAnimated delay_ms=80 nav_id="sys_user_details_password">
                            <h4 class="details-section__title">"Пароль"</h4>

                            <div class="form__group">
                                <label class="form__label">
                                    "Пароль "
                                    <span style="color: var(--color-error);">"*"</span>
                                </label>
                                <Input
                                    value=password
                                    input_type=InputType::Password
                                    placeholder="Введите пароль"
                                    disabled=Signal::derive(move || saving.get())
                                />
                            </div>

                            <div class="form__group">
                                <label class="form__label">
                                    "Подтверждение пароля "
                                    <span style="color: var(--color-error);">"*"</span>
                                </label>
                                <Input
                                    value=confirm_password
                                    input_type=InputType::Password
                                    placeholder="Повторите пароль"
                                    disabled=Signal::derive(move || saving.get())
                                />
                            </div>
                        </CardAnimated>
                    </div>
                </div>
            </PageFrame>
        </RequireAdmin>
    }
}

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
    let primary_role_code = RwSignal::new("viewer".to_string());
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
            primary_role_code: primary_role_code.get(),
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
                        <Label>"Основная роль"</Label>
                        <select
                            class="thaw-input"
                            style="width: 100%;"
                            prop:value=move || primary_role_code.get()
                            on:change=move |ev| primary_role_code.set(event_target_value(&ev))
                        >
                            <option value="viewer">"viewer — только просмотр"</option>
                            <option value="operator">"operator — операционная работа"</option>
                            <option value="manager">"manager — полный доступ"</option>
                            <option value="admin">"admin — администратор системы"</option>
                        </select>
                    </div>

                    <div class="form__group">
                        <Checkbox checked=is_admin label="Суперадмин (is_admin bypass)" />
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
