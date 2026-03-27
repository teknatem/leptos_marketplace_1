mod state;

use contracts::system::users::User;
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::table::{
    TableCellCheckbox, TableCrosshairHighlight, TableHeaderCheckbox,
};
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

fn primary_role_label(code: &str) -> &'static str {
    match code {
        "admin" => "admin",
        "manager" => "manager",
        "operator" => "operator",
        "viewer" => "viewer",
        _ => "viewer",
    }
}

fn primary_role_badge_class(code: &str) -> &'static str {
    match code {
        "admin" => "badge badge--warning",
        "manager" => "badge badge--primary",
        "operator" => "badge badge--info",
        _ => "badge badge--neutral",
    }
}

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
            "primary_role_code" => self.primary_role_code.cmp(&other.primary_role_code),
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
    let selected: RwSignal<HashSet<String>> = RwSignal::new(HashSet::new());

    let global_ctx = use_context::<AppGlobalContext>();

    let open_user_details = move |user_id: String, username: String| {
        let tab_key = format!("sys_user_details_{}", user_id);
        if let Some(ctx) = global_ctx {
            ctx.open_tab(&tab_key, &username);
        }
    };

    let open_new_user = move || {
        if let Some(ctx) = global_ctx {
            ctx.open_tab("sys_user_new", "Новый пользователь");
        }
    };

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
                    state.update(|s| {
                        s.page = 0;
                        s.is_loaded = true;
                    });
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
        state.update(|s| {
            s.page = page;
        });
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
            if checked {
                s.insert(id);
            } else {
                s.remove(&id);
            }
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
        value
            .as_deref()
            .map(format_datetime)
            .unwrap_or_else(|| "-".to_string())
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
                        on_click=move |_| open_new_user()
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
                                <TableHeaderCell resizable=false class="resizable" min_width=100.0>
                                    <div class="table__sortable-header" style="cursor:pointer;" on:click=toggle_sort("primary_role_code")>
                                        "Основная роль"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "primary_role_code"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "primary_role_code", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false class="resizable" min_width=80.0>
                                    <div class="table__sortable-header" style="cursor:pointer;" on:click=toggle_sort("is_admin")>
                                        "is_admin"
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
                            </TableRow>
                        </TableHeader>

                        <TableBody>
                            <For
                                each=move || state.get().items
                                key=|u| format!("{}:{}", u.id, u.updated_at)
                                children=move |user| {
                                    let user_id = user.id.clone();
                                    let user_id_for_click = user.id.clone();
                                    let username_for_click = user.username.clone();
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
                                                    <span
                                                        class="table__link"
                                                        on:click=move |_| open_user_details(user_id_for_click.clone(), username_for_click.clone())
                                                    >
                                                        {user.username.clone()}
                                                    </span>
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
                                                    <span class=primary_role_badge_class(&user.primary_role_code)>
                                                        {primary_role_label(&user.primary_role_code)}
                                                    </span>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    {if user.is_admin {
                                                        view! { <span class="badge badge--error">"superadmin"</span> }.into_any()
                                                    } else {
                                                        view! { <span class="badge badge--neutral">"-"</span> }.into_any()
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
                                        </TableRow>
                                    }
                                }
                            />
                        </TableBody>
                    </Table>
                </div>

            </div>
        </PageFrame>
    }
}
