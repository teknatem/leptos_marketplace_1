mod state;

use contracts::system::users::{UpdateUserDto, User};
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::table::{TableCrosshairHighlight, TableHeaderCheckbox, TableCellCheckbox};
use crate::shared::date_utils::format_datetime;
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator, sort_list, Sortable};
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_SYSTEM;
use crate::shared::table_utils::{init_column_resize, was_just_resizing};
use crate::system::auth::guard::RequireAdmin;
use crate::system::users::api;
use state::{create_state, UsersListState};
use std::collections::HashSet;

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
    let all_users: RwSignal<Vec<User>> = RwSignal::new(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);
    let (loading, set_loading) = signal(false);
    let (show_create_form, set_show_create_form) = signal(false);
    let editing_user: RwSignal<Option<User>> = RwSignal::new(None);
    let selected: RwSignal<HashSet<String>> = RwSignal::new(HashSet::new());

    let refresh_view = move || {
        let query = state.with_untracked(|s| s.search_query.to_lowercase());
        let mut data = all_users.get_untracked();
        if !query.is_empty() {
            data.retain(|u| {
                u.username.to_lowercase().contains(&query)
                    || u.full_name
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&query)
                    || u.email
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&query)
            });
        }
        state.update(|s| {
            sort_list(&mut data, &s.sort_field, s.sort_ascending);
            s.total_count = data.len();
            recalc_pagination(s);
            let start = s.page * s.page_size;
            let end = (start + s.page_size).min(data.len());
            s.items = data.get(start..end).unwrap_or(&[]).to_vec();
        });
    };

    let load_data = move || {
        set_loading.set(true);
        set_error.set(None);
        spawn_local(async move {
            match api::fetch_users().await {
                Ok(data) => {
                    all_users.set(data);
                    state.update(|s| { s.page = 0; s.is_loaded = true; });
                    refresh_view();
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(format!("Не удалось загрузить пользователей: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            load_data();
        }
    });

    Effect::new(move |_| {
        if state.with(|s| s.is_loaded) {
            spawn_local(async move {
                TimeoutFuture::new(50).await;
                init_column_resize(TABLE_ID, COLUMN_WIDTHS_KEY);
            });
        }
    });

    let search_signal = RwSignal::new(String::new());

    let apply_search = move || {
        state.update(|s| {
            s.search_query = search_signal.get_untracked();
            s.page = 0;
        });
        refresh_view();
    };

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
            });
            refresh_view();
        }
    };

    let go_to_page = move |page: usize| {
        state.update(|s| { s.page = page; });
        refresh_view();
    };

    let change_page_size = move |size: usize| {
        state.update(|s| {
            s.page_size = size;
            s.page = 0;
        });
        refresh_view();
    };

    let items_signal = Signal::derive(move || state.get().items.clone());
    let selected_signal = Signal::derive(move || selected.get());

    let toggle_selection = move |id: String, checked: bool| {
        selected.update(|s| {
            if checked { s.insert(id); } else { s.remove(&id); }
        });
    };

    let toggle_all = move |check_all: bool| {
        if check_all {
            let all_ids = items_signal.get().iter().map(|u| u.id.clone()).collect();
            selected.set(all_ids);
        } else {
            selected.set(HashSet::new());
        }
    };

    let format_ts = |value: &str| format_datetime(value);
    let format_ts_opt = |value: &Option<String>| {
        value.as_deref().map(format_datetime).unwrap_or_else(|| "-".to_string())
    };

    view! {
        <PageFrame page_id="sys_users--list" category=PAGE_CAT_SYSTEM>
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Пользователи"</h1>
                    <Badge>
                        {move || state.get().total_count.to_string()}
                    </Badge>
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=move |_| set_show_create_form.set(true)
                    >
                        {icon("plus")}
                        " Новый"
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| load_data()
                        disabled=Signal::derive(move || loading.get())
                    >
                        {icon("refresh")}
                        {move || if loading.get() { " Загрузка..." } else { " Обновить" }}
                    </Button>
                </div>
            </div>

            <div class="page__content">
                {move || error.get().map(|e| view! { <div class="alert alert--error">{e}</div> })}

                <div class="filter-panel">
                    <div class="filter-panel-header">
                        <div class="filter-panel-header__left">
                            {icon("filter")}
                            <span class="filter-panel__title">"Поиск"</span>
                        </div>
                        <div class="filter-panel-header__center">
                            <PaginationControls
                                current_page=Signal::derive(move || state.get().page)
                                total_pages=Signal::derive(move || state.get().total_pages)
                                total_count=Signal::derive(move || state.get().total_count)
                                page_size=Signal::derive(move || state.get().page_size)
                                on_page_change=Callback::new(go_to_page)
                                on_page_size_change=Callback::new(change_page_size)
                                page_size_options=vec![25, 50, 100]
                            />
                        </div>
                        <div class="filter-panel-header__right">
                        </div>
                    </div>

                    <div class="filter-panel-content">
                        <Flex gap=FlexGap::Small align=FlexAlign::End>
                            <div style="flex: 1; max-width: 320px;">
                                <Input
                                    value=search_signal
                                    placeholder="Логин, ФИО или Email..."
                                />
                            </div>
                            <Button
                                appearance=ButtonAppearance::Primary
                                on_click=move |_| apply_search()
                                disabled=Signal::derive(move || loading.get())
                            >
                                "Найти"
                            </Button>
                            <Button
                                appearance=ButtonAppearance::Secondary
                                on_click=move |_| {
                                    search_signal.set(String::new());
                                    state.update(|s| { s.search_query = String::new(); s.page = 0; });
                                    refresh_view();
                                }
                            >
                                "Сбросить"
                            </Button>
                        </Flex>
                    </div>
                </div>

                <div class="table-wrapper">
                    <TableCrosshairHighlight table_id=TABLE_ID.to_string() />

                    <Table attr:id=TABLE_ID attr:style="width: 100%;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCheckbox
                                    items=items_signal
                                    selected=selected_signal
                                    get_id=Callback::new(|u: User| u.id.clone())
                                    on_change=Callback::new(toggle_all)
                                />
                                <TableHeaderCell resizable=false class="resizable" min_width=140.0>
                                    <div class="table__sortable-header" style="cursor:pointer;" on:click=toggle_sort("username")>
                                        "Логин"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "username"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "username", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false class="resizable" min_width=160.0>
                                    <div class="table__sortable-header" style="cursor:pointer;" on:click=toggle_sort("full_name")>
                                        "ФИО"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "full_name"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "full_name", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false class="resizable" min_width=160.0>
                                    <div class="table__sortable-header" style="cursor:pointer;" on:click=toggle_sort("email")>
                                        "Email"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "email"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "email", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false class="resizable" min_width=80.0>
                                    <div class="table__sortable-header" style="cursor:pointer;" on:click=toggle_sort("is_admin")>
                                        "Роль"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "is_admin"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "is_admin", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false class="resizable" min_width=90.0>
                                    <div class="table__sortable-header" style="cursor:pointer;" on:click=toggle_sort("is_active")>
                                        "Статус"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "is_active"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "is_active", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false class="resizable" min_width=130.0>
                                    <div class="table__sortable-header" style="cursor:pointer;" on:click=toggle_sort("created_at")>
                                        "Создан"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "created_at"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "created_at", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false class="resizable" min_width=130.0>
                                    <div class="table__sortable-header" style="cursor:pointer;" on:click=toggle_sort("last_login_at")>
                                        "Последний вход"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "last_login_at"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "last_login_at", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=60.0>
                                </TableHeaderCell>
                            </TableRow>
                        </TableHeader>

                        <TableBody>
                            <For
                                each=move || state.get().items
                                key=|u| u.id.clone()
                                children=move |user| {
                                    let user_id = user.id.clone();
                                    let user_for_edit = user.clone();
                                    let created = format_ts(&user.created_at);
                                    let last_login = format_ts_opt(&user.last_login_at);
                                    view! {
                                        <TableRow>
                                            <TableCellCheckbox
                                                item_id=user_id.clone()
                                                selected=selected_signal
                                                on_change=Callback::new(move |(id, checked)| toggle_selection(id, checked))
                                            />
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    <span style="font-weight: 500;">{user.username.clone()}</span>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {user.full_name.clone().unwrap_or_default()}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {user.email.clone().unwrap_or_default()}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    {if user.is_admin {
                                                        view! { <span class="badge badge--warning">"Админ"</span> }.into_any()
                                                    } else {
                                                        view! { <span class="badge badge--neutral">"Пользователь"</span> }.into_any()
                                                    }}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    {if user.is_active {
                                                        view! { <span class="badge badge--success">"Активен"</span> }.into_any()
                                                    } else {
                                                        view! { <span class="badge badge--error">"Заблок."</span> }.into_any()
                                                    }}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>{created}</TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>{last_login}</TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <Button
                                                    appearance=ButtonAppearance::Subtle
                                    on_click=move |_| editing_user.set(Some(user_for_edit.clone()))
                                    attr:title="Редактировать"
                                                >
                                                    {icon("edit")}
                                                </Button>
                                            </TableCell>
                                        </TableRow>
                                    }
                                }
                            />
                        </TableBody>
                    </Table>
                </div>

                {move || if show_create_form.get() {
                    view! {
                        <super::details::CreateUserForm
                            on_close=move || set_show_create_form.set(false)
                            on_created=move || {
                                set_show_create_form.set(false);
                                load_data();
                            }
                        />
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }}

                {move || editing_user.get().map(|user| view! {
                    <EditUserForm
                        user=user
                        on_close=move || editing_user.set(None)
                        on_saved=move || { editing_user.set(None); load_data(); }
                    />
                })}
            </div>
        </PageFrame>
    }
}

#[component]
fn EditUserForm<F1, F2>(user: User, on_close: F1, on_saved: F2) -> impl IntoView
where
    F1: Fn() + 'static + Copy + Send + Sync,
    F2: Fn() + 'static + Copy + Send + Sync,
{
    let email = RwSignal::new(user.email.clone().unwrap_or_default());
    let full_name = RwSignal::new(user.full_name.clone().unwrap_or_default());
    let is_admin = RwSignal::new(user.is_admin);
    let is_active = RwSignal::new(user.is_active);
    let (error, set_error) = signal::<Option<String>>(None);
    let (saving, set_saving) = signal(false);

    let username_display = user.username.clone();

    let on_save = move |_| {
        set_saving.set(true);
        set_error.set(None);

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
                Ok(_) => on_saved(),
                Err(e) => {
                    set_error.set(Some(format!("Ошибка сохранения: {}", e)));
                    set_saving.set(false);
                }
            }
        });
    };

    view! {
        <div class="modal-overlay" on:click=move |_| on_close()>
            <div class="modal" on:click=move |ev| ev.stop_propagation()>
                <div class="modal-header">
                    <h2 class="modal-title">{format!("Редактирование: {}", username_display)}</h2>
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
                        <Label>"Email"</Label>
                        <Input
                            value=email
                            input_type=InputType::Email
                            disabled=Signal::derive(move || saving.get())
                        />
                    </div>

                    <div class="form__group">
                        <Label>"ФИО"</Label>
                        <Input
                            value=full_name
                            disabled=Signal::derive(move || saving.get())
                        />
                    </div>

                    <div class="form__group">
                        <Checkbox checked=is_admin label="Администратор" />
                    </div>

                    <div class="form__group">
                        <Checkbox checked=is_active label="Активен" />
                    </div>
                </div>

                <div class="modal-footer">
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| on_close()
                        disabled=Signal::derive(move || saving.get())
                    >
                        "Отмена"
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=on_save
                        disabled=Signal::derive(move || saving.get())
                    >
                        {move || if saving.get() { "Сохранение..." } else { "Сохранить" }}
                    </Button>
                </div>
            </div>
        </div>
    }
}
