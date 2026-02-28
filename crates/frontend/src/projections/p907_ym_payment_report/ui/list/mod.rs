mod state;

use crate::layout::global_context::AppGlobalContext;
use crate::shared::page_frame::PageFrame;
use crate::shared::components::date_range_picker::DateRangePicker;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::table::TableCellMoney;
use crate::shared::components::ui::badge::Badge as UiBadge;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator};
use contracts::projections::p907_ym_payment_report::dto::{
    YmPaymentReportDto, YmPaymentReportListResponse,
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

#[allow(clippy::too_many_arguments)]
async fn fetch_payment_report(
    limit: usize,
    offset: i32,
    date_from: &str,
    date_to: &str,
    transaction_type: &str,
    payment_status: &str,
    shop_sku: &str,
    connection_mp_ref: &str,
    sort_by: &str,
    sort_desc: bool,
) -> Result<YmPaymentReportListResponse, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let mut params = format!(
        "date_from={}&date_to={}&limit={}&offset={}&sort_by={}&sort_desc={}",
        urlencoding::encode(date_from),
        urlencoding::encode(date_to),
        limit,
        offset,
        sort_by,
        sort_desc
    );
    if !transaction_type.is_empty() {
        params += &format!(
            "&transaction_type={}",
            urlencoding::encode(transaction_type)
        );
    }
    if !payment_status.is_empty() {
        params += &format!("&payment_status={}", urlencoding::encode(payment_status));
    }
    if !shop_sku.is_empty() {
        params += &format!("&shop_sku={}", urlencoding::encode(shop_sku));
    }
    if !connection_mp_ref.is_empty() {
        params += &format!(
            "&connection_mp_ref={}",
            urlencoding::encode(connection_mp_ref)
        );
    }

    let url = format!("/api/p907/payment-report?{}", params);

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

    serde_json::from_str::<YmPaymentReportListResponse>(&text).map_err(|e| format!("{e}"))
}

/// Format ISO date "YYYY-MM-DD HH:MM" → "DD.MM.YYYY HH:MM" for display
fn fmt_date(s: &str) -> String {
    let bytes = s.as_bytes();
    if bytes.len() >= 10 && bytes[4] == b'-' && bytes[7] == b'-' {
        let year = &s[0..4];
        let month = &s[5..7];
        let day = &s[8..10];
        let rest = &s[10..];
        return format!("{}.{}.{}{}", day, month, year, rest);
    }
    s.to_string()
}

#[component]
pub fn YmPaymentReportList() -> impl IntoView {
    let state = create_state();

    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");

    let (items, set_items) = signal(Vec::<YmPaymentReportDto>::new());
    let (is_loading, set_is_loading) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);
    let (connections, set_connections) = signal(Vec::<(String, String)>::new());
    let (is_filter_expanded, set_is_filter_expanded) = signal(true);

    // RwSignals bound to controls
    let transaction_type_filter =
        RwSignal::new(state.get_untracked().transaction_type_filter.clone());
    let payment_status_filter = RwSignal::new(state.get_untracked().payment_status_filter.clone());
    let shop_sku_filter = RwSignal::new(state.get_untracked().shop_sku_filter.clone());
    let connection_filter = RwSignal::new(state.get_untracked().connection_filter.clone());

    let active_filters_count = Signal::derive(move || {
        let s = state.get();
        let mut count = 0usize;
        if !s.date_from.is_empty() || !s.date_to.is_empty() {
            count += 1;
        }
        if !s.transaction_type_filter.is_empty() {
            count += 1;
        }
        if !s.payment_status_filter.is_empty() {
            count += 1;
        }
        if !s.shop_sku_filter.is_empty() {
            count += 1;
        }
        if !s.connection_filter.is_empty() {
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
        let v = transaction_type_filter.get();
        state.update(|s| s.transaction_type_filter = v);
        persist_state(state);
    });
    Effect::new(move |_| {
        let v = payment_status_filter.get();
        state.update(|s| s.payment_status_filter = v);
        persist_state(state);
    });
    Effect::new(move |_| {
        let v = shop_sku_filter.get();
        state.update(|s| s.shop_sku_filter = v);
        persist_state(state);
    });
    Effect::new(move |_| {
        let v = connection_filter.get();
        state.update(|s| s.connection_filter = v);
        persist_state(state);
    });

    let load = move || {
        set_is_loading.set(true);
        set_error.set(None);

        let st = state.get_untracked();
        let limit = st.page_size;
        let offset = (st.page * st.page_size) as i32;
        let date_from_val = st.date_from;
        let date_to_val = st.date_to;
        let tt_val = st.transaction_type_filter;
        let ps_val = st.payment_status_filter;
        let sku_val = st.shop_sku_filter;
        let conn_val = st.connection_filter;
        let sort_by_val = st.sort_by;
        let sort_desc = !st.sort_ascending;

        leptos::task::spawn_local(async move {
            match fetch_payment_report(
                limit,
                offset,
                &date_from_val,
                &date_to_val,
                &tt_val,
                &ps_val,
                &sku_val,
                &conn_val,
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
                    log!("Failed to fetch YM payment report: {:?}", e);
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

    let open_detail = move |record_key: String, date_str: String| {
        let tab_key = format!(
            "p907_ym_payment_report_detail_{}",
            js_sys::encode_uri_component(&record_key)
                .as_string()
                .unwrap_or_else(|| record_key.clone())
        );
        let tab_title = format!("ЯМ Платёж {}", date_str);
        tabs_store.open_tab(&tab_key, &tab_title);
    };

    view! {
        <PageFrame page_id="p907_ym_payment_report--list" category="list">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Отчёт по платежам Яндекс Маркет"</h1>
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
                                page_size_options=vec![100, 500, 1000]
                            />
                        </div>

                        <div class="filter-panel-header__right"></div>
                    </div>

                    <Show when=move || is_filter_expanded.get()>
                        <div class="filter-panel-content">
                            <Flex gap=FlexGap::Small align=FlexAlign::End>
                                <div style="min-width: 420px;">
                                    <DateRangePicker
                                        date_from=Signal::derive(move || state.with(|s| s.date_from.clone()))
                                        date_to=Signal::derive(move || state.with(|s| s.date_to.clone()))
                                        on_change=Callback::new(move |(from, to)| {
                                            state.update(|s| {
                                                s.date_from = from;
                                                s.date_to = to;
                                                s.page = 0;
                                            });
                                            persist_state(state);
                                            load();
                                        })
                                        label="Период (дата транзакции):".to_string()
                                    />
                                </div>

                                <div style="width: 220px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Тип транзакции:"</Label>
                                        <Input
                                            value=transaction_type_filter
                                            placeholder="Тип транзакции..."
                                        />
                                    </Flex>
                                </div>

                                <div style="width: 220px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Статус платежа:"</Label>
                                        <Input
                                            value=payment_status_filter
                                            placeholder="Статус..."
                                        />
                                    </Flex>
                                </div>

                                <div style="width: 180px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"SKU:"</Label>
                                        <Input
                                            value=shop_sku_filter
                                            placeholder="Артикул SKU..."
                                        />
                                    </Flex>
                                </div>

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
                    <Table attr:style="width: 100%; min-width: 1200px;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCell resizable=true min_width=120.0>
                                    "Дата"
                                    <span class=move || get_sort_class("transaction_date", &state.get().sort_by)
                                        on:click=move |_| toggle_sort("transaction_date")
                                        style="cursor: pointer;"
                                    >
                                        {move || get_sort_indicator("transaction_date", &state.get().sort_by, state.get().sort_ascending)}
                                    </span>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=true min_width=140.0>
                                    "Тип транзакции"
                                    <span class=move || get_sort_class("transaction_type", &state.get().sort_by)
                                        on:click=move |_| toggle_sort("transaction_type")
                                        style="cursor: pointer;"
                                    >
                                        {move || get_sort_indicator("transaction_type", &state.get().sort_by, state.get().sort_ascending)}
                                    </span>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=true min_width=160.0>"ID транзакции"</TableHeaderCell>

                                <TableHeaderCell resizable=true min_width=100.0>
                                    "Заказ"
                                    <span class=move || get_sort_class("order_id", &state.get().sort_by)
                                        on:click=move |_| toggle_sort("order_id")
                                        style="cursor: pointer;"
                                    >
                                        {move || get_sort_indicator("order_id", &state.get().sort_by, state.get().sort_ascending)}
                                    </span>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=true min_width=100.0>"Тип заказа"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=120.0>"SKU"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=200.0>"Товар / Услуга"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=60.0>"Кол-во"</TableHeaderCell>

                                <TableHeaderCell resizable=true min_width=100.0>
                                    "Сумма"
                                    <span class=move || get_sort_class("transaction_sum", &state.get().sort_by)
                                        on:click=move |_| toggle_sort("transaction_sum")
                                        style="cursor: pointer;"
                                    >
                                        {move || get_sort_indicator("transaction_sum", &state.get().sort_by, state.get().sort_ascending)}
                                    </span>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=true min_width=100.0>
                                    "Сумма ПП"
                                    <span class=move || get_sort_class("bank_sum", &state.get().sort_by)
                                        on:click=move |_| toggle_sort("bank_sum")
                                        style="cursor: pointer;"
                                    >
                                        {move || get_sort_indicator("bank_sum", &state.get().sort_by, state.get().sort_ascending)}
                                    </span>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=true min_width=100.0>"Статус"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=100.0>"Источник"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=160.0>"Комментарий"</TableHeaderCell>
                            </TableRow>
                        </TableHeader>

                        <TableBody>
                            {move || {
                                if is_loading.get() {
                                    return view! {
                                        <TableRow>
                                            <TableCell attr:colspan="13">
                                                <TableCellLayout>
                                                    "Загрузка..."
                                                </TableCellLayout>
                                            </TableCell>
                                        </TableRow>
                                    }.into_any();
                                }
                                let data = items.get();
                                if data.is_empty() {
                                    return view! {
                                        <TableRow>
                                            <TableCell attr:colspan="13">
                                                <TableCellLayout>
                                                    "Нет данных. Выполните импорт через u503 или проверьте фильтры."
                                                </TableCellLayout>
                                            </TableCell>
                                        </TableRow>
                                    }.into_any();
                                }
                                data.into_iter().map(|row| {
                                    let date_str = row.transaction_date
                                        .as_deref()
                                        .map(fmt_date)
                                        .unwrap_or_default();
                                    let tt = row.transaction_type.clone().unwrap_or_default();
                                    let tid = row.transaction_id.clone().unwrap_or_default();
                                    let oid = row.order_id.map(|v| v.to_string()).unwrap_or_default();
                                    let order_type = row.order_type.clone().unwrap_or_default();
                                    let sku = row.shop_sku.clone().unwrap_or_default();
                                    let offer = row.offer_or_service_name.clone().unwrap_or_default();
                                    let cnt = row.count.map(|v| v.to_string()).unwrap_or_default();
                                    let status = row.payment_status.clone().unwrap_or_default();
                                    let source = row.transaction_source.clone().unwrap_or_default();
                                    let comments = row.comments.clone().unwrap_or_default();
                                    let ts = row.transaction_sum;
                                    let bs = row.bank_sum;
                                    let record_key = row.record_key.clone();
                                    let date_for_link = date_str.clone();

                                    view! {
                                        <TableRow>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    <a
                                                        href="#"
                                                        style="color: var(--colorBrandForeground1); text-decoration: none; cursor: pointer;"
                                                        on:click=move |e| {
                                                            e.prevent_default();
                                                            open_detail(record_key.clone(), date_for_link.clone());
                                                        }
                                                    >
                                                        {date_str}
                                                    </a>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {tt}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    <span style="font-family: monospace; font-size: 0.85em;">{tid}</span>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {oid}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {order_type}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {sku}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {offer}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {cnt}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCellMoney value=ts />
                                            <TableCellMoney value=bs />
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {status}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {source}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {comments}
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
