mod state;

use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::table::{format_number_with_decimals, TableCellMoney};
use crate::shared::components::ui::badge::Badge as UiBadge;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator};
use contracts::projections::p908_wb_goods_prices::dto::{
    WbGoodsPriceDto, WbGoodsPriceListResponse,
};
use leptos::logging::log;
use leptos::prelude::*;
use state::{create_state, persist_state};
use thaw::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

async fn fetch_connections() -> Result<Vec<(String, String)>, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init("/api/connection_mp", &opts)
        .map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;

    let connections: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    let mut result = Vec::new();
    if let Some(items) = connections.as_array() {
        for item in items {
            if let (Some(id), Some(description)) = (
                item.get("id").and_then(|v| v.as_str()),
                item.get("description").and_then(|v| v.as_str()),
            ) {
                result.push((id.to_string(), description.to_string()));
            }
        }
    }
    Ok(result)
}

async fn fetch_goods_prices(
    limit: usize,
    offset: i32,
    connection_mp_ref: &str,
    vendor_code: &str,
    search: &str,
    sort_by: &str,
    sort_desc: bool,
) -> Result<WbGoodsPriceListResponse, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let mut params = format!(
        "limit={}&offset={}&sort_by={}&sort_desc={}",
        limit, offset, sort_by, sort_desc
    );
    if !connection_mp_ref.is_empty() {
        params += &format!(
            "&connection_mp_ref={}",
            urlencoding::encode(connection_mp_ref)
        );
    }
    if !vendor_code.is_empty() {
        params += &format!("&vendor_code={}", urlencoding::encode(vendor_code));
    }
    if !search.is_empty() {
        params += &format!("&search={}", urlencoding::encode(search));
    }

    let url = format!("/api/p908/goods-prices?{}", params);

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;

    serde_json::from_str::<WbGoodsPriceListResponse>(&text).map_err(|e| format!("{e}"))
}

fn fmt_percent(val: Option<f64>) -> String {
    match val {
        Some(v) => format_number_with_decimals(v, 1),
        None => "—".to_string(),
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}…", truncated)
    }
}

#[component]
pub fn WbGoodsPricesList() -> impl IntoView {
    let state = create_state();
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");

    let (items, set_items) = signal(Vec::<WbGoodsPriceDto>::new());
    let (is_loading, set_is_loading) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);
    let (connections, set_connections) = signal(Vec::<(String, String)>::new());
    let (is_filter_expanded, set_is_filter_expanded) = signal(true);

    // RwSignals bound to controls
    let connection_filter = RwSignal::new(state.get_untracked().connection_filter.clone());
    let vendor_code_filter = RwSignal::new(state.get_untracked().vendor_code_filter.clone());
    let search_filter = RwSignal::new(state.get_untracked().search_filter.clone());

    let active_filters_count = Signal::derive(move || {
        let s = state.get();
        let mut count = 0usize;
        if !s.connection_filter.is_empty() {
            count += 1;
        }
        if !s.vendor_code_filter.is_empty() {
            count += 1;
        }
        if !s.search_filter.is_empty() {
            count += 1;
        }
        count
    });

    // Load connections on mount
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            if let Ok(conns) = fetch_connections().await {
                set_connections.set(conns);
            }
        });
    });

    // Sync RwSignals → state
    Effect::new(move |_| {
        let v = connection_filter.get();
        state.update(|s| s.connection_filter = v);
        persist_state(state);
    });
    Effect::new(move |_| {
        let v = vendor_code_filter.get();
        state.update(|s| s.vendor_code_filter = v);
        persist_state(state);
    });
    Effect::new(move |_| {
        let v = search_filter.get();
        state.update(|s| s.search_filter = v);
        persist_state(state);
    });

    let load = move || {
        set_is_loading.set(true);
        set_error.set(None);

        let st = state.get_untracked();
        let limit = st.page_size;
        let offset = (st.page * st.page_size) as i32;
        let conn_val = st.connection_filter;
        let vc_val = st.vendor_code_filter;
        let search_val = st.search_filter;
        let sort_by_val = st.sort_by;
        let sort_desc = !st.sort_ascending;

        leptos::task::spawn_local(async move {
            match fetch_goods_prices(
                limit,
                offset,
                &conn_val,
                &vc_val,
                &search_val,
                &sort_by_val,
                sort_desc,
            )
            .await
            {
                Ok(response) => {
                    let total = response.total_count.max(0) as usize;
                    let total_pages = if limit == 0 {
                        0
                    } else {
                        (total + limit - 1) / limit
                    };
                    set_items.set(response.items);
                    state.update(|s| {
                        s.total_count = total;
                        s.total_pages = total_pages;
                        s.is_loaded = true;
                    });
                    persist_state(state);
                    set_is_loading.set(false);
                }
                Err(e) => {
                    log!("Failed to fetch WB goods prices: {:?}", e);
                    set_error.set(Some(e));
                    set_is_loading.set(false);
                }
            }
        });
    };

    // Initial load
    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            load();
        }
    });

    let go_to_page = move |page: usize| {
        state.update(|s| s.page = page);
        persist_state(state);
        load();
    };

    let change_page_size = move |size: usize| {
        state.update(|s| {
            s.page_size = size;
            s.page = 0;
        });
        persist_state(state);
        load();
    };

    let toggle_sort = move |field: &'static str| {
        state.update(|s| {
            if s.sort_by == field {
                s.sort_ascending = !s.sort_ascending;
            } else {
                s.sort_by = field.to_string();
                s.sort_ascending = true;
            }
            s.page = 0;
        });
        persist_state(state);
        load();
    };

    let open_nomenclature = move |id: String, name: String| {
        let title = if name.is_empty() {
            format!("Номенклатура {}", id.chars().take(8).collect::<String>())
        } else {
            format!("Ном. {}", truncate(&name, 20))
        };
        tabs_store.open_tab(&format!("a004_nomenclature_detail_{}", id), &title);
    };

    view! {
        <div class="page">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Цены товаров Wildberries"</h1>
                    <UiBadge variant="primary".to_string()>
                        {move || state.get().total_count.to_string()}
                    </UiBadge>
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=move |_| load()
                        disabled=Signal::derive(move || is_loading.get())
                    >
                        {move || if is_loading.get() { "Загрузка..." } else { "Обновить" }}
                    </Button>
                </div>
            </div>

            <div class="page__content">
                // Filter panel
                <div class="filter-panel">
                    <div class="filter-panel-header">
                        <div
                            class="filter-panel-header__left"
                            on:click=move |_| set_is_filter_expanded.update(|e| *e = !*e)
                        >
                            <svg
                                width="16"
                                height="16"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                class=move || {
                                    if is_filter_expanded.get() {
                                        "filter-panel__chevron filter-panel__chevron--expanded"
                                    } else {
                                        "filter-panel__chevron"
                                    }
                                }
                            >
                                <polyline points="6 9 12 15 18 9"></polyline>
                            </svg>
                            <span class="filter-panel__title">"Фильтры"</span>
                            {move || {
                                let count = active_filters_count.get();
                                if count > 0 {
                                    view! {
                                        <span class="filter-panel__badge">{count}</span>
                                    }.into_any()
                                } else {
                                    view! { <></> }.into_any()
                                }
                            }}
                        </div>

                        <div class="filter-panel-header__center">
                            <PaginationControls
                                current_page=Signal::derive(move || state.get().page)
                                total_pages=Signal::derive(move || state.get().total_pages)
                                total_count=Signal::derive(move || state.get().total_count)
                                page_size=Signal::derive(move || state.get().page_size)
                                on_page_change=Callback::new(go_to_page)
                                on_page_size_change=Callback::new(change_page_size)
                                page_size_options=vec![100, 500, 1000, 5000]
                            />
                        </div>

                        <div class="filter-panel-header__right"></div>
                    </div>

                    <Show when=move || is_filter_expanded.get()>
                        <div class="filter-panel-content">
                            <Flex gap=FlexGap::Small align=FlexAlign::End>
                                <div style="width: 220px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Подключение:"</Label>
                                        <Select value=connection_filter>
                                            <option value="">"— все —"</option>
                                            {move || connections.get().into_iter().map(|(id, name)| {
                                                view! { <option value={id}>{name}</option> }
                                            }).collect_view()}
                                        </Select>
                                    </Flex>
                                </div>

                                <div style="width: 200px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Артикул продавца:"</Label>
                                        <Input
                                            value=vendor_code_filter
                                            placeholder="Артикул..."
                                        />
                                    </Flex>
                                </div>

                                <div style="width: 200px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Поиск (nmId / артикул):"</Label>
                                        <Input
                                            value=search_filter
                                            placeholder="nmId или артикул..."
                                        />
                                    </Flex>
                                </div>

                                <div style="width: 120px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>" "</Label>
                                        <Button
                                            appearance=ButtonAppearance::Primary
                                            on_click=move |_| {
                                                state.update(|s| s.page = 0);
                                                persist_state(state);
                                                load();
                                            }
                                            disabled=Signal::derive(move || is_loading.get())
                                        >
                                            {move || if is_loading.get() { "Загрузка..." } else { "Применить" }}
                                        </Button>
                                    </Flex>
                                </div>
                            </Flex>
                        </div>
                    </Show>
                </div>

                // Error
                {move || {
                    error.get().map(|e| view! {
                        <div class="alert alert--error">
                            {format!("Ошибка: {}", e)}
                        </div>
                    })
                }}

                // Table
                <div class="table-wrapper">
                    <Table attr:style="width: 100%;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCell resizable=true min_width=120.0>
                                    "nmId"
                                    <span
                                        class=move || get_sort_class("nm_id", &state.get().sort_by)
                                        on:click=move |_| toggle_sort("nm_id")
                                        style="cursor: pointer;"
                                    >
                                        {move || get_sort_indicator("nm_id", &state.get().sort_by, state.get().sort_ascending)}
                                    </span>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=true min_width=180.0>
                                    "Кабинет"
                                    <span
                                        class=move || get_sort_class("connection_name", &state.get().sort_by)
                                        on:click=move |_| toggle_sort("connection_name")
                                        style="cursor: pointer;"
                                    >
                                        {move || get_sort_indicator("connection_name", &state.get().sort_by, state.get().sort_ascending)}
                                    </span>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=true min_width=180.0>
                                    "Артикул продавца"
                                    <span
                                        class=move || get_sort_class("vendor_code", &state.get().sort_by)
                                        on:click=move |_| toggle_sort("vendor_code")
                                        style="cursor: pointer;"
                                    >
                                        {move || get_sort_indicator("vendor_code", &state.get().sort_by, state.get().sort_ascending)}
                                    </span>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=true min_width=220.0>
                                    "Номенклатура"
                                    <span
                                        class=move || get_sort_class("nomenclature_name", &state.get().sort_by)
                                        on:click=move |_| toggle_sort("nomenclature_name")
                                        style="cursor: pointer;"
                                    >
                                        {move || get_sort_indicator("nomenclature_name", &state.get().sort_by, state.get().sort_ascending)}
                                    </span>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=true min_width=100.0>
                                    "Цена"
                                    <span
                                        class=move || get_sort_class("price", &state.get().sort_by)
                                        on:click=move |_| toggle_sort("price")
                                        style="cursor: pointer;"
                                    >
                                        {move || get_sort_indicator("price", &state.get().sort_by, state.get().sort_ascending)}
                                    </span>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=true min_width=100.0>
                                    "Цена со скидкой"
                                    <span
                                        class=move || get_sort_class("discounted_price", &state.get().sort_by)
                                        on:click=move |_| toggle_sort("discounted_price")
                                        style="cursor: pointer;"
                                    >
                                        {move || get_sort_indicator("discounted_price", &state.get().sort_by, state.get().sort_ascending)}
                                    </span>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=true min_width=80.0>
                                    "Скидка %"
                                    <span
                                        class=move || get_sort_class("discount", &state.get().sort_by)
                                        on:click=move |_| toggle_sort("discount")
                                        style="cursor: pointer;"
                                    >
                                        {move || get_sort_indicator("discount", &state.get().sort_by, state.get().sort_ascending)}
                                    </span>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=true min_width=110.0>
                                    "Цена дилера"
                                    <span
                                        class=move || get_sort_class("dealer_price_ut", &state.get().sort_by)
                                        on:click=move |_| toggle_sort("dealer_price_ut")
                                        style="cursor: pointer;"
                                    >
                                        {move || get_sort_indicator("dealer_price_ut", &state.get().sort_by, state.get().sort_ascending)}
                                    </span>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=true min_width=90.0>
                                    "Маржа %"
                                    <span
                                        class=move || get_sort_class("margin_pro", &state.get().sort_by)
                                        on:click=move |_| toggle_sort("margin_pro")
                                        style="cursor: pointer;"
                                    >
                                        {move || get_sort_indicator("margin_pro", &state.get().sort_by, state.get().sort_ascending)}
                                    </span>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=true min_width=160.0>
                                    "Дата загрузки"
                                    <span
                                        class=move || get_sort_class("fetched_at", &state.get().sort_by)
                                        on:click=move |_| toggle_sort("fetched_at")
                                        style="cursor: pointer;"
                                    >
                                        {move || get_sort_indicator("fetched_at", &state.get().sort_by, state.get().sort_ascending)}
                                    </span>
                                </TableHeaderCell>
                            </TableRow>
                        </TableHeader>

                        <TableBody>
                            {move || items.get().into_iter().map(|item| {
                                let nom_id = item.ext_nomenklature_ref.clone();
                                let nom_name = item.nomenclature_name.clone().unwrap_or_default();
                                let nom_name_display = truncate(&nom_name, 40);
                                let nom_name_for_cb = nom_name.clone();
                                let conn_name = item.connection_name.clone().unwrap_or_default();
                                let conn_name_display = truncate(&conn_name, 30);

                                view! {
                                    <TableRow>
                                        <TableCell>
                                            <span style="font-family: monospace; font-size: var(--font-size-sm);">
                                                {item.nm_id.to_string()}
                                            </span>
                                        </TableCell>
                                        <TableCell>
                                            <span
                                                title={conn_name}
                                                style="font-size: var(--font-size-sm);"
                                            >
                                                {conn_name_display}
                                            </span>
                                        </TableCell>
                                        <TableCell>
                                        <TableCellLayout truncate = true>
                                        {item.vendor_code.clone().unwrap_or_default()}
                                        </TableCellLayout>
                                        </TableCell>
                                        <TableCell>
                                            <TableCellLayout truncate = true>
                                            {move || {
                                                if let Some(ref id) = nom_id {
                                                    let id_clone = id.clone();
                                                    let name_clone = nom_name_for_cb.clone();
                                                    view! {
                                                        <a
                                                            href="#"
                                                            title={nom_name.clone()}
                                                            style="color: var(--color-link); text-decoration: none; cursor: pointer;"
                                                            on:click=move |ev| {
                                                                ev.prevent_default();
                                                                open_nomenclature(id_clone.clone(), name_clone.clone());
                                                            }
                                                        >
                                                            {nom_name_display.clone()}
                                                        </a>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <span style="color: var(--color-text-secondary); font-size: var(--font-size-sm);">
                                                            "—"
                                                        </span>
                                                    }.into_any()
                                                }
                                            }}
                                            </TableCellLayout>
                                        </TableCell>
                                        <TableCellMoney
                                            value=Signal::derive(move || item.price)
                                            show_currency=false
                                            color_by_sign=false
                                        />
                                        <TableCellMoney
                                            value=Signal::derive(move || item.discounted_price)
                                            show_currency=false
                                            color_by_sign=false
                                            bold=true
                                        />
                                        <TableCell class="text-right">
                                            <span>
                                                {match item.discount {
                                                    Some(d) => format_number_with_decimals(d as f64, 1),
                                                    None => "—".to_string(),
                                                }}
                                            </span>
                                        </TableCell>
                                        <TableCellMoney
                                            value=Signal::derive(move || item.dealer_price_ut)
                                            show_currency=false
                                            color_by_sign=false
                                        />
                                        <TableCell class="text-right">
                                            {
                                                let margin = item.margin_pro;
                                                let style = match margin {
                                                    Some(m) if m >= 0.0 => "color: var(--color-success-700); font-weight: 500;",
                                                    Some(_) => "color: var(--color-error-700); font-weight: 500;",
                                                    None => "",
                                                };
                                                view! {
                                                    <span style={style}>
                                                        {fmt_percent(margin)}
                                                    </span>
                                                }
                                            }
                                        </TableCell>
                                        <TableCell>
                                            <span style="font-size: var(--font-size-sm); color: var(--color-text-secondary);">
                                                {item.fetched_at.get(..16).unwrap_or(&item.fetched_at).to_string()}
                                            </span>
                                        </TableCell>
                                    </TableRow>
                                }
                            }).collect_view()}
                        </TableBody>
                    </Table>
                </div>
            </div>
        </div>
    }
}
