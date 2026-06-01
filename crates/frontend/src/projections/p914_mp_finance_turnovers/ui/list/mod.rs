mod state;

use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::icons::icon;
use crate::shared::list_utils::{format_number, get_sort_class, get_sort_indicator};
use crate::shared::page_frame::PageFrame;
use chrono::Datelike;
use contracts::projections::p914_mp_finance_turnovers::dto::{
    MpFinanceTurnoverDto, MpFinanceTurnoverListResponse,
};
use leptos::prelude::*;
use state::{create_state, persist_state};
use thaw::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

fn current_month_bounds() -> (String, String) {
    let now = chrono::Utc::now().date_naive();
    let year = now.year();
    let month = now.month();
    let start = chrono::NaiveDate::from_ymd_opt(year, month, 1)
        .unwrap()
        .format("%Y-%m-%d")
        .to_string();
    let end = if month == 12 {
        chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap() - chrono::Duration::days(1)
    } else {
        chrono::NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap() - chrono::Duration::days(1)
    }
    .format("%Y-%m-%d")
    .to_string();
    (start, end)
}

fn fmt_date(v: &str) -> String {
    // transaction_date может быть "YYYY-MM-DD" или "YYYY-MM-DD HH:MM".
    let date_part = v.split([' ', 'T']).next().unwrap_or(v);
    if let Some((y, rest)) = date_part.split_once('-') {
        if let Some((m, d)) = rest.split_once('-') {
            return format!("{}.{}.{}", d, m, y);
        }
    }
    v.to_string()
}

fn fmt_money(v: f64) -> String {
    format!("{:.2}", v)
}

fn event_kind_label(code: &str) -> &'static str {
    match code {
        "sold" => "Продажа",
        "returned" => "Возврат",
        "fee" => "Удержание",
        "adjustment" => "Корректировка",
        "ordered" => "Заказ",
        _ => "Прочее",
    }
}

fn event_kind_color(code: &str) -> BadgeColor {
    match code {
        "sold" => BadgeColor::Success,
        "returned" => BadgeColor::Danger,
        "fee" => BadgeColor::Warning,
        "adjustment" => BadgeColor::Informative,
        _ => BadgeColor::Subtle,
    }
}

fn registrator_short_label(registrator_type: &str) -> &'static str {
    match registrator_type {
        "p903_wb_finance_report" => "WB p903",
        "p907_ym_payment_report" => "YM p907",
        _ => "—",
    }
}

fn customer_kind_label(code: &str) -> &'static str {
    match code {
        "URL" => "Юрлицо",
        "FIZ" => "Физлицо",
        _ => "—",
    }
}

#[component]
pub fn MpFinanceTurnoverList() -> impl IntoView {
    let tabs_store = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let (default_from, default_to) = current_month_bounds();
    let state = create_state();
    if state.with_untracked(|s| s.date_from.is_empty()) {
        state.update(|s| s.date_from = default_from.clone());
    }
    if state.with_untracked(|s| s.date_to.is_empty()) {
        state.update(|s| s.date_to = default_to.clone());
    }

    let (items, set_items) = signal(Vec::<MpFinanceTurnoverDto>::new());
    let (is_loading, set_is_loading) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);
    let (filter_open, set_filter_open) = signal(false);

    let date_from = RwSignal::new(state.get_untracked().date_from.clone());
    let date_to = RwSignal::new(state.get_untracked().date_to.clone());
    let registrator_type_filter = RwSignal::new(state.get_untracked().registrator_type.clone());
    let turnover_code_filter = RwSignal::new(state.get_untracked().turnover_code.clone());
    let order_key_filter = RwSignal::new(state.get_untracked().order_key.clone());
    let event_kind_filter = RwSignal::new(state.get_untracked().event_kind.clone());

    let active_filters_count = Signal::derive(move || {
        let s = state.get();
        [
            &s.date_from,
            &s.date_to,
            &s.registrator_type,
            &s.turnover_code,
            &s.order_key,
            &s.event_kind,
        ]
        .iter()
        .filter(|v| !v.is_empty())
        .count()
    });

    let fetch = Callback::new(move |()| {
        let s = state.get_untracked();
        let offset = s.page * s.page_size;
        let mut params = vec![
            format!("limit={}", s.page_size),
            format!("offset={}", offset),
            format!("sort_by={}", s.sort_by),
            format!("sort_desc={}", !s.sort_ascending),
        ];
        if !s.date_from.is_empty() {
            params.push(format!("date_from={}", s.date_from));
        }
        if !s.date_to.is_empty() {
            params.push(format!("date_to={}", s.date_to));
        }
        if !s.registrator_type.is_empty() {
            params.push(format!("registrator_type={}", s.registrator_type));
        }
        if !s.turnover_code.is_empty() {
            params.push(format!("turnover_code={}", s.turnover_code));
        }
        if !s.order_key.is_empty() {
            params.push(format!("order_key={}", s.order_key));
        }
        if !s.event_kind.is_empty() {
            params.push(format!("event_kind={}", s.event_kind));
        }
        let url = format!(
            "{}/api/p914/mp-finance-turnovers?{}",
            api_base(),
            params.join("&")
        );
        leptos::task::spawn_local(async move {
            set_is_loading.set(true);
            set_error.set(None);
            let result: Result<MpFinanceTurnoverListResponse, String> = async {
                let window = web_sys::window().ok_or("no window")?;
                let promise = window.fetch_with_str(&url);
                let response: web_sys::Response = JsFuture::from(promise)
                    .await
                    .map_err(|e| format!("{:?}", e))?
                    .dyn_into()
                    .map_err(|_| "not a Response")?;
                if !response.ok() {
                    return Err(format!("HTTP {}", response.status()));
                }
                let json = JsFuture::from(response.json().map_err(|_| "json()")?)
                    .await
                    .map_err(|e| format!("{:?}", e))?;
                serde_wasm_bindgen::from_value::<MpFinanceTurnoverListResponse>(json)
                    .map_err(|e| format!("{:?}", e))
            }
            .await;
            match result {
                Ok(resp) => {
                    let total = resp.total_count as usize;
                    state.update(|s| {
                        s.total_count = total;
                        s.total_pages = total.div_ceil(s.page_size);
                        s.is_loaded = true;
                    });
                    set_items.set(resp.items);
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_is_loading.set(false);
        });
    });

    let apply_filters = {
        let fetch = fetch.clone();
        Callback::new(move |()| {
            state.update(|s| {
                s.date_from = date_from.get_untracked();
                s.date_to = date_to.get_untracked();
                s.registrator_type = registrator_type_filter.get_untracked();
                s.turnover_code = turnover_code_filter.get_untracked();
                s.order_key = order_key_filter.get_untracked();
                s.event_kind = event_kind_filter.get_untracked();
                s.page = 0;
            });
            persist_state(state);
            fetch.run(());
        })
    };

    let reset_filters = {
        let fetch = fetch.clone();
        let (df, dt) = current_month_bounds();
        Callback::new(move |()| {
            date_from.set(df.clone());
            date_to.set(dt.clone());
            registrator_type_filter.set(String::new());
            turnover_code_filter.set(String::new());
            order_key_filter.set(String::new());
            event_kind_filter.set(String::new());
            state.update(|s| {
                s.date_from = df.clone();
                s.date_to = dt.clone();
                s.registrator_type = String::new();
                s.turnover_code = String::new();
                s.order_key = String::new();
                s.event_kind = String::new();
                s.page = 0;
            });
            persist_state(state);
            fetch.run(());
        })
    };

    let go_to_page = {
        let fetch = fetch.clone();
        move |page: usize| {
            state.update(|s| s.page = page);
            persist_state(state);
            fetch.run(());
        }
    };

    let change_page_size = {
        let fetch = fetch.clone();
        move |size: usize| {
            state.update(|s| {
                s.page_size = size;
                s.page = 0;
            });
            persist_state(state);
            fetch.run(());
        }
    };

    let toggle_sort = {
        let fetch = fetch.clone();
        move |field: &'static str| {
            state.update(|s| {
                if s.sort_by == field {
                    s.sort_ascending = !s.sort_ascending;
                } else {
                    s.sort_by = field.to_string();
                    s.sort_ascending = false;
                }
                s.page = 0;
            });
            persist_state(state);
            fetch.run(());
        }
    };

    let open_registrator_tab = {
        let tabs_store = tabs_store.clone();
        Callback::new(move |(reg_type, reg_ref): (String, String)| {
            let short = &reg_ref[..reg_ref.len().min(8)];
            match reg_type.as_str() {
                "p903_wb_finance_report" => tabs_store.open_tab(
                    &format!("p903_wb_finance_report_details_id_{}", reg_ref),
                    &format!("WB p903 {}", short),
                ),
                "p907_ym_payment_report" => tabs_store.open_tab(
                    &format!("p907_ym_payment_report_details_{}", reg_ref),
                    &format!("YM p907 {}", short),
                ),
                _ => {}
            }
        })
    };

    let open_order_tab = {
        let tabs_store = tabs_store.clone();
        Callback::new(move |(order_type, order_ref): (String, String)| {
            let short = &order_ref[..order_ref.len().min(8)];
            match order_type.as_str() {
                "a015_wb_orders" => tabs_store.open_tab(
                    &format!("a015_wb_orders_details_{}", order_ref),
                    &format!("WB заказ {}", short),
                ),
                "a013_ym_order" => tabs_store.open_tab(
                    &format!("a013_ym_order_details_{}", order_ref),
                    &format!("YM заказ {}", short),
                ),
                _ => {}
            }
        })
    };

    Effect::new({
        let fetch = fetch.clone();
        move |_| fetch.run(())
    });

    view! {
        <PageFrame page_id="p914_mp_finance_turnovers--list" category="list" class="page--wide">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Финансовые обороты (fina)"</h1>
                    {move || {
                        let s = state.get();
                        if s.is_loaded {
                            view! {
                                <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Informative>
                                    {format!("Строк: {}", format_number(s.total_count as f64))}
                                </Badge>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }
                    }}
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| set_filter_open.update(|v| *v = !*v)
                    >
                        {icon("filter")}
                        "Фильтры"
                        {move || {
                            let n = active_filters_count.get();
                            if n > 0 {
                                view! {
                                    <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Danger>{n}</Badge>
                                }.into_any()
                            } else {
                                view! { <span></span> }.into_any()
                            }
                        }}
                    </Button>
                </div>
            </div>

            <Show when=move || filter_open.get()>
                <div class="filter-panel">
                    <div class="filter-panel-content">
                        <Flex gap=FlexGap::Medium>
                            <div style="min-width:160px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Дата с"</Label>
                                    <input
                                        type="date"
                                        prop:value=move || date_from.get()
                                        on:input=move |ev| date_from.set(event_target_value(&ev))
                                    />
                                </Flex>
                            </div>
                            <div style="min-width:160px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Дата по"</Label>
                                    <input
                                        type="date"
                                        prop:value=move || date_to.get()
                                        on:input=move |ev| date_to.set(event_target_value(&ev))
                                    />
                                </Flex>
                            </div>
                            <div style="min-width:160px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Источник"</Label>
                                    <select
                                        prop:value=move || registrator_type_filter.get()
                                        on:change=move |ev| registrator_type_filter.set(event_target_value(&ev))
                                    >
                                        <option value="">"Все"</option>
                                        <option value="p903_wb_finance_report">"WB (p903)"</option>
                                        <option value="p907_ym_payment_report">"YM (p907)"</option>
                                    </select>
                                </Flex>
                            </div>
                            <div style="min-width:160px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Событие"</Label>
                                    <select
                                        prop:value=move || event_kind_filter.get()
                                        on:change=move |ev| event_kind_filter.set(event_target_value(&ev))
                                    >
                                        <option value="">"Все"</option>
                                        <option value="sold">"Продажа"</option>
                                        <option value="returned">"Возврат"</option>
                                        <option value="fee">"Удержание"</option>
                                        <option value="adjustment">"Корректировка"</option>
                                    </select>
                                </Flex>
                            </div>
                            <div style="min-width:180px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Тип оборота"</Label>
                                    <Input value=turnover_code_filter placeholder="turnover_code..." />
                                </Flex>
                            </div>
                            <div style="min-width:180px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Заказ"</Label>
                                    <Input value=order_key_filter placeholder="srid / order_id..." />
                                </Flex>
                            </div>
                        </Flex>
                        <Flex gap=FlexGap::Small style="margin-top: 8px;">
                            <Button appearance=ButtonAppearance::Primary on_click=move |_| apply_filters.run(())>
                                {icon("search")} "Найти"
                            </Button>
                            <Button appearance=ButtonAppearance::Secondary on_click=move |_| reset_filters.run(())>
                                "Сбросить"
                            </Button>
                        </Flex>
                    </div>
                </div>
            </Show>

            <div class="page__content">
                {move || if is_loading.get() {
                    view! {
                        <Flex gap=FlexGap::Small style="align-items:center;justify-content:center;padding:var(--spacing-4xl);">
                            <Spinner />
                            <span>"Загрузка..."</span>
                        </Flex>
                    }.into_any()
                } else if let Some(err) = error.get() {
                    view! { <div class="alert alert--error">{err}</div> }.into_any()
                } else {
                    let open_registrator = open_registrator_tab.clone();
                    let open_order = open_order_tab.clone();
                    view! {
                        <PaginationControls
                            current_page=Signal::derive(move || state.get().page)
                            total_pages=Signal::derive(move || state.get().total_pages)
                            total_count=Signal::derive(move || state.get().total_count)
                            page_size=Signal::derive(move || state.get().page_size)
                            on_page_change=Callback::new(go_to_page)
                            on_page_size_change=Callback::new(change_page_size)
                        />

                        <div class="table-wrapper">
                        <Table attr:style="width:100%;min-width:1000px;">
                            <TableHeader>
                                <TableRow>
                                    <TableHeaderCell resizable=true min_width=90.0>
                                        "Дата"
                                        <span
                                            class=move || get_sort_class("transaction_date", &state.get().sort_by)
                                            on:click=move |_| toggle_sort("transaction_date")
                                        >
                                            {move || get_sort_indicator("transaction_date", &state.get().sort_by, state.get().sort_ascending)}
                                        </span>
                                    </TableHeaderCell>
                                    <TableHeaderCell resizable=true min_width=140.0>
                                        "Тип оборота"
                                        <span
                                            class=move || get_sort_class("turnover_code", &state.get().sort_by)
                                            on:click=move |_| toggle_sort("turnover_code")
                                        >
                                            {move || get_sort_indicator("turnover_code", &state.get().sort_by, state.get().sort_ascending)}
                                        </span>
                                    </TableHeaderCell>
                                    <TableHeaderCell resizable=true min_width=110.0>
                                        "Событие"
                                        <span
                                            class=move || get_sort_class("event_kind", &state.get().sort_by)
                                            on:click=move |_| toggle_sort("event_kind")
                                        >
                                            {move || get_sort_indicator("event_kind", &state.get().sort_by, state.get().sort_ascending)}
                                        </span>
                                    </TableHeaderCell>
                                    <TableHeaderCell resizable=true min_width=150.0>"Заказ"</TableHeaderCell>
                                    <TableHeaderCell resizable=true min_width=110.0>
                                        "Сумма"
                                        <span
                                            class=move || get_sort_class("amount", &state.get().sort_by)
                                            on:click=move |_| toggle_sort("amount")
                                        >
                                            {move || get_sort_indicator("amount", &state.get().sort_by, state.get().sort_ascending)}
                                        </span>
                                    </TableHeaderCell>
                                    <TableHeaderCell resizable=true min_width=70.0>"Кол-во"</TableHeaderCell>
                                    <TableHeaderCell resizable=true min_width=100.0>"Источник"</TableHeaderCell>
                                    <TableHeaderCell resizable=true min_width=70.0>"Клиент"</TableHeaderCell>
                                    <TableHeaderCell resizable=true min_width=80.0>"Схема"</TableHeaderCell>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                <For each=move || items.get() key=|r| r.id.clone() children=move |r| {
                                    let order_display = if r.order_key.is_empty() { "—".to_string() } else { r.order_key.clone() };
                                    let qty_display = r.quantity.map(|q| format!("{:.0}", q)).unwrap_or_else(|| "—".to_string());
                                    let customer_display = r.customer_kind.as_deref().map(customer_kind_label).unwrap_or("—");
                                    let fulfillment_display = r.fulfillment_type.clone().unwrap_or_else(|| "—".to_string());
                                    let ev = r.event_kind.clone();
                                    let reg_type = r.registrator_type.clone();
                                    let reg_ref = r.registrator_ref.clone();
                                    let order_ref = r.order_ref.clone();
                                    let order_registrator_type = r.order_registrator_type.clone();
                                    let open_registrator2 = open_registrator.clone();
                                    let open_order2 = open_order.clone();
                                    view! {
                                        <TableRow>
                                            <TableCell><TableCellLayout>{fmt_date(&r.transaction_date)}</TableCellLayout></TableCell>
                                            <TableCell><TableCellLayout truncate=true>{r.turnover_code.clone()}</TableCellLayout></TableCell>
                                            <TableCell><TableCellLayout>
                                                <Badge appearance=BadgeAppearance::Tint color=event_kind_color(&ev)>
                                                    {event_kind_label(&ev)}
                                                </Badge>
                                            </TableCellLayout></TableCell>
                                            <TableCell><TableCellLayout truncate=true>
                                                {match (order_ref, order_registrator_type) {
                                                    (Some(oref), Some(otype)) if !oref.is_empty() => {
                                                        let payload = (otype, oref);
                                                        view! {
                                                            <a href="#" class="table__link" on:click=move |e| {
                                                                e.prevent_default();
                                                                open_order2.run(payload.clone());
                                                            }>{order_display}</a>
                                                        }.into_any()
                                                    }
                                                    _ => view! { <span>{order_display}</span> }.into_any(),
                                                }}
                                            </TableCellLayout></TableCell>
                                            <TableCell class="text-right"><TableCellLayout>{fmt_money(r.amount)}</TableCellLayout></TableCell>
                                            <TableCell class="text-right"><TableCellLayout>{qty_display}</TableCellLayout></TableCell>
                                            <TableCell><TableCellLayout>
                                                {
                                                    let label = registrator_short_label(&reg_type);
                                                    if reg_type == "p903_wb_finance_report" || reg_type == "p907_ym_payment_report" {
                                                        let payload = (reg_type.clone(), reg_ref.clone());
                                                        view! {
                                                            <a href="#" class="table__link" on:click=move |e| {
                                                                e.prevent_default();
                                                                open_registrator2.run(payload.clone());
                                                            }>{label}</a>
                                                        }.into_any()
                                                    } else {
                                                        view! { <span>{label}</span> }.into_any()
                                                    }
                                                }
                                            </TableCellLayout></TableCell>
                                            <TableCell><TableCellLayout>{customer_display}</TableCellLayout></TableCell>
                                            <TableCell><TableCellLayout>{fulfillment_display}</TableCellLayout></TableCell>
                                        </TableRow>
                                    }
                                } />
                            </TableBody>
                        </Table>
                        </div>
                    }.into_any()
                }}
            </div>
        </PageFrame>
    }
}
