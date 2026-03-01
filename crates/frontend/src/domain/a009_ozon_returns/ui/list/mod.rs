pub mod state;

use super::details::OzonReturnsDetail;
use self::state::create_state;
use crate::shared::api_utils::api_base;
use crate::shared::components::date_range_picker::DateRangePicker;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::table::{
    TableCellCheckbox, TableCellMoney, TableCrosshairHighlight, TableHeaderCheckbox,
};
use crate::shared::components::ui::badge::Badge as UiBadge;
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator, Sortable};
use crate::shared::modal_stack::ModalStackService;
use crate::shared::page_frame::PageFrame;
use crate::shared::table_utils::init_column_resize;
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use thaw::*;

/// Форматирует ISO 8601 дату в dd.mm.yyyy
fn format_date(iso_date: &str) -> String {
    if let Some((year, rest)) = iso_date.split_once('-') {
        if let Some((month, day)) = rest.split_once('-') {
            return format!("{}.{}.{}", day, month, year);
        }
    }
    iso_date.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OzonReturnsDto {
    pub id: String,
    #[serde(rename = "returnId")]
    pub return_id: String,
    #[serde(rename = "returnDate")]
    pub return_date: String,
    #[serde(rename = "returnType")]
    pub return_type: String,
    #[serde(rename = "returnReasonName")]
    pub return_reason_name: String,
    #[serde(rename = "orderNumber")]
    pub order_number: String,
    #[serde(rename = "postingNumber")]
    pub posting_number: String,
    pub sku: String,
    #[serde(rename = "productName")]
    pub product_name: String,
    pub quantity: i32,
    pub price: f64,
    #[serde(rename = "isPosted")]
    pub is_posted: bool,
}

impl Sortable for OzonReturnsDto {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "return_id" => self
                .return_id
                .to_lowercase()
                .cmp(&other.return_id.to_lowercase()),
            "return_date" => self.return_date.cmp(&other.return_date),
            "return_type" => self
                .return_type
                .to_lowercase()
                .cmp(&other.return_type.to_lowercase()),
            "return_reason" => self
                .return_reason_name
                .to_lowercase()
                .cmp(&other.return_reason_name.to_lowercase()),
            "order_number" => self
                .order_number
                .to_lowercase()
                .cmp(&other.order_number.to_lowercase()),
            "posting_number" => self
                .posting_number
                .to_lowercase()
                .cmp(&other.posting_number.to_lowercase()),
            "sku" => self.sku.to_lowercase().cmp(&other.sku.to_lowercase()),
            "product_name" => self
                .product_name
                .to_lowercase()
                .cmp(&other.product_name.to_lowercase()),
            "quantity" => self.quantity.cmp(&other.quantity),
            "price" => self
                .price
                .partial_cmp(&other.price)
                .unwrap_or(Ordering::Equal),
            "is_posted" => self.is_posted.cmp(&other.is_posted),
            _ => Ordering::Equal,
        }
    }
}

const TABLE_ID: &str = "a009-ozon-returns-table";
const COLUMN_WIDTHS_KEY: &str = "a009_ozon_returns_column_widths";

#[component]
pub fn OzonReturnsList() -> impl IntoView {
    let modal_stack =
        use_context::<ModalStackService>().expect("ModalStackService not found in context");
    let state = create_state();
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (is_filter_expanded, set_is_filter_expanded) = signal(false);
    let (posting_in_progress, set_posting_in_progress) = signal(false);
    let (current_operation, set_current_operation) = signal::<Option<(usize, usize)>>(None);
    let (detail_reload_trigger, set_detail_reload_trigger) = signal::<u32>(0);

    // All loaded rows (unfiltered source of truth for client-side filtering)
    let all_rows: RwSignal<Vec<OzonReturnsDto>> = RwSignal::new(Vec::new());

    let refresh_view = move || {
        let source = all_rows.get_untracked();
        let date_from = state.with_untracked(|s| s.date_from.clone());
        let date_to = state.with_untracked(|s| s.date_to.clone());
        let field = state.with_untracked(|s| s.sort_field.clone());
        let ascending = state.with_untracked(|s| s.sort_ascending);
        let page_size = state.with_untracked(|s| s.page_size);
        let page = state.with_untracked(|s| s.page);

        let mut filtered: Vec<OzonReturnsDto> = source
            .into_iter()
            .filter(|item| {
                if !date_from.is_empty() && item.return_date < date_from {
                    return false;
                }
                if !date_to.is_empty() && item.return_date > date_to {
                    return false;
                }
                true
            })
            .collect();

        filtered.sort_by(|a, b| {
            let cmp = a.compare_by_field(b, &field);
            if ascending { cmp } else { cmp.reverse() }
        });

        let total = filtered.len();
        let total_pages = if total == 0 { 0 } else { (total + page_size - 1) / page_size };
        let page = page.min(if total_pages == 0 { 0 } else { total_pages - 1 });
        let start = page * page_size;
        let end = (start + page_size).min(total);
        let page_items = if start < total { filtered[start..end].to_vec() } else { vec![] };

        state.update(|s| {
            s.items = page_items;
            s.total_count = total;
            s.total_pages = total_pages;
            s.page = page;
        });
    };

    let load_returns = move || {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            let url = format!("{}/api/ozon_returns", api_base());
            match Request::get(&url).send().await {
                Ok(response) => {
                    if response.status() == 200 {
                        match response.json::<Vec<OzonReturnsDto>>().await {
                            Ok(items) => {
                                log!("Loaded {} OZON returns", items.len());
                                all_rows.set(items);
                                state.update(|s| { s.page = 0; s.is_loaded = true; });
                                refresh_view();
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
            load_returns();
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

    let open_detail_modal = move |id: String| {
        let reload = detail_reload_trigger;
        modal_stack.push_with_frame(
            Some("max-width: min(1200px, 95vw); width: min(1200px, 95vw); height: calc(100vh - 80px); overflow: hidden;".to_string()),
            Some("ozon-returns-detail-modal".to_string()),
            move |handle| {
                view! {
                    <OzonReturnsDetail
                        id=id.clone()
                        on_close=Callback::new({
                            let handle = handle.clone();
                            move |_| handle.close()
                        })
                        reload_trigger=reload
                    />
                }
                .into_any()
            },
        );
    };

    let post_batch = move |post: bool| {
        let ids: Vec<String> = state.with_untracked(|s| s.selected_ids.iter().cloned().collect());
        if ids.is_empty() {
            return;
        }
        let total = ids.len();
        set_posting_in_progress.set(true);
        set_current_operation.set(Some((0, total)));
        spawn_local(async move {
            let action = if post { "post" } else { "unpost" };
            for (index, id) in ids.iter().enumerate() {
                set_current_operation.set(Some((index + 1, total)));
                let url = format!("{}/api/a009/ozon-returns/{}/{}", api_base(), id, action);
                let _ = Request::post(&url).send().await;
            }
            set_posting_in_progress.set(false);
            set_current_operation.set(None);
            state.update(|s| s.selected_ids.clear());
            set_detail_reload_trigger.update(|v| *v += 1);
            // Reload the list
            load_returns();
        });
    };

    let active_filters_count = Signal::derive(move || {
        let s = state.get();
        let mut count = 0;
        if !s.date_from.is_empty() { count += 1; }
        if !s.date_to.is_empty() { count += 1; }
        count
    });

    let toggle_sort = move |field: &'static str| {
        state.update(|s| {
            if s.sort_field == field {
                s.sort_ascending = !s.sort_ascending;
            } else {
                s.sort_field = field.to_string();
                s.sort_ascending = true;
            }
            s.page = 0;
        });
        refresh_view();
    };

    let go_to_page = move |new_page: usize| {
        state.update(|s| s.page = new_page);
        refresh_view();
    };

    let change_page_size = move |new_size: usize| {
        state.update(|s| {
            s.page_size = new_size;
            s.page = 0;
        });
        refresh_view();
    };

    let selected = RwSignal::new(state.with_untracked(|s| s.selected_ids.clone()));

    let toggle_selection = move |id: String, checked: bool| {
        selected.update(|s| {
            if checked { s.insert(id.clone()); } else { s.remove(&id); }
        });
        state.update(|s| {
            if checked { s.selected_ids.insert(id); } else { s.selected_ids.remove(&id); }
        });
    };

    let toggle_all = move |check_all: bool| {
        let items = state.get().items;
        if check_all {
            selected.update(|s| {
                s.clear();
                for item in items.iter() { s.insert(item.id.clone()); }
            });
            state.update(|s| {
                s.selected_ids.clear();
                for item in items.iter() { s.selected_ids.insert(item.id.clone()); }
            });
        } else {
            selected.update(|s| s.clear());
            state.update(|s| s.selected_ids.clear());
        }
    };

    let items_signal = Signal::derive(move || state.get().items);
    let selected_signal = Signal::derive(move || selected.get());
    let selected_count = Signal::derive(move || selected.get().len());

    view! {
        <PageFrame page_id="a009_ozon_returns--list" category="list">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Возвраты OZON"</h1>
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
                    {move || {
                        current_operation.get().map(|(cur, total)| view! {
                            <span class="page__status">{format!("Обработка {}/{} ...", cur, total)}</span>
                        })
                    }}
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
                                on_click=move |_| load_returns()
                                disabled=Signal::derive(move || loading.get())
                            >
                                {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                            </Button>
                        </div>
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
                                            refresh_view();
                                        })
                                        label="Период:".to_string()
                                    />
                                </div>
                            </Flex>
                        </div>
                    </Show>
                </div>

                {move || {
                    error.get().map(|err| view! {
                        <div class="alert alert--error">{err}</div>
                    })
                }}

                <div class="table-wrapper">
                    <TableCrosshairHighlight table_id=TABLE_ID.to_string() />

                    <Table attr:id=TABLE_ID attr:style="width: 100%; min-width: 1000px;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCheckbox
                                    items=items_signal
                                    selected=selected_signal
                                    get_id=Callback::new(|row: OzonReturnsDto| row.id.clone())
                                    on_change=Callback::new(toggle_all)
                                />

                                <TableHeaderCell resizable=false min_width=130.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("return_date")>
                                        "Дата"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "return_date"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "return_date", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=120.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("return_id")>
                                        "Return ID"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "return_id"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "return_id", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=170.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("posting_number")>
                                        "Номер постинга"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "posting_number"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "posting_number", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("return_type")>
                                        "Тип"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "return_type"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "return_type", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=200.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("product_name")>
                                        "Товар"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "product_name"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "product_name", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=80.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("quantity")>
                                        "Кол-во"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "quantity"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "quantity", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("price")>
                                        "Цена"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "price"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "price", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=170.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("return_reason")>
                                        "Причина"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "return_reason"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "return_reason", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=100.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("is_posted")>
                                        "Статус"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "is_posted"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "is_posted", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                            </TableRow>
                        </TableHeader>

                        <TableBody>
                            <For
                                each=move || state.get().items
                                key=|item| item.id.clone()
                                children=move |item| {
                                    let item_id = item.id.clone();
                                    let item_id_for_click = item.id.clone();
                                    let formatted_date = format_date(&item.return_date);
                                    let is_posted = item.is_posted;
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
                                                            open_detail_modal(item_id_for_click.clone());
                                                        }
                                                    >
                                                        {formatted_date}
                                                    </a>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>{item.return_id.clone()}</TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {item.posting_number.clone()}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>{item.return_type.clone()}</TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {item.product_name.clone()}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    <span style="font-variant-numeric: tabular-nums;">{item.quantity}</span>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCellMoney
                                                value=Signal::derive(move || Some(item.price))
                                                show_currency=false
                                                color_by_sign=false
                                            />
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {item.return_reason_name.clone()}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    {if is_posted {
                                                        view! { <span class="badge badge--success">"Проведен"</span> }.into_any()
                                                    } else {
                                                        view! { <span class="badge badge--neutral">"Не проведен"</span> }.into_any()
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
