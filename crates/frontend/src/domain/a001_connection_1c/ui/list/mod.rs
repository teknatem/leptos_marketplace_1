pub mod state;

use self::state::create_state;
use crate::domain::a001_connection_1c::ui::details::Connection1CDetails;
use crate::shared::components::table_checkbox::TableCheckbox;
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator, Sortable};
use crate::shared::table_utils::{clear_resize_flag, init_column_resize, was_just_resizing};
use contracts::domain::a001_connection_1c::aggregate::Connection1CDatabase;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

const COLUMN_WIDTHS_KEY: &str = "a001_connection_1c_column_widths";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse {
    pub items: Vec<Connection1CDatabase>,
    pub total: u64,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

/// Форматирует ISO 8601 дату в dd.mm.yyyy HH:MM
fn format_datetime(iso_date: &str) -> String {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(iso_date) {
        dt.format("%d.%m.%Y %H:%M").to_string()
    } else {
        iso_date.to_string()
    }
}

impl Sortable for Connection1CDatabase {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "description" => self
                .base
                .description
                .to_lowercase()
                .cmp(&other.base.description.to_lowercase()),
            "url" => self.url.to_lowercase().cmp(&other.url.to_lowercase()),
            "login" => self.login.to_lowercase().cmp(&other.login.to_lowercase()),
            "is_primary" => self.is_primary.cmp(&other.is_primary),
            "created_at" => self
                .base
                .metadata
                .created_at
                .cmp(&other.base.metadata.created_at),
            _ => Ordering::Equal,
        }
    }
}

#[component]
#[allow(non_snake_case)]
pub fn Connection1CList() -> impl IntoView {
    let state = create_state();
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (editing_id, set_editing_id) = signal::<Option<String>>(None);

    let load_connections = move || {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let page = state.with(|s| s.page);
            let page_size = state.with(|s| s.page_size);
            let sort_field = state.with(|s| s.sort_field.clone());
            let sort_ascending = state.with(|s| s.sort_ascending);
            let offset = page * page_size;

            let url = format!(
                "http://localhost:3000/api/connection_1c/list?limit={}&offset={}&sort_by={}&sort_desc={}",
                page_size, offset, sort_field, !sort_ascending
            );

            match gloo_net::http::Request::get(&url).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status == 200 {
                        match response.text().await {
                            Ok(text) => match serde_json::from_str::<PaginatedResponse>(&text) {
                                Ok(paginated) => {
                                    state.update(|s| {
                                        s.items = paginated.items;
                                        s.total_count = paginated.total as usize;
                                        s.total_pages = paginated.total_pages;
                                        s.is_loaded = true;
                                    });
                                    set_loading.set(false);
                                }
                                Err(e) => {
                                    set_error.set(Some(format!("Failed to parse response: {}", e)));
                                    set_loading.set(false);
                                }
                            },
                            Err(e) => {
                                set_error.set(Some(format!("Failed to read response: {}", e)));
                                set_loading.set(false);
                            }
                        }
                    } else {
                        set_error.set(Some(format!("Server error: {}", status)));
                        set_loading.set(false);
                    }
                }
                Err(e) => {
                    set_error.set(Some(format!("Failed to fetch: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    // Загрузка при монтировании
    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            load_connections();
        }
    });

    // Функция для изменения сортировки
    let toggle_sort = move |field: &'static str| {
        if was_just_resizing() {
            clear_resize_flag();
            return;
        }

        state.update(|s| {
            if s.sort_field == field {
                s.sort_ascending = !s.sort_ascending;
            } else {
                s.sort_field = field.to_string();
                s.sort_ascending = true;
            }
            s.page = 0; // Сброс на первую страницу
        });
        load_connections();
    };

    // Переключение выбора одного элемента
    let toggle_selection = move |id: String| {
        state.update(|s| {
            if s.selected_ids.contains(&id) {
                s.selected_ids.retain(|x| x != &id);
            } else {
                s.selected_ids.push(id);
            }
        });
    };

    // Выбрать все / снять все
    let toggle_all = move |_| {
        let items = state.with(|s| s.items.clone());
        let all_ids: Vec<String> = items
            .iter()
            .map(|item| {
                use contracts::domain::common::AggregateId;
                item.base.id.as_string()
            })
            .collect();
        state.update(|s| {
            if s.selected_ids.len() == all_ids.len() && !all_ids.is_empty() {
                s.selected_ids.clear();
            } else {
                s.selected_ids = all_ids;
            }
        });
    };

    // Проверка, выбраны ли все
    let all_selected = move || {
        let items = state.with(|s| s.items.clone());
        let selected_len = state.with(|s| s.selected_ids.len());
        !items.is_empty() && selected_len == items.len()
    };

    // Проверка, выбран ли элемент
    let is_selected = move |id: &str| state.with(|s| s.selected_ids.contains(&id.to_string()));

    let handle_create_new = move |_| {
        set_editing_id.set(Some(String::new())); // Пустая строка = создание
    };

    let handle_edit = move |id: String| {
        set_editing_id.set(Some(id));
    };

    let handle_saved = move |_| {
        set_editing_id.set(None); // Закрываем модалку
        load_connections(); // Обновляем список
    };

    let handle_close = move |_| {
        set_editing_id.set(None); // Просто закрываем
    };

    view! {
        <div class="page">
            // Page header with title and action buttons
            <div class="header">
                <div class="header__content">
                    <h1 class="header__title">{"1C Подключения"}</h1>
                </div>
                <div class="header__actions">
                    <button class="button button--primary" on:click=handle_create_new>
                        {icon("plus")}
                        {"Новое"}
                    </button>
                    <button class="button button--secondary" on:click=move |_| load_connections() prop:disabled=move || loading.get()>
                        {icon("refresh")}
                        {"Обновить"}
                    </button>
                </div>
            </div>

            // Error message
            {move || error.get().map(|err| view! {
                <div class="warning-box" style="background: var(--color-error-50); border-color: var(--color-error-100);">
                    <span class="warning-box__icon" style="color: var(--color-error);">"⚠"</span>
                    <span class="warning-box__text" style="color: var(--color-error);">{err}</span>
                </div>
            })}

            // Loading indicator or table
            {move || {
                if loading.get() {
                    view! {
                        <div class="loading-spinner" style="text-align: center; padding: 40px;">"Загрузка подключений..."</div>
                    }.into_any()
                } else {
                    let items = state.with(|s| s.items.clone());
                    let current_sort_field = state.with(|s| s.sort_field.clone());
                    let current_sort_asc = state.with(|s| s.sort_ascending);

                    // Initialize column resize
                    spawn_local(async {
                        gloo_timers::future::TimeoutFuture::new(50).await;
                        init_column_resize("connection-1c-table", COLUMN_WIDTHS_KEY);
                    });

                    view! {
                        <div class="table">
                            <table id="connection-1c-table" class="table__data table--striped" style="min-width: 1200px; table-layout: fixed;">
                                <thead class="table__head">
                                    <tr>
                                        <th class="table__header-cell table__header-cell--checkbox">
                                            <input
                                                type="checkbox"
                                                class="table__checkbox"
                                                on:change=toggle_all
                                                prop:checked=move || all_selected()
                                            />
                                        </th>
                                        <th class="table__header-cell resizable" style="width: 250px; min-width: 120px;" on:click=move |_| toggle_sort("description")>
                                            <span class="table__sortable-header">"Наименование" <span class={get_sort_class("description", &current_sort_field)}>{get_sort_indicator("description", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th class="table__header-cell resizable" style="width: 300px; min-width: 150px;" on:click=move |_| toggle_sort("url")>
                                            <span class="table__sortable-header">"URL" <span class={get_sort_class("url", &current_sort_field)}>{get_sort_indicator("url", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th class="table__header-cell resizable" style="width: 120px; min-width: 80px;" on:click=move |_| toggle_sort("login")>
                                            <span class="table__sortable-header">"Логин" <span class={get_sort_class("login", &current_sort_field)}>{get_sort_indicator("login", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th class="table__header-cell resizable" style="width: 200px; min-width: 100px;">"Комментарий"</th>
                                        <th class="table__header-cell resizable text-center" style="width: 80px; min-width: 60px;" on:click=move |_| toggle_sort("is_primary")>
                                            <span class="table__sortable-header" style="justify-content: center;">"Основное" <span class={get_sort_class("is_primary", &current_sort_field)}>{get_sort_indicator("is_primary", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th class="table__header-cell resizable" style="width: 130px; min-width: 100px;" on:click=move |_| toggle_sort("created_at")>
                                            <span class="table__sortable-header">"Создано" <span class={get_sort_class("created_at", &current_sort_field)}>{get_sort_indicator("created_at", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {items.into_iter().map(|item| {
                                        use contracts::domain::common::AggregateId;
                                        let id = item.base.id.as_string();
                                        let description = item.base.description.clone();
                                        let url = item.url.clone();
                                        let login = item.login.clone();
                                        let comment = item.base.comment.clone().unwrap_or_else(|| "-".to_string());
                                        let is_primary = item.is_primary;
                                        let created_at = format_datetime(&item.base.metadata.created_at.to_rfc3339());

                                        let id_check = id.clone();
                                        let id_toggle = id.clone();
                                        let id_row = id.clone();

                                        view! {
                                            <tr class="table__row" on:click=move |_| handle_edit(id_row.clone())>
                                                <TableCheckbox
                                                    checked=Signal::derive(move || is_selected(&id_check))
                                                    on_change=Callback::new(move |_checked| toggle_selection(id_toggle.clone()))
                                                />
                                                <td class="table__cell cell-truncate">{description}</td>
                                                <td class="table__cell cell-truncate" style="color: #1565c0; font-size: 12px;">{url}</td>
                                                <td class="table__cell cell-truncate">{login}</td>
                                                <td class="table__cell cell-truncate">{comment}</td>
                                                <td class="table__cell text-center">{if is_primary { "✓" } else { "" }}</td>
                                                <td class="table__cell" style="font-size: 12px;">{created_at}</td>
                                            </tr>
                                        }
                                    }).collect::<Vec<_>>()}
                                </tbody>
                            </table>
                        </div>
                    }.into_any()
                }
            }}

            // Details modal (controlled by editing_id)
            <Connection1CDetails
                id=editing_id.into()
                on_saved=Callback::new(handle_saved)
                on_close=Callback::new(handle_close)
            />
        </div>
    }
}
