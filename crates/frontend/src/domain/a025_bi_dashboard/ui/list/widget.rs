use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::table::{
    TableCellCheckbox, TableCrosshairHighlight, TableHeaderCheckbox,
};
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator};
use crate::shared::page_frame::PageFrame;
use crate::shared::table_utils::init_column_resize;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashSet;
use thaw::*;
use wasm_bindgen::JsCast;

use super::state::create_state;

// ── Row struct ────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct BiDashboardRow {
    pub id: String,
    pub code: String,
    pub description: String,
    pub status: String,
    pub is_public: bool,
    pub rating: Option<u8>,
    pub created_at: String,
}

// ── API helpers ───────────────────────────────────────────────────────────────

async fn fetch_paginated(
    limit: usize,
    offset: usize,
    sort_by: &str,
    sort_desc: bool,
    q: &str,
) -> Result<(Vec<BiDashboardRow>, u64, usize), String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let mut url = format!(
        "{}/api/a025-bi-dashboard/list?limit={}&offset={}&sort_by={}&sort_desc={}",
        api_base(),
        limit,
        offset,
        sort_by,
        sort_desc,
    );
    let q_trimmed = q.trim();
    if q_trimmed.len() >= 2 {
        let encoded = js_sys::encode_uri_component(q_trimmed)
            .as_string()
            .unwrap_or_default();
        url.push_str(&format!("&q={}", encoded));
    }

    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or("no window")?;
    let resp: Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?
        .dyn_into()
        .map_err(|e| format!("{e:?}"))?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let text: String =
        wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
            .await
            .map_err(|e| format!("{e:?}"))?
            .as_string()
            .ok_or("bad text")?;

    let parsed: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let total = parsed["total"].as_u64().unwrap_or(0);
    let total_pages = parsed["total_pages"].as_u64().unwrap_or(1) as usize;

    let rows: Vec<BiDashboardRow> = parsed["items"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| {
            Some(BiDashboardRow {
                id: v["id"].as_str()?.to_string(),
                code: v["code"].as_str().unwrap_or("").to_string(),
                description: v["description"].as_str().unwrap_or("").to_string(),
                status: v["status"].as_str().unwrap_or("draft").to_string(),
                is_public: v["is_public"].as_bool().unwrap_or(false),
                rating: v["rating"].as_u64().map(|r| r as u8),
                created_at: v["created_at"]
                    .as_str()
                    .map(|s| s[..10.min(s.len())].to_string())
                    .unwrap_or_default(),
            })
        })
        .collect();

    Ok((rows, total, total_pages))
}

async fn load_testdata() -> Result<(), String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a025-bi-dashboard/testdata", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp: Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?
        .dyn_into()
        .map_err(|e| format!("{e:?}"))?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    Ok(())
}

async fn delete_dashboard(id: &str) -> Result<(), String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("DELETE");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a025-bi-dashboard/{}", api_base(), id);
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or("no window")?;
    let resp: Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?
        .dyn_into()
        .map_err(|e| format!("{e:?}"))?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn status_badge(status: &str) -> AnyView {
    let (color, label) = match status {
        "active" => (BadgeColor::Success, "Активен"),
        "archived" => (BadgeColor::Warning, "Архив"),
        _ => (BadgeColor::Subtle, "Черновик"),
    };
    view! { <Badge color=color>{label}</Badge> }.into_any()
}

fn rating_stars(rating: Option<u8>) -> AnyView {
    let n = rating.unwrap_or(0);
    view! {
        <span class="bi-rating">
            {(1u8..=5).map(|i| view! {
                <span class:bi-rating__star--filled=(i <= n) class="bi-rating__star">
                    {if i <= n { "★" } else { "☆" }}
                </span>
            }).collect::<Vec<_>>()}
        </span>
    }
    .into_any()
}

// ── Constants ─────────────────────────────────────────────────────────────────

const TABLE_ID: &str = "a025-bi-dashboard-table";
const COLUMN_WIDTHS_KEY: &str = "a025_bi_dashboard_column_widths";

// ── Component ─────────────────────────────────────────────────────────────────

#[component]
#[allow(non_snake_case)]
pub fn BiDashboardList() -> impl IntoView {
    let ctx = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let state = create_state(); // BiDashboardListState: Copy

    let (items, set_items) = signal::<Vec<BiDashboardRow>>(vec![]);
    let (error, set_error) = signal::<Option<String>>(None);
    let (testdata_loading, set_testdata_loading) = signal(false);

    // ── Reload ────────────────────────────────────────────────────────────────
    // All captured values are Copy, so reload itself is Copy.
    let reload = move || {
        wasm_bindgen_futures::spawn_local(async move {
            set_error.set(None);
            let offset = state.page.get_untracked() * state.page_size.get_untracked();
            match fetch_paginated(
                state.page_size.get_untracked(),
                offset,
                &state.sort_by.get_untracked(),
                state.sort_desc.get_untracked(),
                &state.query.get_untracked(),
            )
            .await
            {
                Ok((data, total, total_pages)) => {
                    state.total_count.set(total);
                    state.total_pages.set(total_pages);
                    state.is_loaded.set(true);
                    set_items.set(data);
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    // Reload reactively when search / sort / page change
    Effect::new(move |_| {
        let _ = state.query.get();
        let _ = state.page.get();
        let _ = state.sort_by.get();
        let _ = state.sort_desc.get();
        reload();
    });

    // ── Handlers ──────────────────────────────────────────────────────────────

    let on_load_testdata = move |_| {
        set_testdata_loading.set(true);
        set_error.set(None);
        wasm_bindgen_futures::spawn_local(async move {
            match load_testdata().await {
                Ok(_) => {
                    set_testdata_loading.set(false);
                    reload();
                }
                Err(e) => {
                    set_testdata_loading.set(false);
                    set_error.set(Some(format!("Ошибка загрузки тест. данных: {}", e)));
                }
            }
        });
    };

    let toggle_select = move |id: String, checked: bool| {
        state.selected.update(|s| {
            if checked {
                s.insert(id);
            } else {
                s.remove(&id);
            }
        });
    };

    let delete_selected = move || {
        let ids: Vec<String> = state.selected.get().into_iter().collect();
        if ids.is_empty() {
            return;
        }
        let confirmed = web_sys::window()
            .and_then(|w| {
                w.confirm_with_message(&format!(
                    "Удалить выбранные дашборды? Количество: {}",
                    ids.len()
                ))
                .ok()
            })
            .unwrap_or(false);
        if !confirmed {
            return;
        }
        wasm_bindgen_futures::spawn_local(async move {
            for id in ids {
                let _ = delete_dashboard(&id).await;
            }
            state.selected.set(HashSet::new());
            reload();
        });
    };

    // ── Column resize ─────────────────────────────────────────────────────────
    let resize_initialized = StoredValue::new(false);
    Effect::new(move |_| {
        if !resize_initialized.get_value() {
            resize_initialized.set_value(true);
            spawn_local(async move {
                gloo_timers::future::TimeoutFuture::new(100).await;
                init_column_resize(TABLE_ID, COLUMN_WIDTHS_KEY);
            });
        }
    });

    view! {
        <PageFrame page_id="a025_bi_dashboard--list" category="list">
            // ── Header ────────────────────────────────────────────────────────
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"BI Дашборды"</h1>
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=on_load_testdata
                        disabled=Signal::derive(move || testdata_loading.get())
                    >
                        {icon("database")}
                        {move || if testdata_loading.get() { " Загрузка..." } else { " Тест. данные" }}
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=move |_| ctx.open_tab("a025_bi_dashboard_details_new", "Новый BI Дашборд")
                    >
                        {icon("plus")} " Новый"
                    </Button>
                    <Button appearance=ButtonAppearance::Secondary on_click=move |_| reload()>
                        {icon("refresh")} " Обновить"
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| delete_selected()
                        disabled=Signal::derive(move || state.selected.get().is_empty())
                    >
                        {icon("delete")}
                        {move || format!(" Удалить ({})", state.selected.get().len())}
                    </Button>
                </div>
            </div>

            // ── Toolbar ───────────────────────────────────────────────────────
            <div class="page__toolbar">
                <Input value=state.query placeholder="Поиск…" />
            </div>

            // ── Error ─────────────────────────────────────────────────────────
            {move || error.get().map(|e| view! {
                <div class="warning-box warning-box--error">
                    <span class="warning-box__icon">"⚠"</span>
                    <span class="warning-box__text">{e}</span>
                </div>
            })}

            // ── Table ─────────────────────────────────────────────────────────
            <div class="page__content">
                <div class="table-wrapper">
                    <TableCrosshairHighlight table_id=TABLE_ID.to_string() />

                    <Table attr:id=TABLE_ID attr:style="width: 100%; min-width: 760px;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCheckbox
                                    items=items
                                    selected=state.selected
                                    get_id=Callback::new(|row: BiDashboardRow| row.id.clone())
                                    on_change=Callback::new(move |check_all: bool| {
                                        if check_all {
                                            state.selected.update(|s| {
                                                for item in items.get() { s.insert(item.id.clone()); }
                                            });
                                        } else {
                                            state.selected.set(HashSet::new());
                                        }
                                    })
                                />
                                <TableHeaderCell resizable=true min_width=90.0 class="resizable">
                                    "Код"
                                    <span
                                        class=move || get_sort_class(&state.sort_by.get(), "code")
                                        style="cursor:pointer;margin-left:4px"
                                        on:click=move |e| {
                                            e.stop_propagation();
                                            if state.sort_by.get_untracked() == "code" {
                                                state.sort_desc.update(|v| *v = !*v);
                                            } else {
                                                state.sort_by.set("code".to_string());
                                                state.sort_desc.set(true);
                                            }
                                            state.page.set(0);
                                        }
                                    >
                                        {move || get_sort_indicator("code", &state.sort_by.get(), state.sort_desc.get())}
                                    </span>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=200.0 class="resizable">
                                    "Наименование"
                                    <span
                                        class=move || get_sort_class(&state.sort_by.get(), "description")
                                        style="cursor:pointer;margin-left:4px"
                                        on:click=move |e| {
                                            e.stop_propagation();
                                            if state.sort_by.get_untracked() == "description" {
                                                state.sort_desc.update(|v| *v = !*v);
                                            } else {
                                                state.sort_by.set("description".to_string());
                                                state.sort_desc.set(true);
                                            }
                                            state.page.set(0);
                                        }
                                    >
                                        {move || get_sort_indicator("description", &state.sort_by.get(), state.sort_desc.get())}
                                    </span>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=100.0>"Статус"</TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=100.0>"Оценка"</TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=80.0>"Публичный"</TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=110.0>
                                    "Создан"
                                    <span
                                        class=move || get_sort_class(&state.sort_by.get(), "created_at")
                                        style="cursor:pointer;margin-left:4px"
                                        on:click=move |e| {
                                            e.stop_propagation();
                                            if state.sort_by.get_untracked() == "created_at" {
                                                state.sort_desc.update(|v| *v = !*v);
                                            } else {
                                                state.sort_by.set("created_at".to_string());
                                                state.sort_desc.set(true);
                                            }
                                            state.page.set(0);
                                        }
                                    >
                                        {move || get_sort_indicator("created_at", &state.sort_by.get(), state.sort_desc.get())}
                                    </span>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=90.0>"Edit"</TableHeaderCell>
                            </TableRow>
                        </TableHeader>

                        <TableBody>
                            {move || items.get().into_iter().map(|row| {
                                let id      = row.id.clone();
                                let id_sel  = id.clone();
                                let is_sel  = state.selected.get().contains(&id);

                                let tab_key   = format!("a025_bi_dashboard_details_{}", id);
                                let tab_title = format!("Дашборд · {}", row.code);
                                let tab_key2  = tab_key.clone();
                                let tab_title2 = tab_title.clone();
                                let ctx_view  = ctx.clone();
                                let view_tab_key = format!("a025_bi_dashboard_view_{}", row.id);
                                let view_tab_title = format!("View · {}", row.code);
                                let view_tab_key2 = format!("a025_bi_dashboard_view_{}", row.id);
                                let view_tab_title2 = format!("View · {}", row.code);
                                let ctx_desc  = ctx.clone();

                                view! {
                                    <TableRow class:table__row--selected=is_sel>
                                        <TableCellCheckbox
                                            item_id=id_sel
                                            selected=state.selected
                                            on_change=Callback::new(move |(id, checked)| {
                                                toggle_select(id, checked);
                                            })
                                        />
                                        <TableCell>
                                            <TableCellLayout>
                                                <a
                                                    href="#"
                                                    class="table__link"
                                                    on:click=move |e| {
                                                        e.prevent_default();
                                                        ctx_desc.open_tab(&view_tab_key, &view_tab_title);
                                                    }
                                                >
                                                    {row.code.clone()}
                                                </a>
                                            </TableCellLayout>
                                        </TableCell>
                                        <TableCell>
                                            <TableCellLayout>
                                                <a
                                                    href="#"
                                                    class="table__link"
                                                    on:click=move |e| {
                                                        e.prevent_default();
                                                        ctx_desc.open_tab(&view_tab_key2, &view_tab_title2);
                                                    }
                                                >
                                                    {row.description.clone()}
                                                </a>
                                            </TableCellLayout>
                                        </TableCell>
                                        <TableCell>
                                            <TableCellLayout>{status_badge(&row.status)}</TableCellLayout>
                                        </TableCell>
                                        <TableCell>
                                            <TableCellLayout>{rating_stars(row.rating)}</TableCellLayout>
                                        </TableCell>
                                        <TableCell>
                                            <TableCellLayout>
                                                {if row.is_public {
                                                    view! { <Badge color=BadgeColor::Success>"✓"</Badge> }.into_any()
                                                } else {
                                                    view! { <span class="text-muted">"—"</span> }.into_any()
                                                }}
                                            </TableCellLayout>
                                        </TableCell>
                                        <TableCell>
                                            <TableCellLayout>
                                                <span class="text-muted">{row.created_at}</span>
                                            </TableCellLayout>
                                        </TableCell>
                                        <TableCell>
                                            <TableCellLayout>
                                                <a
                                                    href="#"
                                                    class="table__link"
                                                    on:click=move |e| {
                                                        e.prevent_default();
                                                        ctx_view.open_tab(&tab_key2, &tab_title2);
                                                    }
                                                >
                                                    {icon("edit")} " Edit"
                                                </a>
                                            </TableCellLayout>
                                        </TableCell>
                                    </TableRow>
                                }
                            }).collect_view()}
                        </TableBody>
                    </Table>
                </div>
            </div>

            // ── Pagination ────────────────────────────────────────────────────
            <div class="page__footer">
                <PaginationControls
                    current_page=Signal::derive(move || state.page.get())
                    total_pages=Signal::derive(move || state.total_pages.get())
                    total_count=Signal::derive(move || state.total_count.get() as usize)
                    page_size=Signal::derive(move || state.page_size.get())
                    on_page_change=Callback::new(move |new_page: usize| {
                        state.page.set(new_page);
                        reload();
                    })
                    on_page_size_change=Callback::new(move |new_size: usize| {
                        state.page_size.set(new_size);
                        state.page.set(0);
                        reload();
                    })
                />
            </div>
        </PageFrame>
    }
}
