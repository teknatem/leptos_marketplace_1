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
struct BiIndicatorRow {
    pub id: String,
    pub code: String,
    pub description: String,
    pub status: String,
    pub is_public: bool,
    pub owner_user_id: String,
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
) -> Result<(Vec<BiIndicatorRow>, u64, usize, usize, usize), String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let mut url = format!(
        "{}/api/a024-bi-indicator/list?limit={}&offset={}&sort_by={}&sort_desc={}",
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

    let rows: Vec<BiIndicatorRow> = parsed
        .items
        .into_iter()
        .filter_map(|v| {
            let id = v["id"].as_str()?.to_string();
            let code = v["code"].as_str().unwrap_or("").to_string();
            let description = v["description"].as_str().unwrap_or("").to_string();
            let status = v["status"].as_str().unwrap_or("draft").to_string();
            let is_public = v["is_public"].as_bool().unwrap_or(false);
            let owner_user_id = v["owner_user_id"].as_str().unwrap_or("").to_string();
            let created_at = v["created_at"].as_str().map(|s| s[..10.min(s.len())].to_string());
            Some(BiIndicatorRow {
                id,
                code,
                description,
                status,
                is_public,
                owner_user_id,
                created_at,
            })
        })
        .collect();

    Ok((rows, parsed.total, parsed.page, parsed.page_size, parsed.total_pages))
}

async fn delete_indicator(id: &str) -> Result<(), String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("DELETE");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a024-bi-indicator/{}", api_base(), id);
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
        "archived" => (BadgeColor::Subtle, "Архив"),
        _ => (BadgeColor::Warning, "Черновик"),
    };
    view! {
        <Badge appearance=BadgeAppearance::Tint color=color>
            {label}
        </Badge>
    }
    .into_any()
}

#[component]
pub fn BiIndicatorList() -> impl IntoView {
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");
    let state = create_state();

    let (items, set_items) = signal(Vec::<BiIndicatorRow>::new());
    let (is_loading, set_is_loading) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);

    let q = RwSignal::new(state.get_untracked().q.clone());
    let sort_field = RwSignal::new(state.get_untracked().sort_field.clone());
    let sort_ascending = RwSignal::new(state.get_untracked().sort_ascending);

    let load = move || {
        set_is_loading.set(true);
        set_error.set(None);

        let s = state.get_untracked();
        let limit = s.page_size;
        let offset = s.page * s.page_size;
        let sort_desc = !s.sort_ascending;
        let q_val = s.q.clone();
        let sort_by = s.sort_field.clone();

        leptos::task::spawn_local(async move {
            match fetch_paginated(limit, offset, &sort_by, sort_desc, &q_val).await {
                Ok((rows, total, page, page_size, total_pages)) => {
                    set_items.set(rows);
                    state.update(|st| {
                        st.total_count = total as usize;
                        st.total_pages = total_pages;
                        st.page = page;
                        st.page_size = page_size;
                        st.is_loaded = true;
                    });
                    set_is_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_is_loading.set(false);
                }
            }
        });
    };

    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            load();
        }
    });

    let debounce_timeout = StoredValue::new(None::<i32>);
    let q_first_run = StoredValue::new(true);
    Effect::new(move |_| {
        let q_now = q.get();

        if q_first_run.get_value() {
            q_first_run.set_value(false);
            return;
        }

        if let Some(timeout_id) = debounce_timeout.get_value() {
            web_sys::window().and_then(|w| Some(w.clear_timeout_with_handle(timeout_id)));
        }

        if !(q_now.trim().is_empty() || q_now.trim().len() >= 2) {
            return;
        }

        let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
            state.update(|s| {
                s.q = q_now.clone();
                s.page = 0;
            });
            load();
        }) as Box<dyn Fn()>);

        let window = web_sys::window().expect("no window");
        let timeout_id = window
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref::<js_sys::Function>(),
                300,
            )
            .expect("setTimeout failed");

        closure.forget();
        debounce_timeout.set_value(Some(timeout_id));
    });

    let go_to_page = move |page: usize| {
        state.update(|s| s.page = page);
        load();
    };

    let change_page_size = move |size: usize| {
        state.update(|s| {
            s.page_size = size;
            s.page = 0;
        });
        load();
    };

    let open_details_tab = {
        let tabs_store = tabs_store;
        move |id: String, code: String, description: String| {
            use crate::layout::tabs::{detail_tab_label, pick_identifier};
            use contracts::domain::a024_bi_indicator::ENTITY_METADATA as A024;
            let identifier = pick_identifier(None, Some(&code), Some(&description), &id);
            let title = detail_tab_label(A024.ui.element_name, identifier);
            tabs_store.open_tab(&format!("a024_bi_indicator_detail_{}", id), &title);
        }
    };

    let open_new_tab = {
        let tabs_store = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
        move |_| {
            use contracts::domain::a024_bi_indicator::ENTITY_METADATA as A024;
            let title = format!("Новый {}", A024.ui.element_name);
            tabs_store.open_tab("a024_bi_indicator_detail_new", &title);
        }
    };

    view! {
        <PageFrame page_id="a024_bi_indicator--list" category="list" class="page--wide">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"BI Индикаторы"</h1>
                    <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                        {move || state.get().total_count.to_string()}
                    </Badge>
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| load()
                        disabled=move || is_loading.get()
                    >
                        {icon("refresh")}
                        {move || if is_loading.get() { " Загрузка..." } else { " Обновить" }}
                    </Button>
                    <Button appearance=ButtonAppearance::Primary on_click=open_new_tab>
                        {icon("plus")}
                        " Создать"
                    </Button>
                </div>
            </div>

            {move || error.get().map(|e| view! {
                <div class="warning-box warning-box--error">
                    <span class="warning-box__icon">"⚠"</span>
                    <span class="warning-box__text">{e}</span>
                </div>
            })}

            <div class="filter-panel">
                <div class="filter-panel-header">
                    <div class="filter-panel-header__left">
                        {icon("filter")}
                        <span class="filter-panel__title">"Фильтры"</span>
                    </div>
                    <div class="filter-panel-header__center">
                        <PaginationControls
                            current_page=Signal::derive(move || state.get().page)
                            total_pages=Signal::derive(move || state.get().total_pages)
                            total_count=Signal::derive(move || state.get().total_count)
                            page_size=Signal::derive(move || state.get().page_size)
                            on_page_change=Callback::new(go_to_page)
                            on_page_size_change=Callback::new(change_page_size)
                        />
                    </div>
                    <div class="filter-panel-header__right"></div>
                </div>

                <div class="filter-panel__collapsible filter-panel__collapsible--expanded">
                    <div class="filter-panel-content">
                        <Flex gap=FlexGap::Small align=FlexAlign::End>
                            <div style="min-width: 320px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Поиск:"</Label>
                                    <Input
                                        value=q
                                        placeholder="Код, наименование… (мин. 2 символа)"
                                    />
                                </Flex>
                            </div>
                        </Flex>
                    </div>
                </div>
            </div>

            <div class="page__content">
                <div style="width: 100%; overflow-x: auto;">
                    <Table attr:style="width: 100%;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCell resizable=true min_width=120.0>
                                    "Код"
                                    <span
                                        class={move || format!("table__header-sort-indicator {}", get_sort_class("code", &sort_field.get()))}
                                        on:click=move |e| {
                                            e.stop_propagation();
                                            state.update(|s| {
                                                if s.sort_field == "code" {
                                                    s.sort_ascending = !s.sort_ascending;
                                                } else {
                                                    s.sort_field = "code".to_string();
                                                    s.sort_ascending = true;
                                                }
                                                s.page = 0;
                                            });
                                            sort_field.set(state.get_untracked().sort_field.clone());
                                            sort_ascending.set(state.get_untracked().sort_ascending);
                                            load();
                                        }
                                    >
                                        {move || get_sort_indicator("code", &sort_field.get(), sort_ascending.get())}
                                    </span>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=true min_width=240.0>
                                    "Наименование"
                                    <span
                                        class={move || format!("table__header-sort-indicator {}", get_sort_class("description", &sort_field.get()))}
                                        on:click=move |e| {
                                            e.stop_propagation();
                                            state.update(|s| {
                                                if s.sort_field == "description" {
                                                    s.sort_ascending = !s.sort_ascending;
                                                } else {
                                                    s.sort_field = "description".to_string();
                                                    s.sort_ascending = true;
                                                }
                                                s.page = 0;
                                            });
                                            sort_field.set(state.get_untracked().sort_field.clone());
                                            sort_ascending.set(state.get_untracked().sort_ascending);
                                            load();
                                        }
                                    >
                                        {move || get_sort_indicator("description", &sort_field.get(), sort_ascending.get())}
                                    </span>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=true min_width=100.0>"Статус"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=80.0>"Публичный"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=140.0>"Владелец"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=100.0>"Создан"</TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=80.0>"Действия"</TableHeaderCell>
                            </TableRow>
                        </TableHeader>

                        <TableBody>
                            {move || {
                                let data = items.get();
                                if data.is_empty() && !is_loading.get() {
                                    return view! {
                                        <TableRow>
                                            <TableCell attr:colspan="7">
                                                <TableCellLayout>
                                                    <span class="table__cell--muted">"Нет данных"</span>
                                                </TableCellLayout>
                                            </TableCell>
                                        </TableRow>
                                    }.into_any();
                                }

                                data.into_iter().map(|row| {
                                    let id = row.id.clone();
                                    let id_for_delete = id.clone();
                                    let id_for_open = id.clone();
                                    let code_for_open = row.code.clone();
                                    let desc_for_open = row.description.clone();
                                    let code_display = row.code.clone();
                                    let desc_display = row.description.clone();
                                    let status_str = row.status.clone();
                                    let owner = row.owner_user_id.clone();
                                    let created_at = row.created_at.clone().unwrap_or_default();
                                    let is_public = row.is_public;

                                    view! {
                                        <TableRow>
                                            <TableCell>
                                                <TableCellLayout>
                                                    <a
                                                        href="#"
                                                        style="color: var(--colorBrandForeground1); text-decoration: none; cursor: pointer;"
                                                        on:click={
                                                            let id = id_for_open.clone();
                                                            let code = code_for_open.clone();
                                                            let desc = desc_for_open.clone();
                                                            move |e| {
                                                                e.prevent_default();
                                                                open_details_tab(id.clone(), code.clone(), desc.clone());
                                                            }
                                                        }
                                                    >
                                                        {code_display}
                                                    </a>
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {desc_display}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout>
                                                    {status_badge(&status_str)}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout>
                                                    {if is_public { "Да" } else { "Нет" }}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {owner}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout>
                                                    {created_at}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout>
                                                    <Button
                                                        appearance=ButtonAppearance::Subtle
                                                        size=ButtonSize::Small
                                                        on_click={
                                                            let id = id_for_delete.clone();
                                                            move |_| {
                                                                let id = id.clone();
                                                                leptos::task::spawn_local(async move {
                                                                    if let Err(e) = delete_indicator(&id).await {
                                                                        web_sys::window().and_then(|w| w.alert_with_message(&format!("Ошибка: {e}")).ok());
                                                                    } else {
                                                                        load();
                                                                    }
                                                                });
                                                            }
                                                        }
                                                    >
                                                        {icon("trash-2")}
                                                    </Button>
                                                </TableCellLayout>
                                            </TableCell>
                                        </TableRow>
                                    }
                                }).collect_view().into_any()
                            }}
                        </TableBody>
                    </Table>
                </div>
            </div>
        </PageFrame>
    }
}
