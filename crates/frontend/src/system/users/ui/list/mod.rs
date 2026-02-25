mod state;

use contracts::system::users::{UpdateUserDto, User};
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::shared::date_utils::format_datetime;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator, sort_list, Sortable};
use crate::shared::table_utils::{init_column_resize, was_just_resizing};
use crate::system::auth::guard::RequireAdmin;
use crate::system::users::api;
use state::{create_state, UsersListState};

const TABLE_ID: &str = "sys-users-table";
const COLUMN_WIDTHS_KEY: &str = "sys_users_column_widths";

impl Sortable for User {
    fn compare_by_field(&self, other: &Self, field: &str) -> std::cmp::Ordering {
        match field {
            "username" => self
                .username
                .to_lowercase()
                .cmp(&other.username.to_lowercase()),
            "full_name" => self
                .full_name
                .as_deref()
                .unwrap_or("")
                .to_lowercase()
                .cmp(&other.full_name.as_deref().unwrap_or("").to_lowercase()),
            "email" => self
                .email
                .as_deref()
                .unwrap_or("")
                .to_lowercase()
                .cmp(&other.email.as_deref().unwrap_or("").to_lowercase()),
            "is_admin" => self.is_admin.cmp(&other.is_admin),
            "is_active" => self.is_active.cmp(&other.is_active),
            "created_at" => self.created_at.cmp(&other.created_at),
            "last_login_at" => self
                .last_login_at
                .as_deref()
                .unwrap_or("")
                .cmp(other.last_login_at.as_deref().unwrap_or("")),
            _ => self.username.cmp(&other.username),
        }
    }
}

#[component]
pub fn UsersListPage() -> impl IntoView {
    view! {
        <RequireAdmin>
            <UsersList />
        </RequireAdmin>
    }
}

fn recalc_pagination(state: &mut UsersListState) {
    let total_pages = if state.total_count == 0 {
        1
    } else {
        (state.total_count + state.page_size - 1) / state.page_size
    };
    state.total_pages = total_pages;
    if state.page >= total_pages {
        state.page = total_pages.saturating_sub(1);
    }
}

#[component]
fn UsersList() -> impl IntoView {
    let state = create_state();
    let error_message = RwSignal::new(Option::<String>::None);
    let loading = RwSignal::new(false);
    let show_create_form = RwSignal::new(false);
    let editing_user = RwSignal::new(Option::<User>::None);

    // Load users
    let load_data = move || {
        loading.set(true);
        error_message.set(None);

        let state = state.clone();
        spawn_local(async move {
            match api::fetch_users().await {
                Ok(mut data) => {
                    state.update(|s| {
                        s.items = data.clone();
                        s.total_count = data.len();
                        sort_list(&mut data, &s.sort_field, s.sort_ascending);
                        s.items = data;
                        recalc_pagination(s);
                        s.page = 0;
                        s.is_loaded = true;
                    });
                    loading.set(false);
                }
                Err(e) => {
                    error_message.set(Some(format!("Не удалось загрузить пользователей: {}", e)));
                    loading.set(false);
                }
            }
        });
    };

    // Initial load
    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            load_data();
        }
    });

    // Init column resize after load
    Effect::new(move |_| {
        if state.with(|s| s.is_loaded) {
            spawn_local(async move {
                TimeoutFuture::new(50).await;
                init_column_resize(TABLE_ID, COLUMN_WIDTHS_KEY);
            });
        }
    });

    // Derived values
    let page_items = Signal::derive(move || {
        let s = state.get();
        let start = s.page * s.page_size;
        let end = (start + s.page_size).min(s.items.len());
        s.items.get(start..end).unwrap_or(&[]).to_vec()
    });

    let toggle_sort = move |field: &'static str| {
        move |_| {
            if was_just_resizing() {
                return;
            }
            state.update(|s| {
                if s.sort_field == field {
                    s.sort_ascending = !s.sort_ascending;
                } else {
                    s.sort_field = field.to_string();
                    s.sort_ascending = true;
                }
                sort_list(&mut s.items, &s.sort_field, s.sort_ascending);
                s.page = 0;
            });
        }
    };

    let go_to_page = move |page: usize| {
        state.update(|s| {
            if page < s.total_pages {
                s.page = page;
            }
        });
    };

    let change_page_size = move |size: usize| {
        state.update(|s| {
            s.page_size = size;
            recalc_pagination(s);
            s.page = 0;
        });
    };

    let open_edit = move |user: User| {
        editing_user.set(Some(user));
    };

    let close_edit = move || {
        editing_user.set(None);
    };

    let on_saved = move || {
        editing_user.set(None);
        load_data();
    };

    let format_ts = |value: &str| format_datetime(value);
    let format_ts_opt = |value: &Option<String>| {
        value
            .as_deref()
            .map(format_datetime)
            .unwrap_or_else(|| "-".to_string())
    };

    view! {
        <div class="list-container">
            // Header row 1
            <div class="list-header-row gradient-header">
                <div class="header-left">
                    <h2 class="list-title">"👤 Пользователи"</h2>
                </div>
                <div class="pagination-controls">
                    <button
                        class="button button--icon-transparent"
                        on:click=move |_| go_to_page(0)
                        disabled=move || state.get().page == 0
                    >"⏮"</button>
                    <button
                        class="button button--icon-transparent"
                        on:click=move |_| {
                            let p = state.get().page;
                            if p > 0 { go_to_page(p - 1); }
                        }
                        disabled=move || state.get().page == 0
                    >"◀"</button>
                    <span class="pagination-info">
                        {move || {
                            let s = state.get();
                            format!("{} / {} ({})", s.page + 1, s.total_pages.max(1), s.total_count)
                        }}
                    </span>
                    <button
                        class="button button--icon-transparent"
                        on:click=move |_| {
                            let s = state.get();
                            if s.page + 1 < s.total_pages { go_to_page(s.page + 1); }
                        }
                        disabled=move || {
                            let s = state.get();
                            s.page + 1 >= s.total_pages
                        }
                    >"▶"</button>
                    <button
                        class="button button--icon-transparent"
                        on:click=move |_| {
                            let s = state.get();
                            if s.total_pages > 0 { go_to_page(s.total_pages - 1); }
                        }
                        disabled=move || {
                            let s = state.get();
                            s.page + 1 >= s.total_pages
                        }
                    >"⏭"</button>
                    <select
                        class="page-size-select"
                        on:change=move |ev| {
                            let val = event_target_value(&ev).parse().unwrap_or(50);
                            change_page_size(val);
                        }
                    >
                        <option value="25">"25"</option>
                        <option value="50" selected>"50"</option>
                        <option value="100">"100"</option>
                    </select>
                </div>
                <div class="header__actions">
                    <button class="button button--icon-transparent" title="Обновить" on:click=move |_| load_data()>
                        "🔄"
                    </button>
                    <button class="button button--primary" on:click=move |_| show_create_form.set(true)>
                        "+ Новый"
                    </button>
                </div>
            </div>

            // Header row 2 (placeholder for filters)
            <div class="list-header-row sub-header">
                <span class="muted">"Фильтры не заданы"</span>
            </div>

            <Show when=move || error_message.get().is_some()>
                <div class="error-message">
                    {move || error_message.get().unwrap_or_default()}
                </div>
            </Show>

            {move || if show_create_form.get() {
                view! {
                    <super::details::CreateUserForm
                        on_close=move || show_create_form.set(false)
                        on_created=move || {
                            show_create_form.set(false);
                            load_data();
                        }
                    />
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }}

            {move || if editing_user.get().is_some() {
                editing_user
                    .get()
                    .map(|user| view! {
                        <EditUserForm
                            user=user
                            on_close=close_edit
                            on_saved=on_saved
                        />
                    })
                    .into_any()
            } else {
                view! { <></> }.into_any()
            }}

            <Show
                when=move || !loading.get()
                fallback=|| view! { <div class="loading">"Загрузка пользователей..."</div> }
            >
                <div class="table-container">
                    <table class="table__data" id=TABLE_ID>
                        <thead>
                            <tr>
                                <th class="resizable" on:click=toggle_sort("username")>
                                    <span class="table__sortable-header">
                                        "Логин"
                                        <span class=move || get_sort_class(
                                            state.get().sort_field.as_str(),
                                            "username",
                                        )>
                                            {move || get_sort_indicator(
                                                state.get().sort_field.as_str(),
                                                "username",
                                                state.get().sort_ascending,
                                            )}
                                        </span>
                                    </span>
                                </th>
                                <th class="resizable" on:click=toggle_sort("full_name")>
                                    <span class="table__sortable-header">
                                        "ФИО"
                                        <span class=move || get_sort_class(
                                            state.get().sort_field.as_str(),
                                            "full_name",
                                        )>
                                            {move || get_sort_indicator(
                                                state.get().sort_field.as_str(),
                                                "full_name",
                                                state.get().sort_ascending,
                                            )}
                                        </span>
                                    </span>
                                </th>
                                <th class="resizable" on:click=toggle_sort("email")>
                                    <span class="table__sortable-header">
                                        "Email"
                                        <span class=move || get_sort_class(
                                            state.get().sort_field.as_str(),
                                            "email",
                                        )>
                                            {move || get_sort_indicator(
                                                state.get().sort_field.as_str(),
                                                "email",
                                                state.get().sort_ascending,
                                            )}
                                        </span>
                                    </span>
                                </th>
                                <th class="resizable" on:click=toggle_sort("is_admin")>
                                    <span class="table__sortable-header">
                                        "Админ"
                                        <span class=move || get_sort_class(
                                            state.get().sort_field.as_str(),
                                            "is_admin",
                                        )>
                                            {move || get_sort_indicator(
                                                state.get().sort_field.as_str(),
                                                "is_admin",
                                                state.get().sort_ascending,
                                            )}
                                        </span>
                                    </span>
                                </th>
                                <th class="resizable" on:click=toggle_sort("is_active")>
                                    <span class="table__sortable-header">
                                        "Активен"
                                        <span class=move || get_sort_class(
                                            state.get().sort_field.as_str(),
                                            "is_active",
                                        )>
                                            {move || get_sort_indicator(
                                                state.get().sort_field.as_str(),
                                                "is_active",
                                                state.get().sort_ascending,
                                            )}
                                        </span>
                                    </span>
                                </th>
                                <th class="resizable" on:click=toggle_sort("created_at")>
                                    <span class="table__sortable-header">
                                        "Создан"
                                        <span class=move || get_sort_class(
                                            state.get().sort_field.as_str(),
                                            "created_at",
                                        )>
                                            {move || get_sort_indicator(
                                                state.get().sort_field.as_str(),
                                                "created_at",
                                                state.get().sort_ascending,
                                            )}
                                        </span>
                                    </span>
                                </th>
                                <th class="resizable" on:click=toggle_sort("last_login_at")>
                                    <span class="table__sortable-header">
                                        "Последний вход"
                                        <span class=move || get_sort_class(
                                            state.get().sort_field.as_str(),
                                            "last_login_at",
                                        )>
                                            {move || get_sort_indicator(
                                                state.get().sort_field.as_str(),
                                                "last_login_at",
                                                state.get().sort_ascending,
                                            )}
                                        </span>
                                    </span>
                                </th>
                                <th class="resizable text-center">"Действия"</th>
                            </tr>
                        </thead>
                        <tbody>
                            <For
                                each=move || page_items.get()
                                key=|user| user.id.clone()
                                let:user
                            >
                                {
                                    let user_for_edit = user.clone();
                                    view! {
                                        <tr>
                                            <td>{user.username.clone()}</td>
                                            <td>{user.full_name.clone().unwrap_or_default()}</td>
                                            <td>{user.email.clone().unwrap_or_default()}</td>
                                            <td>{if user.is_admin { "Да" } else { "Нет" }}</td>
                                            <td>{if user.is_active { "Активен" } else { "Блок" }}</td>
                                            <td>{format_ts(&user.created_at)}</td>
                                            <td>{format_ts_opt(&user.last_login_at)}</td>
                                            <td class="text-center">
                                                <button class="button button--smallall" on:click=move |_| open_edit(user_for_edit.clone())>
                                                    "✎"
                                                </button>
                                            </td>
                                        </tr>
                                    }
                                }
                            </For>
                        </tbody>
                    </table>
                </div>
            </Show>
        </div>
    }
}

#[component]
fn EditUserForm<F1, F2>(user: User, on_close: F1, on_saved: F2) -> impl IntoView
where
    F1: Fn() + 'static + Copy,
    F2: Fn() + 'static + Copy,
{
    let email = RwSignal::new(user.email.clone().unwrap_or_default());
    let full_name = RwSignal::new(user.full_name.clone().unwrap_or_default());
    let is_admin = RwSignal::new(user.is_admin);
    let is_active = RwSignal::new(user.is_active);
    let error_message = RwSignal::new(Option::<String>::None);
    let saving = RwSignal::new(false);

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        saving.set(true);
        error_message.set(None);

        let dto = UpdateUserDto {
            id: user.id.clone(),
            email: if email.get().trim().is_empty() {
                None
            } else {
                Some(email.get())
            },
            full_name: if full_name.get().trim().is_empty() {
                None
            } else {
                Some(full_name.get())
            },
            is_active: is_active.get(),
            is_admin: is_admin.get(),
        };

        spawn_local(async move {
            match api::update_user(dto).await {
                Ok(_) => {
                    on_saved();
                }
                Err(e) => {
                    error_message.set(Some(format!("Ошибка сохранения: {}", e)));
                    saving.set(false);
                }
            }
        });
    };

    view! {
        <div class="modal-overlay" on:click=move |_| on_close()>
            <div class="modal" on:click=move |ev| ev.stop_propagation()>
                <div class="modal-header">
                    <h3 class="modal-title">{format!("Редактирование: {}", user.username)}</h3>
                    <div class="modal-header-actions">
                        <button class="button button--ghost" on:click=move |_| on_close()>"×"</button>
                    </div>
                </div>

                <div class="modal-body">
                    <Show when=move || error_message.get().is_some()>
                        <div class="warning-box warning-box--error">
                            <span class="warning-box__icon">"⚠"</span>
                            <span class="warning-box__text">{move || error_message.get().unwrap_or_default()}</span>
                        </div>
                    </Show>

                    <form id="edit-user-form" on:submit=on_submit>
                    <div class="form__group">
                        <label class="form__label">"Email"</label>
                        <input
                            type="email"
                            class="form__input"
                            value=move || email.get()
                            on:input=move |ev| email.set(event_target_value(&ev))
                            disabled=move || saving.get()
                        />
                    </div>

                    <div class="form__group">
                        <label class="form__label">"ФИО"</label>
                        <input
                            type="text"
                            class="form__input"
                            value=move || full_name.get()
                            on:input=move |ev| full_name.set(event_target_value(&ev))
                            disabled=move || saving.get()
                        />
                    </div>

                    <div class="form__group">
                        <label class="form__checkbox-wrapper">
                            <input
                                type="checkbox"
                                class="form__checkbox"
                                checked=move || is_admin.get()
                                on:change=move |ev| is_admin.set(event_target_checked(&ev))
                                disabled=move || saving.get()
                            />
                            <span class="form__checkbox-label">"Администратор"</span>
                        </label>
                    </div>

                    <div class="form__group">
                        <label class="form__checkbox-wrapper">
                            <input
                                type="checkbox"
                                class="form__checkbox"
                                checked=move || is_active.get()
                                on:change=move |ev| is_active.set(event_target_checked(&ev))
                                disabled=move || saving.get()
                            />
                            <span class="form__checkbox-label">"Активен"</span>
                        </label>
                    </div>
                    </form>
                </div>

                <div class="form-actions">
                    <button
                        type="button"
                        class="button button--secondary"
                        on:click=move |_| on_close()
                        disabled=move || saving.get()
                    >
                        "Отмена"
                    </button>
                    <button
                        type="submit"
                        form="edit-user-form"
                        class="button button--primary"
                        disabled=move || saving.get()
                    >
                        {move || if saving.get() { "Сохранение..." } else { "Сохранить" }}
                    </button>
                </div>
            </div>
        </div>
    }
}
