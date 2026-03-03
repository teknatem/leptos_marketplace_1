use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator};
use leptos::prelude::*;
use serde::Deserialize;
use thaw::*;
use wasm_bindgen::JsCast;

use super::state::create_state;
use crate::shared::page_frame::PageFrame;

#[derive(Debug, Clone, Deserialize)]
struct BiDashboardRow {
    pub id: String,
    pub code: String,
    pub description: String,
    pub status: String,
    pub is_public: bool,
    pub owner_user_id: String,
    pub rating: Option<u8>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct PaginatedResponse {
    pub items: Vec<serde_json::Value>,
    pub total: u64,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

async fn fetch_paginated(
    limit: usize,
    offset: usize,
    sort_by: &str,
    sort_desc: bool,
    q: &str,
) -> Result<(Vec<BiDashboardRow>, u64, usize, usize, usize), String> {
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

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;

    let parsed: PaginatedResponse = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;

    let rows: Vec<BiDashboardRow> = parsed
        .items
        .into_iter()
        .filter_map(|v| {
            let id = v["id"].as_str()?.to_string();
            let code = v["code"].as_str().unwrap_or("").to_string();
            let description = v["description"].as_str().unwrap_or("").to_string();
            let status = v["status"].as_str().unwrap_or("draft").to_string();
            let is_public = v["is_public"].as_bool().unwrap_or(false);
            let owner_user_id = v["owner_user_id"].as_str().unwrap_or("").to_string();
            let rating = v["rating"].as_u64().map(|r| r as u8);
            let created_at = v["created_at"].as_str().map(|s| s[..10.min(s.len())].to_string());
            Some(BiDashboardRow {
                id,
                code,
                description,
                status,
                is_public,
                owner_user_id,
                rating,
                created_at,
            })
        })
        .collect();

    Ok((rows, parsed.total, parsed.page, parsed.page_size, parsed.total_pages))
}

async fn load_testdata() -> Result<(), String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a025-bi-dashboard/testdata", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;

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

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    Ok(())
}

fn status_badge(status: &str) -> AnyView {
    let (color, label) = match status {
        "active" => (BadgeColor::Success, "Активен"),
        "archived" => (BadgeColor::Warning, "Архив"),
        _ => (BadgeColor::Subtle, "Черновик"),
    };
    view! {
        <Badge color=color>{label}</Badge>
    }
    .into_any()
}

fn rating_stars(rating: Option<u8>) -> AnyView {
    let stars = rating.unwrap_or(0);
    view! {
        <span class="bi-rating" title=format!("{}/5", stars)>
            {(1u8..=5).map(|i| {
                let filled = i <= stars;
                view! {
                    <span class:bi-rating__star--filled=filled class="bi-rating__star">
                        {if filled { "★" } else { "☆" }}
                    </span>
                }
            }).collect::<Vec<_>>()}
        </span>
    }
    .into_any()
}

#[component]
pub fn BiDashboardList() -> impl IntoView {
    let ctx = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let state = create_state();

    let rows: RwSignal<Vec<BiDashboardRow>> = RwSignal::new(vec![]);
    let error: RwSignal<Option<String>> = RwSignal::new(None);
    let loading: RwSignal<bool> = RwSignal::new(false);

    let reload = {
        let state = state.clone();
        let rows = rows.clone();
        let error = error.clone();
        let loading = loading.clone();
        move || {
            let state = state.clone();
            let rows = rows.clone();
            let error = error.clone();
            let loading = loading.clone();
            leptos::task::spawn_local(async move {
                loading.set(true);
                error.set(None);
                let page = state.page.get_untracked();
                let page_size = state.page_size.get_untracked();
                let sort_by = state.sort_by.get_untracked();
                let sort_desc = state.sort_desc.get_untracked();
                let q = state.query.get_untracked();
                let offset = page * page_size;
                match fetch_paginated(page_size, offset, &sort_by, sort_desc, &q).await {
                    Ok((data, total, _page, _ps, total_pages)) => {
                        rows.set(data);
                        state.total_count.set(total);
                        state.total_pages.set(total_pages);
                        state.is_loaded.set(true);
                    }
                    Err(e) => error.set(Some(e)),
                }
                loading.set(false);
            });
        }
    };

    let reload_effect = reload.clone();
    Effect::new(move |_| {
        let _q = state.query.get();
        let _page = state.page.get();
        let _sort_by = state.sort_by.get();
        let _sort_desc = state.sort_desc.get();
        reload_effect();
    });

    let on_new = {
        let ctx = ctx.clone();
        move |_| {
            ctx.open_tab("a025_bi_dashboard_detail_new", "Новый BI Дашборд");
        }
    };

    let on_testdata = {
        let reload = reload.clone();
        let error = error.clone();
        move |_| {
            let reload = reload.clone();
            let error = error.clone();
            leptos::task::spawn_local(async move {
                if let Err(e) = load_testdata().await {
                    error.set(Some(e));
                } else {
                    reload();
                }
            });
        }
    };

    let make_sort_handler = {
        let state = state.clone();
        let reload = reload.clone();
        move |col: &'static str| {
            let state = state.clone();
            let reload = reload.clone();
            move |_: web_sys::MouseEvent| {
                let current = state.sort_by.get_untracked();
                if current == col {
                    state.sort_desc.update(|v| *v = !*v);
                } else {
                    state.sort_by.set(col.to_string());
                    state.sort_desc.set(true);
                }
                state.page.set(0);
                reload();
            }
        }
    };

    view! {
        <PageFrame page_id="a025_bi_dashboard--list" category="list">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"BI Дашборды"</h1>
                </div>
                <div class="page__header-right">
                    <Button appearance=ButtonAppearance::Primary on_click=on_new>
                        {icon("plus")} " Новый"
                    </Button>
                    <Button appearance=ButtonAppearance::Secondary on_click=on_testdata>
                        {icon("database")} " Тест. данные"
                    </Button>
                </div>
            </div>

            <div class="page__toolbar">
                <Input
                    value=state.query
                    placeholder="Поиск..."
                />
            </div>

            {move || error.get().map(|e| view! {
                <div class="warning-box">
                    <span class="warning-box__icon">"⚠"</span>
                    <span class="warning-box__text">{e}</span>
                </div>
            })}

            <div class="page__content">
                {move || if loading.get() {
                    view! { <div class="placeholder">"Загрузка..."</div> }.into_any()
                } else {
                    let rows_val = rows.get();
                    let sort_by_val = state.sort_by.get();
                    let sort_desc_val = state.sort_desc.get();
                    view! {
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th
                                        class=get_sort_class("code", &sort_by_val)
                                        on:click=make_sort_handler("code")
                                    >
                                        "Код" {get_sort_indicator("code", &sort_by_val, sort_desc_val)}
                                    </th>
                                    <th
                                        class=get_sort_class("description", &sort_by_val)
                                        on:click=make_sort_handler("description")
                                    >
                                        "Наименование" {get_sort_indicator("description", &sort_by_val, sort_desc_val)}
                                    </th>
                                    <th>"Статус"</th>
                                    <th>"Оценка"</th>
                                    <th>"Публичный"</th>
                                    <th>"Владелец"</th>
                                    <th
                                        class=get_sort_class("created_at", &sort_by_val)
                                        on:click=make_sort_handler("created_at")
                                    >
                                        "Создан" {get_sort_indicator("created_at", &sort_by_val, sort_desc_val)}
                                    </th>
                                    <th>"Действия"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {rows_val.into_iter().map(|row| {
                                    let row_id = row.id.clone();
                                    let row_id_delete = row.id.clone();
                                    let row_code = row.code.clone();
                                    let ctx_open = ctx.clone();
                                    let rows_reload = rows.clone();
                                    let error_del = error.clone();

                                    let on_open = {
                                        let tab_key = format!("a025_bi_dashboard_detail_{}", row_id);
                                        let tab_title = format!("Дашборд · {}", row_code);
                                        move |_: web_sys::MouseEvent| {
                                            ctx_open.open_tab(&tab_key, &tab_title);
                                        }
                                    };

                                    let on_view = {
                                        let ctx2 = ctx.clone();
                                        let tab_key = format!("a025_bi_dashboard_view_{}", row_id);
                                        let tab_title = format!("▶ {}", row.description.clone());
                                        move |_: web_sys::MouseEvent| {
                                            ctx2.open_tab(&tab_key, &tab_title);
                                        }
                                    };

                                    let on_delete = move |_: web_sys::MouseEvent| {
                                        let id = row_id_delete.clone();
                                        let rows2 = rows_reload.clone();
                                        let err2 = error_del.clone();
                                        leptos::task::spawn_local(async move {
                                            match delete_dashboard(&id).await {
                                                Ok(_) => rows2.update(|v| v.retain(|r| r.id != id)),
                                                Err(e) => err2.set(Some(e)),
                                            }
                                        });
                                    };

                                    view! {
                                        <tr>
                                            <td>
                                                <a
                                                    class="link"
                                                    href="#"
                                                    on:click=on_open
                                                >
                                                    {row.code.clone()}
                                                </a>
                                            </td>
                                            <td>{row.description.clone()}</td>
                                            <td>{status_badge(&row.status)}</td>
                                            <td>{rating_stars(row.rating)}</td>
                                            <td>
                                                {if row.is_public {
                                                    view! { <Badge color=BadgeColor::Success>"✓"</Badge> }.into_any()
                                                } else {
                                                    view! { <span class="text-muted">"—"</span> }.into_any()
                                                }}
                                            </td>
                                            <td class="text-muted">{row.owner_user_id.clone()}</td>
                                            <td class="text-muted">{row.created_at.clone().unwrap_or_default()}</td>
                                            <td class="actions">
                                                <Button
                                                    size=ButtonSize::Small
                                                    appearance=ButtonAppearance::Secondary
                                                    on_click=on_view
                                                >
                                                    {icon("monitor")}
                                                </Button>
                                                <Button
                                                    size=ButtonSize::Small
                                                    appearance=ButtonAppearance::Secondary
                                                    on_click=on_delete
                                                >
                                                    {icon("trash-2")}
                                                </Button>
                                            </td>
                                        </tr>
                                    }
                                }).collect::<Vec<_>>()}
                            </tbody>
                        </table>
                    }.into_any()
                }}
            </div>

            <div class="page__footer">
                <PaginationControls
                    current_page=Signal::derive(move || state.page.get())
                    total_pages=Signal::derive(move || state.total_pages.get())
                    total_count=Signal::derive(move || state.total_count.get() as usize)
                    page_size=Signal::derive(move || state.page_size.get())
                    on_page_change=Callback::new({
                        let state = state.clone();
                        let reload = reload.clone();
                        move |new_page: usize| {
                            state.page.set(new_page);
                            reload();
                        }
                    })
                    on_page_size_change=Callback::new({
                        let state = state.clone();
                        let reload = reload.clone();
                        move |new_size: usize| {
                            state.page_size.set(new_size);
                            state.page.set(0);
                            reload();
                        }
                    })
                />
            </div>
        </PageFrame>
    }
}
