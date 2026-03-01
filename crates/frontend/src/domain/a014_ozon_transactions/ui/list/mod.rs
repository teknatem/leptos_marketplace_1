pub mod state;

use self::state::create_state;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::components::date_input::DateInput;
use crate::shared::components::month_selector::MonthSelector;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::table::{
    TableCellCheckbox, TableCellMoney, TableCrosshairHighlight, TableHeaderCheckbox,
};
use crate::shared::components::ui::badge::Badge as UiBadge;
use crate::shared::icons::icon;
use crate::shared::list_utils::{format_number, get_sort_class, get_sort_indicator, Sortable};
use crate::shared::page_frame::PageFrame;
use crate::shared::table_utils::init_column_resize;
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::cmp::Ordering;
use thaw::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

use crate::shared::page_standard::PAGE_CAT_LIST;

fn format_date(date_str: &str) -> String {
    let date_part = date_str.split_whitespace().next().unwrap_or(date_str);
    let date_part = date_part.split('T').next().unwrap_or(date_part);
    if let Some((year, rest)) = date_part.split_once('-') {
        if let Some((month, day)) = rest.split_once('-') {
            return format!("{}.{}.{}", day, month, year);
        }
    }
    date_str.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OzonTransactionsDto {
    pub id: String,
    #[serde(rename = "operation_id")]
    pub operation_id: i64,
    pub operation_type: String,
    pub operation_type_name: String,
    pub operation_date: String,
    pub posting_number: String,
    pub transaction_type: String,
    pub delivery_schema: String,
    pub amount: f64,
    pub accruals_for_sale: f64,
    pub sale_commission: f64,
    pub delivery_charge: f64,
    pub substatus: Option<String>,
    pub delivering_date: Option<String>,
    pub is_posted: bool,
}

impl Sortable for OzonTransactionsDto {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "operation_id" => self.operation_id.cmp(&other.operation_id),
            "operation_type" => self.operation_type.to_lowercase().cmp(&other.operation_type.to_lowercase()),
            "operation_type_name" => self.operation_type_name.to_lowercase().cmp(&other.operation_type_name.to_lowercase()),
            "operation_date" => self.operation_date.cmp(&other.operation_date),
            "posting_number" => self.posting_number.to_lowercase().cmp(&other.posting_number.to_lowercase()),
            "transaction_type" => self.transaction_type.to_lowercase().cmp(&other.transaction_type.to_lowercase()),
            "delivery_schema" => self.delivery_schema.to_lowercase().cmp(&other.delivery_schema.to_lowercase()),
            "amount" => self.amount.partial_cmp(&other.amount).unwrap_or(Ordering::Equal),
            "accruals_for_sale" => self.accruals_for_sale.partial_cmp(&other.accruals_for_sale).unwrap_or(Ordering::Equal),
            "sale_commission" => self.sale_commission.partial_cmp(&other.sale_commission).unwrap_or(Ordering::Equal),
            "delivery_charge" => self.delivery_charge.partial_cmp(&other.delivery_charge).unwrap_or(Ordering::Equal),
            "substatus" => match (&self.substatus, &other.substatus) {
                (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
                (Some(_), None) => Ordering::Greater,
                (None, Some(_)) => Ordering::Less,
                (None, None) => Ordering::Equal,
            },
            "delivering_date" => match (&self.delivering_date, &other.delivering_date) {
                (Some(a), Some(b)) => a.cmp(b),
                (Some(_), None) => Ordering::Greater,
                (None, Some(_)) => Ordering::Less,
                (None, None) => Ordering::Equal,
            },
            "is_posted" => self.is_posted.cmp(&other.is_posted),
            _ => Ordering::Equal,
        }
    }
}

const TABLE_ID: &str = "a014-ozon-transactions-table";
const COLUMN_WIDTHS_KEY: &str = "a014_ozon_transactions_column_widths";
const FORM_KEY: &str = "a014_ozon_transactions";

#[component]
pub fn OzonTransactionsList() -> impl IntoView {
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");
    let state = create_state();
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (is_filter_expanded, set_is_filter_expanded) = signal(true);
    let (posting_in_progress, set_posting_in_progress) = signal(false);
    let (current_operation, set_current_operation) = signal::<Option<(usize, usize)>>(None);
    let (save_notification, set_save_notification) = signal(None::<String>);

    // All loaded transactions (server-filtered, client-paginated)
    let all_transactions: RwSignal<Vec<OzonTransactionsDto>> = RwSignal::new(Vec::new());

    let refresh_pagination = move || {
        let source = all_transactions.get_untracked();
        let field = state.with_untracked(|s| s.sort_field.clone());
        let ascending = state.with_untracked(|s| s.sort_ascending);
        let page_size = state.with_untracked(|s| s.page_size);
        let page = state.with_untracked(|s| s.page);

        let mut sorted = source.clone();
        sorted.sort_by(|a, b| {
            let cmp = a.compare_by_field(b, &field);
            if ascending { cmp } else { cmp.reverse() }
        });

        let total = sorted.len();
        let total_pages = if total == 0 { 0 } else { (total + page_size - 1) / page_size };
        let page = page.min(if total_pages == 0 { 0 } else { total_pages - 1 });
        let start = page * page_size;
        let end = (start + page_size).min(total);
        let page_items = if start < total { sorted[start..end].to_vec() } else { vec![] };

        state.update(|s| {
            s.transactions = page_items;
            s.total_count = total;
            s.total_pages = total_pages;
            s.page = page;
        });
    };

    let load_transactions = move || {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let date_from_val = state.with_untracked(|s| s.date_from.clone());
            let date_to_val = state.with_untracked(|s| s.date_to.clone());
            let transaction_type_val = state.with_untracked(|s| s.transaction_type_filter.clone());
            let operation_type_name_val = state.with_untracked(|s| s.operation_type_name_filter.clone());
            let posting_number_val = state.with_untracked(|s| s.posting_number_filter.clone());

            let mut query_params = format!("?date_from={}&date_to={}", date_from_val, date_to_val);
            if !transaction_type_val.is_empty() {
                query_params.push_str(&format!("&transaction_type={}", transaction_type_val));
            }
            if !operation_type_name_val.is_empty() {
                query_params.push_str(&format!("&operation_type_name={}", operation_type_name_val));
            }
            if !posting_number_val.is_empty() {
                query_params.push_str(&format!("&posting_number={}", posting_number_val));
            }

            let url = format!("{}/api/ozon_transactions{}", api_base(), query_params);
            log!("Fetching transactions: {}", url);

            match Request::get(&url).send().await {
                Ok(response) => {
                    if response.status() == 200 {
                        match response.json::<Vec<OzonTransactionsDto>>().await {
                            Ok(items) => {
                                log!("Loaded {} OZON transactions", items.len());
                                all_transactions.set(items);
                                state.update(|s| { s.page = 0; s.is_loaded = true; });
                                refresh_pagination();
                            }
                            Err(e) => set_error.set(Some(format!("Ошибка парсинга: {}", e))),
                        }
                    } else {
                        set_error.set(Some(format!("Ошибка сервера: {}", response.status())));
                    }
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(format!("Ошибка сети: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            spawn_local(async move {
                match load_saved_settings(FORM_KEY).await {
                    Ok(Some(settings)) => {
                        state.update(|s| {
                            if let Some(v) = settings.get("date_from").and_then(|v| v.as_str()) { s.date_from = v.to_string(); }
                            if let Some(v) = settings.get("date_to").and_then(|v| v.as_str()) { s.date_to = v.to_string(); }
                            if let Some(v) = settings.get("transaction_type_filter").and_then(|v| v.as_str()) { s.transaction_type_filter = v.to_string(); }
                            if let Some(v) = settings.get("operation_type_name_filter").and_then(|v| v.as_str()) { s.operation_type_name_filter = v.to_string(); }
                            if let Some(v) = settings.get("posting_number_filter").and_then(|v| v.as_str()) { s.posting_number_filter = v.to_string(); }
                        });
                        log!("Loaded saved settings for A014");
                    }
                    Ok(None) => log!("No saved settings for A014"),
                    Err(e) => log!("Failed to load settings: {}", e),
                }
                load_transactions();
            });
        }
    });

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

    let open_detail = move |id: String, operation_id: i64| {
        tabs_store.open_tab(
            &format!("a014_ozon_transactions_detail_{}", id),
            &format!("Транзакция OZON #{}", operation_id),
        );
    };

    let post_batch = move |post: bool| {
        let ids: Vec<String> = state.with_untracked(|s| s.selected_ids.iter().cloned().collect());
        if ids.is_empty() { return; }
        let total = ids.len();
        set_posting_in_progress.set(true);
        set_current_operation.set(Some((0, total)));
        spawn_local(async move {
            let action = if post { "post" } else { "unpost" };
            for (index, id) in ids.iter().enumerate() {
                set_current_operation.set(Some((index + 1, total)));
                let url = format!("{}/api/a014/ozon-transactions/{}/{}", api_base(), id, action);
                let _ = Request::post(&url).send().await;
            }
            set_posting_in_progress.set(false);
            set_current_operation.set(None);
            state.update(|s| s.selected_ids.clear());
            load_transactions();
        });
    };

    let save_settings = move |_| {
        let settings = json!({
            "date_from": state.with(|s| s.date_from.clone()),
            "date_to": state.with(|s| s.date_to.clone()),
            "transaction_type_filter": state.with(|s| s.transaction_type_filter.clone()),
            "operation_type_name_filter": state.with(|s| s.operation_type_name_filter.clone()),
            "posting_number_filter": state.with(|s| s.posting_number_filter.clone()),
        });
        spawn_local(async move {
            match save_settings_to_database(FORM_KEY, settings).await {
                Ok(_) => {
                    set_save_notification.set(Some("Настройки сохранены".to_string()));
                    spawn_local(async move {
                        gloo_timers::future::TimeoutFuture::new(3000).await;
                        set_save_notification.set(None);
                    });
                }
                Err(e) => {
                    set_save_notification.set(Some(format!("Ошибка: {}", e)));
                    log!("Failed to save settings: {}", e);
                }
            }
        });
    };

    let active_filters_count = Signal::derive(move || {
        let s = state.get();
        let mut count = 0;
        if !s.date_from.is_empty() { count += 1; }
        if !s.date_to.is_empty() { count += 1; }
        if !s.transaction_type_filter.is_empty() { count += 1; }
        if !s.operation_type_name_filter.is_empty() { count += 1; }
        if !s.posting_number_filter.is_empty() { count += 1; }
        count
    });

    let toggle_sort = move |field: &'static str| {
        state.update(|s| {
            if s.sort_field == field { s.sort_ascending = !s.sort_ascending; }
            else { s.sort_field = field.to_string(); s.sort_ascending = true; }
            s.page = 0;
        });
        refresh_pagination();
    };

    let go_to_page = move |new_page: usize| {
        state.update(|s| s.page = new_page);
        refresh_pagination();
    };

    let change_page_size = move |new_size: usize| {
        state.update(|s| { s.page_size = new_size; s.page = 0; });
        refresh_pagination();
    };

    let selected = RwSignal::new(state.with_untracked(|s| s.selected_ids.clone()));

    let toggle_selection = move |id: String, checked: bool| {
        selected.update(|s| { if checked { s.insert(id.clone()); } else { s.remove(&id); } });
        state.update(|s| { if checked { s.selected_ids.insert(id); } else { s.selected_ids.remove(&id); } });
    };

    let toggle_all = move |check_all: bool| {
        let items = state.get().transactions;
        if check_all {
            selected.update(|s| { s.clear(); for item in items.iter() { s.insert(item.id.clone()); } });
            state.update(|s| { s.selected_ids.clear(); for item in items.iter() { s.selected_ids.insert(item.id.clone()); } });
        } else {
            selected.update(|s| s.clear());
            state.update(|s| s.selected_ids.clear());
        }
    };

    let items_signal = Signal::derive(move || state.get().transactions);
    let selected_signal = Signal::derive(move || selected.get());
    let selected_count = Signal::derive(move || selected.get().len());

    // Totals over all loaded transactions
    let totals = Signal::derive(move || {
        let data = all_transactions.get();
        let total_amount: f64 = data.iter().map(|t| t.amount).sum();
        let total_accruals: f64 = data.iter().map(|t| t.accruals_for_sale).sum();
        let total_commission: f64 = data.iter().map(|t| t.sale_commission).sum();
        let total_delivery: f64 = data.iter().map(|t| t.delivery_charge).sum();
        (data.len(), total_amount, total_accruals, total_commission, total_delivery)
    });

    let transaction_type_filter = RwSignal::new(state.get_untracked().transaction_type_filter);
    let operation_type_name_filter = RwSignal::new(state.get_untracked().operation_type_name_filter);
    let posting_number_filter = RwSignal::new(state.get_untracked().posting_number_filter);

    view! {
        <PageFrame page_id="a014_ozon_transactions--list" category=PAGE_CAT_LIST>
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Транзакции OZON"</h1>
                    <UiBadge variant="primary".to_string()>
                        {move || state.get().total_count.to_string()}
                    </UiBadge>
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Subtle
                        on_click=move |_| post_batch(true)
                        disabled=Signal::derive(move || selected_count.get() == 0 || posting_in_progress.get())
                    >
                        {move || format!("Провести ({})", selected_count.get())}
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Subtle
                        on_click=move |_| post_batch(false)
                        disabled=Signal::derive(move || selected_count.get() == 0 || posting_in_progress.get())
                    >
                        {move || format!("Отменить ({})", selected_count.get())}
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| {
                            let data = all_transactions.get_untracked();
                            if let Err(e) = export_to_csv(&data) {
                                log!("Failed to export: {}", e);
                            }
                        }
                        disabled=Signal::derive(move || loading.get() || all_transactions.get().is_empty())
                    >
                        {icon("download")}
                        " Excel"
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=save_settings
                    >
                        {icon("save")}
                    </Button>
                    {move || save_notification.get().map(|msg| view! {
                        <span class="page__status">{msg}</span>
                    })}
                    {move || current_operation.get().map(|(cur, total)| view! {
                        <span class="page__status">{format!("Обработка {}/{} ...", cur, total)}</span>
                    })}
                </div>
            </div>

            <div class="page__content">
                <div class="filter-panel">
                    <div class="filter-panel-header">
                        <div
                            class="filter-panel-header__left"
                            on:click=move |_| set_is_filter_expanded.update(|e| *e = !*e)
                        >
                            <svg
                                width="16" height="16" viewBox="0 0 24 24"
                                fill="none" stroke="currentColor" stroke-width="2"
                                stroke-linecap="round" stroke-linejoin="round"
                                class=move || if is_filter_expanded.get() {
                                    "filter-panel__chevron filter-panel__chevron--expanded"
                                } else {
                                    "filter-panel__chevron"
                                }
                            >
                                <polyline points="6 9 12 15 18 9"></polyline>
                            </svg>
                            {icon("filter")}
                            <span class="filter-panel__title">"Фильтры"</span>
                            {move || {
                                let count = active_filters_count.get();
                                if count > 0 {
                                    view! { <span class="filter-panel__badge">{count}</span> }.into_any()
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
                                page_size_options=vec![100, 200, 500, 1000]
                            />
                        </div>

                        <div class="filter-panel-header__right">
                            <Button
                                appearance=ButtonAppearance::Primary
                                on_click=move |_| {
                                    state.update(|s| {
                                        s.transaction_type_filter = transaction_type_filter.get_untracked();
                                        s.operation_type_name_filter = operation_type_name_filter.get_untracked();
                                        s.posting_number_filter = posting_number_filter.get_untracked();
                                        s.page = 0;
                                    });
                                    load_transactions();
                                }
                                disabled=Signal::derive(move || loading.get())
                            >
                                {move || if loading.get() { "Загрузка..." } else { "Применить" }}
                            </Button>
                        </div>
                    </div>

                    <Show when=move || is_filter_expanded.get()>
                        <div class="filter-panel-content">
                            <Flex gap=FlexGap::Small align=FlexAlign::End>
                                <div style="display: flex; align-items: center; gap: 8px;">
                                    <label style="font-size: 0.875rem; font-weight: 500; white-space: nowrap;">"Период:"</label>
                                    <DateInput
                                        value=Signal::derive(move || state.get().date_from)
                                        on_change=move |val| state.update(|s| s.date_from = val)
                                    />
                                    <span>"—"</span>
                                    <DateInput
                                        value=Signal::derive(move || state.get().date_to)
                                        on_change=move |val| state.update(|s| s.date_to = val)
                                    />
                                    <MonthSelector
                                        on_select=Callback::new(move |(from, to)| {
                                            state.update(|s| { s.date_from = from; s.date_to = to; });
                                        })
                                    />
                                </div>

                                <div style="display: flex; align-items: center; gap: 8px;">
                                    <label style="font-size: 0.875rem; font-weight: 500; white-space: nowrap;">"Тип транзакции:"</label>
                                    <select
                                        prop:value=move || transaction_type_filter.get()
                                        on:change=move |ev| {
                                            transaction_type_filter.set(event_target_value(&ev));
                                        }
                                        style="padding: 6px 10px; border: 1px solid var(--color-border); border-radius: 4px; font-size: 0.875rem; background: var(--color-background);"
                                    >
                                        <option value="">"Все"</option>
                                        <option value="orders">"orders"</option>
                                        <option value="returns">"returns"</option>
                                        <option value="client_returns">"client_returns"</option>
                                        <option value="services">"services"</option>
                                        <option value="other">"other"</option>
                                        <option value="transfer_delivery">"transfer_delivery"</option>
                                    </select>
                                </div>

                                <div style="flex: 1; max-width: 200px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Тип операции:"</Label>
                                        <Input
                                            value=operation_type_name_filter
                                            placeholder="Поиск..."
                                        />
                                    </Flex>
                                </div>

                                <div style="flex: 1; max-width: 200px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Posting №:"</Label>
                                        <Input
                                            value=posting_number_filter
                                            placeholder="Поиск..."
                                        />
                                    </Flex>
                                </div>
                            </Flex>
                        </div>
                    </Show>
                </div>

                {move || {
                    let (count, total_amount, total_accruals, total_commission, total_delivery) = totals.get();
                    if count > 0 {
                        view! {
                            <div class="filter-panel-totals">
                                <span>"Всего: " {count} " | "</span>
                                <span>"Сумма: " {format_number(total_amount)} " | "</span>
                                <span>"Начисления: " {format_number(total_accruals)} " | "</span>
                                <span>"Комиссия: " {format_number(total_commission)} " | "</span>
                                <span>"Доставка: " {format_number(total_delivery)}</span>
                            </div>
                        }.into_any()
                    } else {
                        view! { <></> }.into_any()
                    }
                }}

                {move || error.get().map(|err| view! { <div class="alert alert--error">{err}</div> })}

                <div class="table-wrapper">
                    <TableCrosshairHighlight table_id=TABLE_ID.to_string() />

                    <Table attr:id=TABLE_ID attr:style="width: 100%; min-width: 1200px; font-size: 0.85em;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCheckbox
                                    items=items_signal
                                    selected=selected_signal
                                    get_id=Callback::new(|row: OzonTransactionsDto| row.id.clone())
                                    on_change=Callback::new(toggle_all)
                                />

                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("operation_date")>
                                        "Дата"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "operation_date"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "operation_date", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=100.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("operation_id")>
                                        "Operation ID"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "operation_id"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "operation_id", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=160.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("operation_type_name")>
                                        "Тип операции"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "operation_type_name"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "operation_type_name", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("substatus")>
                                        "Substatus"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "substatus"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "substatus", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("delivering_date")>
                                        "Доставка FBS"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "delivering_date"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "delivering_date", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=160.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("posting_number")>
                                        "Posting №"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "posting_number"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "posting_number", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=100.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("transaction_type")>
                                        "Тип"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "transaction_type"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "transaction_type", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=80.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("delivery_schema")>
                                        "Схема"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "delivery_schema"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "delivery_schema", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=100.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("amount")>
                                        "Сумма"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "amount"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "amount", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=100.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("accruals_for_sale")>
                                        "Начисл."
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "accruals_for_sale"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "accruals_for_sale", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=100.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("sale_commission")>
                                        "Комиссия"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "sale_commission"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "sale_commission", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=100.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("delivery_charge")>
                                        "Доставка"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "delivery_charge"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "delivery_charge", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=80.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("is_posted")>
                                        "Провед."
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "is_posted"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "is_posted", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                            </TableRow>
                        </TableHeader>

                        <TableBody>
                            <For
                                each=move || state.get().transactions
                                key=|item| item.id.clone()
                                children=move |item| {
                                    let item_id = item.id.clone();
                                    let item_id_click = item.id.clone();
                                    let operation_id = item.operation_id;
                                    let is_posted = item.is_posted;
                                    let formatted_date = format_date(&item.operation_date);
                                    let delivering = item.delivering_date.as_deref().map(format_date).unwrap_or_default();
                                    let substatus_display = item.substatus.clone().unwrap_or_default();
                                    view! {
                                        <TableRow>
                                            <TableCellCheckbox
                                                item_id=item_id.clone()
                                                selected=selected_signal
                                                on_change=Callback::new(move |(id, checked)| toggle_selection(id, checked))
                                            />
                                            <TableCell>
                                                <TableCellLayout>
                                                    <a href="#" class="table__link"
                                                        on:click=move |e| {
                                                            e.prevent_default();
                                                            open_detail(item_id_click.clone(), operation_id);
                                                        }
                                                    >
                                                        {formatted_date}
                                                    </a>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    <span style="font-variant-numeric: tabular-nums;">{item.operation_id}</span>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {item.operation_type_name.clone()}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {substatus_display}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>{delivering}</TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {item.posting_number.clone()}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {item.transaction_type.clone()}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>{item.delivery_schema.clone()}</TableCellLayout>
                                            </TableCell>
                                            <TableCellMoney
                                                value=Signal::derive(move || Some(item.amount))
                                                show_currency=false
                                                color_by_sign=true
                                            />
                                            <TableCellMoney
                                                value=Signal::derive(move || Some(item.accruals_for_sale))
                                                show_currency=false
                                                color_by_sign=false
                                            />
                                            <TableCellMoney
                                                value=Signal::derive(move || Some(item.sale_commission))
                                                show_currency=false
                                                color_by_sign=true
                                            />
                                            <TableCellMoney
                                                value=Signal::derive(move || Some(item.delivery_charge))
                                                show_currency=false
                                                color_by_sign=true
                                            />
                                            <TableCell>
                                                <TableCellLayout>
                                                    {if is_posted {
                                                        view! { <span class="badge badge--success">"Да"</span> }.into_any()
                                                    } else {
                                                        view! { <span class="badge badge--neutral">"Нет"</span> }.into_any()
                                                    }}
                                                </TableCellLayout>
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

async fn load_saved_settings(form_key: &str) -> Result<Option<serde_json::Value>, String> {
    use web_sys::{Request as WebRequest, RequestInit, RequestMode, Response};
    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);
    let url = format!("/api/form-settings/{}", form_key);
    let request = WebRequest::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request.headers().set("Accept", "application/json").map_err(|e| format!("{e:?}"))?;
    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await.map_err(|e| format!("{e:?}"))?;
    let resp: web_sys::Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await.map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    let response: Option<serde_json::Value> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    if let Some(form_settings) = response {
        if let Some(settings_json) = form_settings.get("settings_json").and_then(|v| v.as_str()) {
            let settings: serde_json::Value = serde_json::from_str(settings_json).map_err(|e| format!("{e}"))?;
            return Ok(Some(settings));
        }
    }
    Ok(None)
}

async fn save_settings_to_database(form_key: &str, settings: serde_json::Value) -> Result<(), String> {
    use web_sys::{Request as WebRequest, RequestInit, RequestMode};
    let request_body = json!({ "form_key": form_key, "settings": settings });
    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    let body_str = serde_json::to_string(&request_body).map_err(|e| format!("{e}"))?;
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body_str));
    let url = "/api/form-settings";
    let request = WebRequest::new_with_str_and_init(url, &opts).map_err(|e| format!("{e:?}"))?;
    request.headers().set("Content-Type", "application/json").map_err(|e| format!("{e:?}"))?;
    request.headers().set("Accept", "application/json").map_err(|e| format!("{e:?}"))?;
    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await.map_err(|e| format!("{e:?}"))?;
    let resp: web_sys::Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() { return Err(format!("HTTP {}", resp.status())); }
    Ok(())
}

fn export_to_csv(data: &[OzonTransactionsDto]) -> Result<(), String> {
    let mut csv = String::from("\u{FEFF}");
    csv.push_str("Date;Operation ID;Operation Type;Substatus;Delivering Date;Posting Number;Transaction Type;Delivery Schema;Amount;Accruals;Commission;Delivery;Post\n");
    for txn in data {
        let op_date = format_date(&txn.operation_date);
        let substatus = txn.substatus.as_deref().unwrap_or("");
        let delivering_date = txn.delivering_date.as_deref().map(format_date).unwrap_or_default();
        let status = if txn.is_posted { "Да" } else { "Нет" };
        csv.push_str(&format!(
            "\"{}\";{};\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";{};{};{};{};\"{}\"
",
            op_date, txn.operation_id,
            txn.operation_type_name.replace('\"', "\"\""),
            substatus, delivering_date,
            txn.posting_number.replace('\"', "\"\""),
            txn.transaction_type.replace('\"', "\"\""),
            txn.delivery_schema.replace('\"', "\"\""),
            format!("{:.2}", txn.amount).replace(".", ","),
            format!("{:.2}", txn.accruals_for_sale).replace(".", ","),
            format!("{:.2}", txn.sale_commission).replace(".", ","),
            format!("{:.2}", txn.delivery_charge).replace(".", ","),
            status
        ));
    }
    let blob_parts = js_sys::Array::new();
    blob_parts.push(&wasm_bindgen::JsValue::from_str(&csv));
    let blob_props = BlobPropertyBag::new();
    blob_props.set_type("text/csv;charset=utf-8;");
    let blob = Blob::new_with_str_sequence_and_options(&blob_parts, &blob_props)
        .map_err(|e| format!("Failed to create blob: {:?}", e))?;
    let url = Url::create_object_url_with_blob(&blob)
        .map_err(|e| format!("Failed to create URL: {:?}", e))?;
    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let document = window.document().ok_or_else(|| "no document".to_string())?;
    let a = document.create_element("a").map_err(|e| format!("{:?}", e))?
        .dyn_into::<HtmlAnchorElement>().map_err(|e| format!("{:?}", e))?;
    a.set_href(&url);
    a.set_download(&format!("ozon_transactions_{}.csv", chrono::Utc::now().format("%Y%m%d_%H%M%S")));
    a.click();
    Url::revoke_object_url(&url).map_err(|e| format!("Failed to revoke URL: {:?}", e))?;
    Ok(())
}
