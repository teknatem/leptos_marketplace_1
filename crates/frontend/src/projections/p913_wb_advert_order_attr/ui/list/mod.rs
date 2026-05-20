mod state;

use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::icons::icon;
use crate::shared::list_utils::{format_number, get_sort_class, get_sort_indicator};
use crate::shared::page_frame::PageFrame;
use chrono::Datelike;
use contracts::projections::p913_wb_advert_order_attr::dto::{
    WbAdvertOrderAttrDto, WbAdvertOrderAttrListResponse,
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
    if let Some((y, rest)) = v.split_once('-') {
        if let Some((m, d)) = rest.split_once('-') {
            return format!("{}.{}.{}", d, m, y);
        }
    }
    v.to_string()
}

fn fmt_money(v: f64) -> String {
    format!("{:.2}", v)
}

#[component]
pub fn WbAdvertOrderAttrList() -> impl IntoView {
    let tabs_store = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let (default_from, default_to) = current_month_bounds();
    let state = create_state();
    if state.with_untracked(|s| s.date_from.is_empty()) {
        state.update(|s| s.date_from = default_from.clone());
    }
    if state.with_untracked(|s| s.date_to.is_empty()) {
        state.update(|s| s.date_to = default_to.clone());
    }

    let (items, set_items) = signal(Vec::<WbAdvertOrderAttrDto>::new());
    let (is_loading, set_is_loading) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);
    let (filter_open, set_filter_open) = signal(false);

    let date_from = RwSignal::new(state.get_untracked().date_from.clone());
    let date_to = RwSignal::new(state.get_untracked().date_to.clone());
    let turnover_code_filter = RwSignal::new(state.get_untracked().turnover_code.clone());
    let order_key_filter = RwSignal::new(state.get_untracked().order_key.clone());
    let campaign_filter = RwSignal::new(state.get_untracked().wb_advert_campaign_code.clone());

    let active_filters_count = Signal::derive(move || {
        let s = state.get();
        [
            &s.date_from,
            &s.date_to,
            &s.turnover_code,
            &s.order_key,
            &s.wb_advert_campaign_code,
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
        if !s.turnover_code.is_empty() {
            params.push(format!("turnover_code={}", s.turnover_code));
        }
        if !s.order_key.is_empty() {
            params.push(format!("order_key={}", s.order_key));
        }
        if !s.wb_advert_campaign_code.is_empty() {
            params.push(format!(
                "wb_advert_campaign_code={}",
                s.wb_advert_campaign_code
            ));
        }
        let url = format!(
            "{}/api/p913/wb-advert-order-attr?{}",
            api_base(),
            params.join("&")
        );
        leptos::task::spawn_local(async move {
            set_is_loading.set(true);
            set_error.set(None);
            let result: Result<WbAdvertOrderAttrListResponse, String> = async {
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
                serde_wasm_bindgen::from_value::<WbAdvertOrderAttrListResponse>(json)
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
                s.turnover_code = turnover_code_filter.get_untracked();
                s.order_key = order_key_filter.get_untracked();
                s.wb_advert_campaign_code = campaign_filter.get_untracked();
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
            turnover_code_filter.set(String::new());
            order_key_filter.set(String::new());
            campaign_filter.set(String::new());
            state.update(|s| {
                s.date_from = df.clone();
                s.date_to = dt.clone();
                s.turnover_code = String::new();
                s.order_key = String::new();
                s.wb_advert_campaign_code = String::new();
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

    let open_advert_tab = {
        let tabs_store = tabs_store.clone();
        Callback::new(move |doc_id: String| {
            tabs_store.open_tab(
                &format!("a026_wb_advert_daily_details_{}", doc_id),
                &format!("WB Ads {}", &doc_id[..doc_id.len().min(8)]),
            );
        })
    };

    let open_sale_tab = {
        let tabs_store = tabs_store.clone();
        Callback::new(move |doc_id: String| {
            tabs_store.open_tab(
                &format!("a012_wb_sales_details_{}", doc_id),
                &format!("WB Sale {}", &doc_id[..doc_id.len().min(8)]),
            );
        })
    };

    Effect::new({
        let fetch = fetch.clone();
        move |_| fetch.run(())
    });

    view! {
        <PageFrame page_id="p913_wb_advert_order_attr--list" category="list" class="page--wide">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Атрибуция рекламных расходов WB"</h1>
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
                                    <Label>"Тип оборота"</Label>
                                    <select
                                        prop:value=move || turnover_code_filter.get()
                                        on:change=move |ev| turnover_code_filter.set(event_target_value(&ev))
                                    >
                                        <option value="">"Все"</option>
                                        <option value="advert_clicks_order_accrual">"Резерв (a026)"</option>
                                        <option value="advert_clicks_order_expense">"Расход (a012)"</option>
                                    </select>
                                </Flex>
                            </div>
                            <div style="min-width:180px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Заказ (srid)"</Label>
                                    <Input value=order_key_filter placeholder="srid..." />
                                </Flex>
                            </div>
                            <div style="min-width:180px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Кампания WB"</Label>
                                    <Input value=campaign_filter placeholder="ID кампании..." />
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
                    let open_advert = open_advert_tab.clone();
                    let open_sale = open_sale_tab.clone();
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
                        <Table attr:style="width:100%;min-width:850px;">
                            <TableHeader>
                                <TableRow>
                                    <TableHeaderCell resizable=true min_width=90.0>
                                        "Дата"
                                        <span
                                            class=move || get_sort_class("entry_date", &state.get().sort_by)
                                            on:click=move |_| toggle_sort("entry_date")
                                        >
                                            {move || get_sort_indicator("entry_date", &state.get().sort_by, state.get().sort_ascending)}
                                        </span>
                                    </TableHeaderCell>
                                    <TableHeaderCell resizable=true min_width=90.0>
                                        "Тип"
                                        <span
                                            class=move || get_sort_class("turnover_code", &state.get().sort_by)
                                            on:click=move |_| toggle_sort("turnover_code")
                                        >
                                            {move || get_sort_indicator("turnover_code", &state.get().sort_by, state.get().sort_ascending)}
                                        </span>
                                    </TableHeaderCell>
                                    <TableHeaderCell resizable=true min_width=160.0>"Заказ (srid)"</TableHeaderCell>
                                    <TableHeaderCell resizable=true min_width=100.0>"Кампания"</TableHeaderCell>
                                    <TableHeaderCell resizable=true min_width=100.0>
                                        "Сумма"
                                        <span
                                            class=move || get_sort_class("amount", &state.get().sort_by)
                                            on:click=move |_| toggle_sort("amount")
                                        >
                                            {move || get_sort_indicator("amount", &state.get().sort_by, state.get().sort_ascending)}
                                        </span>
                                    </TableHeaderCell>
                                    <TableHeaderCell resizable=true min_width=90.0>"Регистратор"</TableHeaderCell>
                                    <TableHeaderCell resizable=true min_width=80.0>"Статус"</TableHeaderCell>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                <For each=move || items.get() key=|r| r.id.clone() children=move |r| {
                                    let type_label = if r.turnover_code == "advert_clicks_order_accrual" { "Резерв" } else { "Расход" };
                                    let type_color = if r.turnover_code == "advert_clicks_order_accrual" { BadgeColor::Brand } else { BadgeColor::Success };
                                    let order_display = if r.order_key.is_empty() { "—".to_string() } else { r.order_key.clone() };
                                    let (status_color, status_label) = if r.is_problem {
                                        (BadgeColor::Danger, "Проблема")
                                    } else {
                                        (BadgeColor::Success, "OK")
                                    };
                                    let reg_ref = r.registrator_ref.clone();
                                    let reg_type = r.registrator_type.clone();
                                    let open_advert2 = open_advert.clone();
                                    let open_sale2 = open_sale.clone();
                                    view! {
                                        <TableRow>
                                            <TableCell><TableCellLayout>{fmt_date(&r.entry_date)}</TableCellLayout></TableCell>
                                            <TableCell><TableCellLayout>
                                                <Badge appearance=BadgeAppearance::Tint color=type_color>{type_label}</Badge>
                                            </TableCellLayout></TableCell>
                                            <TableCell><TableCellLayout truncate=true>{order_display}</TableCellLayout></TableCell>
                                            <TableCell><TableCellLayout truncate=true>{r.wb_advert_campaign_code.clone()}</TableCellLayout></TableCell>
                                            <TableCell class="text-right"><TableCellLayout>{fmt_money(r.amount)}</TableCellLayout></TableCell>
                                            <TableCell><TableCellLayout>
                                                {if reg_type == "a026_wb_advert_daily" {
                                                    let id = reg_ref.clone();
                                                    view! {
                                                        <a href="#" class="table__link" on:click=move |e| {
                                                            e.prevent_default();
                                                            open_advert2.run(id.clone());
                                                        }>"a026"</a>
                                                    }.into_any()
                                                } else {
                                                    let id = reg_ref.clone();
                                                    view! {
                                                        <a href="#" class="table__link" on:click=move |e| {
                                                            e.prevent_default();
                                                            open_sale2.run(id.clone());
                                                        }>"a012"</a>
                                                    }.into_any()
                                                }}
                                            </TableCellLayout></TableCell>
                                            <TableCell><TableCellLayout>
                                                <Badge appearance=BadgeAppearance::Tint color=status_color>{status_label}</Badge>
                                            </TableCellLayout></TableCell>
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
