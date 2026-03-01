pub mod state;

use self::state::{create_state, ServerTotals};
use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::components::date_range_picker::DateRangePicker;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::table::{
    TableCellCheckbox, TableCellMoney, TableCrosshairHighlight, TableHeaderCheckbox,
};
use crate::shared::components::ui::badge::Badge as UiBadge;
use crate::shared::date_utils::format_datetime;
use crate::shared::icons::icon;
use crate::shared::list_utils::{format_number, get_sort_class, get_sort_indicator, Sortable};
use crate::shared::page_frame::PageFrame;
use crate::shared::table_utils::{init_column_resize, was_just_resizing};
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::cmp::Ordering;
use thaw::*;
use wasm_bindgen::JsCast;
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

const TABLE_ID: &str = "a016-ym-returns-table";
const COLUMN_WIDTHS_KEY: &str = "a016_ym_returns_column_widths";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse {
    pub items: Vec<YmReturnDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
    pub totals: Option<ServerTotals>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct YmReturnDto {
    pub id: String,
    pub return_id: i64,
    pub order_id: i64,
    pub return_type: String,
    pub refund_status: String,
    pub total_items: i32,
    pub total_amount: f64,
    pub created_at_source: String,
    pub fetched_at: String,
    pub is_posted: bool,
}

impl Sortable for YmReturnDto {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "return_id" => self.return_id.cmp(&other.return_id),
            "order_id" => self.order_id.cmp(&other.order_id),
            "return_type" => self.return_type.to_lowercase().cmp(&other.return_type.to_lowercase()),
            "refund_status" => self.refund_status.to_lowercase().cmp(&other.refund_status.to_lowercase()),
            "total_items" => self.total_items.cmp(&other.total_items),
            "total_amount" => self.total_amount.partial_cmp(&other.total_amount).unwrap_or(Ordering::Equal),
            "created_at_source" => self.created_at_source.cmp(&other.created_at_source),
            "fetched_at" => self.fetched_at.cmp(&other.fetched_at),
            _ => Ordering::Equal,
        }
    }
}

#[component]
pub fn YmReturnsList() -> impl IntoView {
    let state = create_state();
    let global_ctx = expect_context::<AppGlobalContext>();

    let (items, set_items) = signal::<Vec<YmReturnDto>>(Vec::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (posting_in_progress, set_posting_in_progress) = signal(false);
    let (is_filter_expanded, set_is_filter_expanded) = signal(false);

    let search_return_id = RwSignal::new(state.get_untracked().search_return_id.clone());
    let search_order_id = RwSignal::new(state.get_untracked().search_order_id.clone());
    let filter_type = RwSignal::new(
        state.get_untracked().filter_type.clone().unwrap_or_default(),
    );

    Effect::new(move || {
        let return_id = search_return_id.get();
        untrack(move || {
            state.update(|s| { s.search_return_id = return_id; s.page = 0; });
        });
    });

    Effect::new(move || {
        let order_id = search_order_id.get();
        untrack(move || {
            state.update(|s| { s.search_order_id = order_id; s.page = 0; });
        });
    });

    Effect::new(move || {
        let ft = filter_type.get();
        untrack(move || {
            state.update(|s| {
                s.filter_type = if ft.is_empty() { None } else { Some(ft.clone()) };
                s.page = 0;
            });
        });
    });

    let load_data = move || {
        let current_state = state.get_untracked();
        set_loading.set(true);
        set_error.set(None);
        spawn_local(async move {
            let offset = current_state.page * current_state.page_size;
            let sort_desc = !current_state.sort_ascending;
            let mut url = format!(
                "{}/api/a016/ym-returns?limit={}&offset={}&sort_by={}&sort_desc={}&date_from={}&date_to={}",
                api_base(),
                current_state.page_size,
                offset,
                current_state.sort_field,
                sort_desc,
                current_state.date_from,
                current_state.date_to
            );
            if let Some(ref t) = current_state.filter_type {
                url.push_str(&format!("&return_type={}", t));
            }
            if !current_state.search_return_id.is_empty() {
                url.push_str(&format!("&search_return_id={}", current_state.search_return_id));
            }
            if !current_state.search_order_id.is_empty() {
                url.push_str(&format!("&search_order_id={}", current_state.search_order_id));
            }
            match Request::get(&url).send().await {
                Ok(response) => {
                    if response.status() == 200 {
                        match response.json::<PaginatedResponse>().await {
                            Ok(data) => {
                                set_items.set(data.items);
                                state.update(|s| {
                                    s.total_count = data.total;
                                    s.total_pages = data.total_pages;
                                    s.server_totals = data.totals;
                                    s.is_loaded = true;
                                });
                            }
                            Err(e) => {
                                log!("Failed to parse response: {:?}", e);
                                set_error.set(Some(format!("Ошибка парсинга: {}", e)));
                            }
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
            load_data();
        }
    });

    let filter_type_first_run = StoredValue::new(true);
    Effect::new(move || {
        let _ = filter_type.get();
        if !filter_type_first_run.get_value() {
            load_data();
        } else {
            filter_type_first_run.set_value(false);
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

    let active_filters_count = Signal::derive(move || {
        let s = state.get();
        let mut count = 0;
        if !s.date_from.is_empty() { count += 1; }
        if !s.date_to.is_empty() { count += 1; }
        if !s.search_return_id.is_empty() { count += 1; }
        if !s.search_order_id.is_empty() { count += 1; }
        if s.filter_type.is_some() { count += 1; }
        count
    });

    let toggle_sort = move |field: &'static str| {
        move |_| {
            if was_just_resizing() { return; }
            state.update(|s| {
                if s.sort_field == field { s.sort_ascending = !s.sort_ascending; }
                else { s.sort_field = field.to_string(); s.sort_ascending = true; }
                s.page = 0;
            });
            load_data();
        }
    };

    let go_to_page = move |page: usize| {
        state.update(|s| s.page = page);
        load_data();
    };

    let change_page_size = move |size: usize| {
        state.update(|s| { s.page_size = size; s.page = 0; });
        load_data();
    };

    let clear_all_filters = move |_| {
        state.update(|s| {
            s.date_from = String::new();
            s.date_to = String::new();
            s.search_return_id = String::new();
            s.search_order_id = String::new();
            s.filter_type = None;
            s.page = 0;
        });
        search_return_id.set(String::new());
        search_order_id.set(String::new());
        filter_type.set(String::new());
        load_data();
    };

    let batch_post = move |_| {
        let ids: Vec<String> = state.with_untracked(|s| s.selected_ids.iter().cloned().collect());
        if ids.is_empty() { return; }
        set_posting_in_progress.set(true);
        spawn_local(async move {
            let body = json!({ "ids": ids });
            match Request::post(&format!("{}/api/a016/ym-returns/batch-post", api_base()))
                .header("Content-Type", "application/json")
                .body(body.to_string()).unwrap().send().await
            {
                Ok(resp) => {
                    if resp.status() == 200 {
                        state.update(|s| s.selected_ids.clear());
                        load_data();
                    }
                }
                Err(e) => log!("Batch post error: {:?}", e),
            }
            set_posting_in_progress.set(false);
        });
    };

    let batch_unpost = move |_| {
        let ids: Vec<String> = state.with_untracked(|s| s.selected_ids.iter().cloned().collect());
        if ids.is_empty() { return; }
        set_posting_in_progress.set(true);
        spawn_local(async move {
            let body = json!({ "ids": ids });
            match Request::post(&format!("{}/api/a016/ym-returns/batch-unpost", api_base()))
                .header("Content-Type", "application/json")
                .body(body.to_string()).unwrap().send().await
            {
                Ok(resp) => {
                    if resp.status() == 200 {
                        state.update(|s| s.selected_ids.clear());
                        load_data();
                    }
                }
                Err(e) => log!("Batch unpost error: {:?}", e),
            }
            set_posting_in_progress.set(false);
        });
    };

    let export_excel = move |_| {
        let data = items.get();
        let mut csv = String::from("\u{FEFF}");
        csv.push_str("Return ID;Order ID;Тип;Статус;Кол-во;Сумма;Дата;Проведен\n");
        for item in data.iter() {
            csv.push_str(&format!(
                "{};{};{};{};{};{};{};{}\n",
                item.return_id, item.order_id, item.return_type, item.refund_status,
                item.total_items, format_number(item.total_amount),
                format_datetime(&item.created_at_source),
                if item.is_posted { "Да" } else { "Нет" }
            ));
        }
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                let parts = js_sys::Array::new();
                parts.push(&wasm_bindgen::JsValue::from_str(&csv));
                let opts = BlobPropertyBag::new();
                opts.set_type("text/csv;charset=utf-8");
                if let Ok(blob) = Blob::new_with_str_sequence_and_options(&parts, &opts) {
                    if let Ok(url_str) = Url::create_object_url_with_blob(&blob) {
                        if let Ok(a) = document.create_element("a") {
                            if let Ok(anchor) = a.dyn_into::<HtmlAnchorElement>() {
                                anchor.set_href(&url_str);
                                anchor.set_download("ym_returns.csv");
                                anchor.click();
                                let _ = Url::revoke_object_url(&url_str);
                            }
                        }
                    }
                }
            }
        }
    };

    let open_detail = move |id: String, return_id: i64| {
        use crate::layout::tabs::{detail_tab_label, pick_identifier};
        let return_id_str = return_id.to_string();
        let identifier = pick_identifier(Some(&return_id_str), None, None, &id);
        global_ctx.open_tab(
            &format!("a016_ym_returns_detail_{}", id),
            &detail_tab_label("YM Возврат", identifier),
        );
    };

    let selected = RwSignal::new(state.with_untracked(|s| s.selected_ids.clone()));

    let toggle_selection = move |id: String, checked: bool| {
        selected.update(|s| { if checked { s.insert(id.clone()); } else { s.remove(&id); } });
        state.update(|s| { if checked { s.selected_ids.insert(id); } else { s.selected_ids.remove(&id); } });
    };

    let toggle_all = move |check_all: bool| {
        let current_items = items.get();
        if check_all {
            selected.update(|s| { s.clear(); for item in current_items.iter() { s.insert(item.id.clone()); } });
            state.update(|s| { s.selected_ids.clear(); for item in current_items.iter() { s.selected_ids.insert(item.id.clone()); } });
        } else {
            selected.update(|s| s.clear());
            state.update(|s| s.selected_ids.clear());
        }
    };

    let items_signal = Signal::derive(move || items.get());
    let selected_signal = Signal::derive(move || selected.get());
    let selected_count = Signal::derive(move || selected.get().len());

    view! {
        <PageFrame page_id="a016_ym_returns--list" category="list">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Возвраты Яндекс Маркет"</h1>
                    <UiBadge variant="primary".to_string()>
                        {move || state.get().total_count.to_string()}
                    </UiBadge>
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Subtle
                        on_click=batch_post
                        disabled=Signal::derive(move || selected_count.get() == 0 || posting_in_progress.get())
                    >
                        {icon("check")}
                        {move || format!(" Провести ({})", selected_count.get())}
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Subtle
                        on_click=batch_unpost
                        disabled=Signal::derive(move || selected_count.get() == 0 || posting_in_progress.get())
                    >
                        {icon("x")}
                        {move || format!(" Отменить ({})", selected_count.get())}
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=export_excel
                    >
                        {icon("download")}
                        " Excel"
                    </Button>
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
                                page_size_options=vec![50, 100, 200, 500]
                            />
                        </div>

                        <div class="filter-panel-header__right">
                            <Button
                                appearance=ButtonAppearance::Primary
                                on_click=move |_| load_data()
                                disabled=Signal::derive(move || loading.get())
                            >
                                {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                            </Button>
                        </div>
                    </div>

                    <Show when=move || is_filter_expanded.get()>
                        <div class="filter-panel-content">
                            <Flex gap=FlexGap::Small align=FlexAlign::End>
                                <div style="min-width: 400px;">
                                    <DateRangePicker
                                        date_from=Signal::derive(move || state.get().date_from)
                                        date_to=Signal::derive(move || state.get().date_to)
                                        on_change=Callback::new(move |(from, to)| {
                                            state.update(|s| { s.date_from = from; s.date_to = to; s.page = 0; });
                                            load_data();
                                        })
                                        label="Период:".to_string()
                                    />
                                </div>

                                <div style="width: 150px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Return ID:"</Label>
                                        <Input value=search_return_id placeholder="Поиск..." />
                                    </Flex>
                                </div>

                                <div style="width: 150px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Order ID:"</Label>
                                        <Input value=search_order_id placeholder="Поиск..." />
                                    </Flex>
                                </div>

                                <div style="width: 150px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Тип:"</Label>
                                        <Select value=filter_type>
                                            <option value="">"Все"</option>
                                            <option value="RETURN">"Возврат"</option>
                                            <option value="UNREDEEMED">"Невыкуп"</option>
                                        </Select>
                                    </Flex>
                                </div>

                                {move || {
                                    if active_filters_count.get() > 0 {
                                        view! {
                                            <Button
                                                appearance=ButtonAppearance::Subtle
                                                on_click=clear_all_filters
                                            >
                                                "Сбросить всё"
                                            </Button>
                                        }.into_any()
                                    } else {
                                        view! { <></> }.into_any()
                                    }
                                }}
                            </Flex>
                        </div>
                    </Show>
                </div>

                {move || state.get().server_totals.map(|totals| view! {
                    <div class="filter-panel-totals">
                        <span>"Записей: " {totals.total_records} " | "</span>
                        <span>"Возвратов: " {totals.returns_count} " | "</span>
                        <span>"Невыкупов: " {totals.unredeemed_count} " | "</span>
                        <span>"Товаров: " {totals.sum_items} " | "</span>
                        <span>"Сумма: " {format_number(totals.sum_amount)}</span>
                    </div>
                })}

                {move || error.get().map(|err| view! { <div class="alert alert--error">{err}</div> })}

                <div class="table-wrapper">
                    <TableCrosshairHighlight table_id=TABLE_ID.to_string() />

                    <Table attr:id=TABLE_ID attr:style="width: 100%; min-width: 800px;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCheckbox
                                    items=items_signal
                                    selected=selected_signal
                                    get_id=Callback::new(|row: YmReturnDto| row.id.clone())
                                    on_change=Callback::new(toggle_all)
                                />

                                <TableHeaderCell resizable=false min_width=130.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=toggle_sort("created_at_source")>
                                        "Дата"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "created_at_source"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "created_at_source", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=toggle_sort("return_id")>
                                        "Return №"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "return_id"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "return_id", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=toggle_sort("order_id")>
                                        "Order №"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "order_id"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "order_id", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=100.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=toggle_sort("return_type")>
                                        "Тип"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "return_type"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "return_type", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=120.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=toggle_sort("refund_status")>
                                        "Статус"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "refund_status"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "refund_status", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=70.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=toggle_sort("total_items")>
                                        "Шт."
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "total_items"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "total_items", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=toggle_sort("total_amount")>
                                        "Сумма"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "total_amount"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "total_amount", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=80.0 class="resizable">
                                    "Провед."
                                </TableHeaderCell>
                            </TableRow>
                        </TableHeader>

                        <TableBody>
                            <For
                                each=move || items.get()
                                key=|item| item.id.clone()
                                children=move |item| {
                                    let item_id = item.id.clone();
                                    let item_id_click = item.id.clone();
                                    let return_id = item.return_id;
                                    let is_posted = item.is_posted;

                                    let return_type_badge = match item.return_type.as_str() {
                                        "UNREDEEMED" => "badge badge--warning",
                                        "RETURN" => "badge badge--info",
                                        _ => "badge badge--neutral",
                                    };
                                    let return_type_label = match item.return_type.as_str() {
                                        "UNREDEEMED" => "Невыкуп".to_string(),
                                        "RETURN" => "Возврат".to_string(),
                                        _ => item.return_type.clone(),
                                    };
                                    let status_badge = match item.refund_status.as_str() {
                                        "REFUNDED" => "badge badge--success",
                                        "NOT_REFUNDED" => "badge badge--error",
                                        "REFUND_IN_PROGRESS" => "badge badge--warning",
                                        _ => "badge badge--neutral",
                                    };
                                    let refund_status = item.refund_status.clone();

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
                                                            open_detail(item_id_click.clone(), return_id);
                                                        }
                                                    >
                                                        {format_datetime(&item.created_at_source)}
                                                    </a>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    <span style="font-weight: 600; font-variant-numeric: tabular-nums;">{return_id}</span>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    <span style="font-variant-numeric: tabular-nums;">{item.order_id}</span>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    <span class=return_type_badge>{return_type_label}</span>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    <span class=status_badge>{refund_status}</span>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    <span style="font-variant-numeric: tabular-nums;">{item.total_items}</span>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCellMoney
                                                value=Signal::derive(move || Some(item.total_amount))
                                                show_currency=false
                                                color_by_sign=false
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
