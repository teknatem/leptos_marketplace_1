pub mod state;

use self::state::create_state;
use chrono::Utc;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use thaw::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

// Импорты компонентов деталей документов
use crate::domain::a009_ozon_returns::ui::details::OzonReturnsDetail;
use crate::domain::a010_ozon_fbs_posting::ui::details::OzonFbsPostingDetail;
use crate::domain::a012_wb_sales::ui::details::WbSalesDetail;
use crate::domain::a013_ym_order::ui::details::YmOrderDetail;
use crate::shared::components::date_range_picker::DateRangePicker;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::ui::badge::Badge as UiBadge;
use crate::shared::components::ui::button::Button as UiButton;
use crate::shared::icons::icon;
use crate::shared::list_utils::{
    format_number, get_sort_class, get_sort_indicator, sort_list, Sortable,
};
use crate::shared::table_utils::{clear_resize_flag, init_column_resize, was_just_resizing};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesRegisterDto {
    pub marketplace: String,
    pub document_no: String,
    pub line_id: String,
    pub scheme: Option<String>,
    pub document_type: String,
    pub document_version: i32,
    pub connection_mp_ref: String,
    pub organization_ref: String,
    pub marketplace_product_ref: Option<String>,
    pub nomenclature_ref: Option<String>,
    pub registrator_ref: String,
    pub event_time_source: String,
    pub sale_date: String,
    pub source_updated_at: Option<String>,
    pub status_source: String,
    pub status_norm: String,
    pub seller_sku: Option<String>,
    pub mp_item_id: String,
    pub barcode: Option<String>,
    pub title: Option<String>,
    pub qty: f64,
    pub price_list: Option<f64>,
    pub discount_total: Option<f64>,
    pub price_effective: Option<f64>,
    pub amount_line: Option<f64>,
    /// Плановая себестоимость (из p906_nomenclature_prices)
    pub cost: Option<f64>,
    pub currency_code: Option<String>,
    pub loaded_at_utc: String,
    pub payload_version: i32,
    pub extra: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesRegisterListResponse {
    pub items: Vec<SalesRegisterDto>,
    pub total_count: i32,
    pub has_more: bool,
}

#[derive(Debug, Clone)]
struct SelectedDocument {
    document_type: String,
    document_id: String,
}

impl Sortable for SalesRegisterDto {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "sale_date" => self.sale_date.cmp(&other.sale_date),
            "marketplace" => self
                .marketplace
                .to_lowercase()
                .cmp(&other.marketplace.to_lowercase()),
            "document_no" => self
                .document_no
                .to_lowercase()
                .cmp(&other.document_no.to_lowercase()),
            "title" => self
                .title
                .as_deref()
                .unwrap_or("")
                .to_lowercase()
                .cmp(&other.title.as_deref().unwrap_or("").to_lowercase()),
            "seller_sku" => self
                .seller_sku
                .as_deref()
                .unwrap_or("")
                .to_lowercase()
                .cmp(&other.seller_sku.as_deref().unwrap_or("").to_lowercase()),
            "qty" => self.qty.partial_cmp(&other.qty).unwrap_or(Ordering::Equal),
            "amount_line" => match (&self.amount_line, &other.amount_line) {
                (Some(a), Some(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            "cost" => {
                let self_cost = self.cost.map(|c| c * self.qty).unwrap_or(0.0);
                let other_cost = other.cost.map(|c| c * other.qty).unwrap_or(0.0);
                self_cost
                    .partial_cmp(&other_cost)
                    .unwrap_or(Ordering::Equal)
            }
            "profit" => {
                let self_profit = self
                    .cost
                    .map(|c| self.amount_line.unwrap_or(0.0) - c * self.qty)
                    .unwrap_or(0.0);
                let other_profit = other
                    .cost
                    .map(|c| other.amount_line.unwrap_or(0.0) - c * other.qty)
                    .unwrap_or(0.0);
                self_profit
                    .partial_cmp(&other_profit)
                    .unwrap_or(Ordering::Equal)
            }
            "status_norm" => self
                .status_norm
                .to_lowercase()
                .cmp(&other.status_norm.to_lowercase()),
            "organization_ref" => self
                .organization_ref
                .to_lowercase()
                .cmp(&other.organization_ref.to_lowercase()),
            _ => Ordering::Equal,
        }
    }
}

#[component]
pub fn SalesRegisterList() -> impl IntoView {
    let state = create_state();
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(None::<String>);
    let (selected_document, set_selected_document) = signal::<Option<SelectedDocument>>(None);

    // Filter panel expansion state
    let (is_filter_expanded, set_is_filter_expanded) = signal(false);

    const TABLE_ID: &str = "p900-sales-register-table";
    const COLUMN_WIDTHS_KEY: &str = "p900_sales_register_column_widths";

    let get_items = move || -> Vec<SalesRegisterDto> {
        let mut items = state.with(|s| s.sales.clone());
        let sort_field = state.with(|s| s.sort_field.clone());
        let sort_ascending = state.with(|s| s.sort_ascending);
        sort_list(&mut items, &sort_field, sort_ascending);
        items
    };

    // Вычисление итогов по Qty, Amount, Cost и Profit
    let totals = move || {
        let data = get_items();
        let total_qty: f64 = data.iter().map(|s| s.qty).sum();
        let total_amount: f64 = data.iter().map(|s| s.amount_line.unwrap_or(0.0)).sum();
        let total_cost: f64 = data.iter().filter_map(|s| s.cost.map(|c| c * s.qty)).sum();
        let total_profit: f64 = data
            .iter()
            .filter_map(|s| s.cost.map(|c| s.amount_line.unwrap_or(0.0) - c * s.qty))
            .sum();
        (total_qty, total_amount, total_cost, total_profit)
    };

    let load_sales = move || {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let (date_from, date_to, marketplace, page, page_size) = state.with(|s| {
                (
                    s.date_from.clone(),
                    s.date_to.clone(),
                    s.marketplace.clone(),
                    s.page,
                    s.page_size,
                )
            });
            let offset = (page * page_size) as i32;

            let mut query_params = format!(
                "?date_from={}&date_to={}&limit={}&offset={}",
                date_from, date_to, page_size, offset
            );

            if !marketplace.is_empty() {
                query_params.push_str(&format!("&marketplace={}", marketplace));
            }

            match fetch_sales(&query_params).await {
                Ok(data) => {
                    let total_count = data.total_count.max(0) as usize;
                    let total_pages = if total_count == 0 {
                        0
                    } else {
                        (total_count + page_size - 1) / page_size
                    };
                    state.update(|s| {
                        s.sales = data.items;
                        s.total_count = total_count;
                        s.total_pages = total_pages;
                        s.is_loaded = true;
                    });
                    set_loading.set(false);
                }
                Err(e) => {
                    log!("Failed to fetch sales: {:?}", e);
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    };

    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            load_sales();
        }
    });

    // Thaw inputs: keep local RwSignal, sync -> state (one-way)
    let marketplace_value = RwSignal::new(state.get_untracked().marketplace.clone());
    Effect::new(move || {
        let v = marketplace_value.get();
        untrack(move || {
            state.update(|s| {
                s.marketplace = v;
                s.page = 0;
            });
        });
    });

    let active_filters_count = Signal::derive(move || {
        let s = state.get();
        let mut count = 0;
        if !s.date_from.is_empty() {
            count += 1;
        }
        if !s.date_to.is_empty() {
            count += 1;
        }
        if !s.marketplace.is_empty() {
            count += 1;
        }
        count
    });

    Effect::new(move |_| {
        let is_loaded = state.get().is_loaded;
        let _page = state.get().page;
        let _len = state.get().sales.len();
        if is_loaded {
            spawn_local(async move {
                gloo_timers::future::TimeoutFuture::new(50).await;
                init_column_resize(TABLE_ID, COLUMN_WIDTHS_KEY);
            });
        }
    });

    // Функция для изменения сортировки
    let toggle_sort = move |field: &'static str| {
        if was_just_resizing() {
            clear_resize_flag();
            return;
        }
        state.update(|s| {
            if s.sort_field == field {
                s.sort_ascending = !s.sort_ascending;
            } else {
                s.sort_field = field.to_string();
                s.sort_ascending = true;
            }
        });
    };

    // Pagination: go to specific page
    let go_to_page = move |new_page: usize| {
        state.update(|s| s.page = new_page);
        load_sales();
    };

    // Pagination: change page size
    let change_page_size = move |new_size: usize| {
        state.update(|s| {
            s.page_size = new_size;
            s.page = 0;
        });
        load_sales();
    };

    view! {
        <div class="page page--wide">
            <div class="page-header">
                <div class="page-header__content">
                    <div class="page-header__icon">{icon("trending-up")}</div>
                    <div class="page-header__text">
                        <h1 class="page-header__title">"Регистр продаж (P900)"</h1>
                        <div class="page-header__badge">
                            <UiBadge variant="primary".to_string()>
                                {move || state.get().total_count.to_string()}
                            </UiBadge>
                        </div>
                    </div>
                </div>

                <div class="page-header__actions">
                    <Space>
                        <UiButton
                            variant="secondary".to_string()
                            on_click=Callback::new(move |_| {
                                let data = get_items();
                                if let Err(e) = export_to_csv(&data) {
                                    log!("Failed to export: {}", e);
                                }
                            })
                            disabled=loading.get() || state.get().sales.is_empty()
                        >
                            {icon("download")}
                            "Excel"
                        </UiButton>
                        {move || {
                            if !loading.get() && !state.get().sales.is_empty() {
                                let (total_qty, total_amount, total_cost, total_profit) = totals();
                                view! {
                                    <span style="font-size: 12px; color: var(--colorNeutralForeground2, #666);">
                                        "Qty: " {format_number(total_qty)} " | "
                                        "Amount: " {format_number(total_amount)} " | "
                                        "Cost: " {format_number(total_cost)} " | "
                                        "Profit: " {format_number(total_profit)}
                                    </span>
                                }.into_any()
                            } else {
                                view! { <></> }.into_any()
                            }
                        }}
                    </Space>
                </div>
            </div>

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
                        {icon("filter")}
                        <span class="filter-panel__title">"Фильтры"</span>
                        {move || {
                            let count = active_filters_count.get();
                            if count > 0 {
                                view! {
                                    <UiBadge variant="primary".to_string()>{count}</UiBadge>
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
                            page_size_options=vec![50, 100, 200, 500, 10000]
                        />
                    </div>

                    <div class="filter-panel-header__right">
                        <thaw::Button
                            appearance=ButtonAppearance::Subtle
                            on_click=move |_| load_sales()
                            disabled=loading.get()
                        >
                            {icon("refresh")}
                            {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                        </thaw::Button>
                    </div>
                </div>

                <div class=move || {
                    if is_filter_expanded.get() {
                        "filter-panel__collapsible filter-panel__collapsible--expanded"
                    } else {
                        "filter-panel__collapsible filter-panel__collapsible--collapsed"
                    }
                }>
                    <div class="filter-panel-content">
                        <Flex gap=FlexGap::Small align=FlexAlign::End>
                            <div style="min-width: 420px;">
                                <DateRangePicker
                                    date_from=Signal::derive(move || state.get().date_from)
                                    date_to=Signal::derive(move || state.get().date_to)
                                    on_change=Callback::new(move |(from, to)| {
                                        state.update(|s| {
                                            s.date_from = from;
                                            s.date_to = to;
                                            s.page = 0;
                                        });
                                        load_sales();
                                    })
                                    label="Период:".to_string()
                                />
                            </div>

                            <div style="width: 220px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Маркетплейс:"</Label>
                                    <Select value=marketplace_value>
                                        <option value="">"Все"</option>
                                        <option value="OZON">"OZON"</option>
                                        <option value="WB">"Wildberries"</option>
                                        <option value="YM">"Yandex Market"</option>
                                    </Select>
                                </Flex>
                            </div>
                        </Flex>
                    </div>
                </div>
            </div>

            // Error message
            {move || {
                if let Some(err) = error.get() {
                    view! {
                        <div class="warning-box" style="background: var(--color-error-50); border-color: var(--color-error-100); margin: 0 var(--spacing-sm) var(--spacing-xs) var(--spacing-sm);">
                            <span class="warning-box__icon" style="color: var(--color-error);">"⚠"</span>
                            <span class="warning-box__text" style="color: var(--color-error);">{err}</span>
                        </div>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}

            <div class="page-content">
                <div class="list-container">
                    {move || {
                        if loading.get() {
                            return view! {
                                <div class="loading-spinner" style="text-align: center; padding: 40px;">
                                    "Загрузка продаж..."
                                </div>
                            }.into_any();
                        }

                        let items = get_items();

                        view! {
                            // Модальное окно для деталей документа
                            {move || {
                                if let Some(selected) = selected_document.get() {
                                    view! {
                                        <div class="modal-overlay" style="align-items: flex-start; padding-top: 40px;">
                                            <div class="modal modal-content-wide">
                                                {match selected.document_type.as_str() {
                                                    "OZON_FBS_Posting" => {
                                                        view! {
                                                            <OzonFbsPostingDetail
                                                                id=selected.document_id.clone()
                                                                on_close=move || set_selected_document.set(None)
                                                            />
                                                        }.into_any()
                                                    }
                                                    "OZON_FBO_Posting" => {
                                                        view! {
                                                            <div style="padding: 20px;">
                                                                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                                                                    <h2 style="margin: 0;">"OZON FBO Posting Details"</h2>
                                                                    <button
                                                                        on:click=move |_| set_selected_document.set(None)
                                                                        style="padding: 8px 16px; background: #f44336; color: white; border: none; border-radius: 4px; cursor: pointer;"
                                                                    >
                                                                        "✕ Close"
                                                                    </button>
                                                                </div>
                                                                <div style="padding: 20px; background: #fff3cd; border: 1px solid #ffc107; border-radius: 4px;">
                                                                    <p>"OZON FBO Posting details component not yet implemented."</p>
                                                                    <p style="font-family: monospace; font-size: 0.9em; margin-top: 10px;">
                                                                        "Document ID: " {selected.document_id.clone()}
                                                                    </p>
                                                                </div>
                                                            </div>
                                                        }.into_any()
                                                    }
                                                    "WB_Sales" => {
                                                        view! {
                                                            <WbSalesDetail
                                                                id=selected.document_id.clone()
                                                                on_close=move || set_selected_document.set(None)
                                                            />
                                                        }.into_any()
                                                    }
                                                    "YM_Order" => {
                                                        view! {
                                                            <YmOrderDetail
                                                                id=selected.document_id.clone()
                                                                on_close=move || set_selected_document.set(None)
                                                            />
                                                        }.into_any()
                                                    }
                                                    "OZON_Returns" => {
                                                        view! {
                                                            <OzonReturnsDetail
                                                                id=selected.document_id.clone()
                                                                on_close=move || set_selected_document.set(None)
                                                            />
                                                        }.into_any()
                                                    }
                                                    _ => {
                                                        view! {
                                                            <div style="padding: 20px;">
                                                                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                                                                    <h2 style="margin: 0;">"Unknown Document Type"</h2>
                                                                    <button
                                                                        on:click=move |_| set_selected_document.set(None)
                                                                        style="padding: 8px 16px; background: #f44336; color: white; border: none; border-radius: 4px; cursor: pointer;"
                                                                    >
                                                                        "✕ Close"
                                                                    </button>
                                                                </div>
                                                                <div style="padding: 20px; background: #ffebee; border: 1px solid #ef5350; border-radius: 4px; color: #c62828;">
                                                                    <p>"Unknown document type: " {selected.document_type.clone()}</p>
                                                                    <p style="font-family: monospace; font-size: 0.9em; margin-top: 10px;">
                                                                        "Document ID: " {selected.document_id.clone()}
                                                                    </p>
                                                                </div>
                                                            </div>
                                                        }.into_any()
                                                    }
                                                }}
                                            </div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <></> }.into_any()
                                }
                            }}

                            // Only horizontal scrolling here; vertical scrolling is handled by `.page`
                            <div class="table-container" style="overflow-x: auto; overflow-y: visible;">
                                <Table attr:id=TABLE_ID attr:style="width: 100%; min-width: 1500px;">
                                    <TableHeader>
                                        <TableRow>
                                            <TableHeaderCell resizable=false min_width=90.0 class="resizable">
                                                <div
                                                    class="table__sortable-header"
                                                    style="cursor: pointer;"
                                                    on:click=move |_| toggle_sort("sale_date")
                                                >
                                                    "Дата"
                                                    <span class=move || state.with(|s| get_sort_class(&s.sort_field, "sale_date"))>
                                                        {move || state.with(|s| get_sort_indicator(&s.sort_field, "sale_date", s.sort_ascending))}
                                                    </span>
                                                </div>
                                            </TableHeaderCell>
                                            <TableHeaderCell resizable=false min_width=120.0 class="resizable">
                                                <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("marketplace")>
                                                    "Маркетплейс"
                                                    <span class=move || state.with(|s| get_sort_class(&s.sort_field, "marketplace"))>
                                                        {move || state.with(|s| get_sort_indicator(&s.sort_field, "marketplace", s.sort_ascending))}
                                                    </span>
                                                </div>
                                            </TableHeaderCell>
                                            <TableHeaderCell resizable=false min_width=130.0 class="resizable">
                                                <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("document_no")>
                                                    "Документ №"
                                                    <span class=move || state.with(|s| get_sort_class(&s.sort_field, "document_no"))>
                                                        {move || state.with(|s| get_sort_indicator(&s.sort_field, "document_no", s.sort_ascending))}
                                                    </span>
                                                </div>
                                            </TableHeaderCell>
                                            <TableHeaderCell resizable=false min_width=220.0 class="resizable">
                                                <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("title")>
                                                    "Товар"
                                                    <span class=move || state.with(|s| get_sort_class(&s.sort_field, "title"))>
                                                        {move || state.with(|s| get_sort_indicator(&s.sort_field, "title", s.sort_ascending))}
                                                    </span>
                                                </div>
                                            </TableHeaderCell>
                                            <TableHeaderCell resizable=false min_width=120.0 class="resizable">
                                                <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("seller_sku")>
                                                    "SKU"
                                                    <span class=move || state.with(|s| get_sort_class(&s.sort_field, "seller_sku"))>
                                                        {move || state.with(|s| get_sort_indicator(&s.sort_field, "seller_sku", s.sort_ascending))}
                                                    </span>
                                                </div>
                                            </TableHeaderCell>
                                            <TableHeaderCell resizable=false min_width=70.0 class="resizable">
                                                <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("qty")>
                                                    "Кол-во"
                                                    <span class=move || state.with(|s| get_sort_class(&s.sort_field, "qty"))>
                                                        {move || state.with(|s| get_sort_indicator(&s.sort_field, "qty", s.sort_ascending))}
                                                    </span>
                                                </div>
                                            </TableHeaderCell>
                                            <TableHeaderCell resizable=false min_width=100.0 class="resizable">
                                                <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("amount_line")>
                                                    "Сумма"
                                                    <span class=move || state.with(|s| get_sort_class(&s.sort_field, "amount_line"))>
                                                        {move || state.with(|s| get_sort_indicator(&s.sort_field, "amount_line", s.sort_ascending))}
                                                    </span>
                                                </div>
                                            </TableHeaderCell>
                                            <TableHeaderCell resizable=false min_width=100.0 class="resizable">
                                                <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("cost")>
                                                    "Себест."
                                                    <span class=move || state.with(|s| get_sort_class(&s.sort_field, "cost"))>
                                                        {move || state.with(|s| get_sort_indicator(&s.sort_field, "cost", s.sort_ascending))}
                                                    </span>
                                                </div>
                                            </TableHeaderCell>
                                            <TableHeaderCell resizable=false min_width=100.0 class="resizable">
                                                <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("profit")>
                                                    "Прибыль"
                                                    <span class=move || state.with(|s| get_sort_class(&s.sort_field, "profit"))>
                                                        {move || state.with(|s| get_sort_indicator(&s.sort_field, "profit", s.sort_ascending))}
                                                    </span>
                                                </div>
                                            </TableHeaderCell>
                                            <TableHeaderCell resizable=false min_width=120.0 class="resizable">
                                                <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("status_norm")>
                                                    "Статус"
                                                    <span class=move || state.with(|s| get_sort_class(&s.sort_field, "status_norm"))>
                                                        {move || state.with(|s| get_sort_indicator(&s.sort_field, "status_norm", s.sort_ascending))}
                                                    </span>
                                                </div>
                                            </TableHeaderCell>
                                            <TableHeaderCell resizable=false min_width=140.0 class="resizable">
                                                <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("organization_ref")>
                                                    "Организация"
                                                    <span class=move || state.with(|s| get_sort_class(&s.sort_field, "organization_ref"))>
                                                        {move || state.with(|s| get_sort_indicator(&s.sort_field, "organization_ref", s.sort_ascending))}
                                                    </span>
                                                </div>
                                            </TableHeaderCell>
                                        </TableRow>
                                    </TableHeader>

                                    <TableBody>
                                        {items.into_iter().map(|sale| {
                                            let sale_date = sale.sale_date.clone();
                                            let marketplace = sale.marketplace.clone();
                                            let document_no = sale.document_no.clone();
                                            let title = sale.title.clone().unwrap_or_else(|| "—".to_string());
                                            let seller_sku = sale.seller_sku.clone().unwrap_or_else(|| "—".to_string());
                                            let qty = sale.qty;
                                            let amount_line = sale.amount_line;
                                            let status_norm = sale.status_norm.clone();
                                            let org_ref = sale.organization_ref.clone();
                                            let org_ref_short = if org_ref.len() > 8 {
                                                format!("{}...", &org_ref[..8])
                                            } else {
                                                org_ref.clone()
                                            };

                                            let amount_text = amount_line.map(format_number).unwrap_or_else(|| "—".to_string());
                                            let cost_total = sale.cost.map(|c| c * qty);
                                            let cost_text = cost_total.map(format_number).unwrap_or_else(|| "—".to_string());
                                            let profit_value = sale.cost.map(|c| amount_line.unwrap_or(0.0) - c * qty);
                                            let profit_text = profit_value.map(format_number).unwrap_or_else(|| "—".to_string());
                                            let profit_style = match profit_value {
                                                Some(p) if p >= 0.0 => "color: var(--color-success); font-weight: 600;",
                                                Some(_) => "color: var(--color-error); font-weight: 600;",
                                                None => "",
                                            };

                                            let document_type = sale.document_type.clone();
                                            let registrator_ref = sale.registrator_ref.clone();
                                            let document_no_for_display = document_no.clone();

                                            view! {
                                                <TableRow>
                                                    <TableCell><TableCellLayout>{sale_date}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{marketplace}</TableCellLayout></TableCell>
                                                    <TableCell>
                                                        <TableCellLayout truncate=true>
                                                            <a
                                                                href="#"
                                                                style="color: var(--colorBrandForeground1); text-decoration: none; cursor: pointer;"
                                                                on:click=move |ev| {
                                                                    ev.prevent_default();
                                                                    set_selected_document.set(Some(SelectedDocument {
                                                                        document_type: document_type.clone(),
                                                                        document_id: registrator_ref.clone(),
                                                                    }));
                                                                }
                                                            >
                                                                {document_no_for_display}
                                                            </a>
                                                        </TableCellLayout>
                                                    </TableCell>
                                                    <TableCell><TableCellLayout truncate=true>{title}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout truncate=true>{seller_sku}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{format_number(qty)}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{amount_text}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{cost_text}</TableCellLayout></TableCell>
                                                    <TableCell>
                                                        <TableCellLayout>
                                                            <span style=profit_style>{profit_text}</span>
                                                        </TableCellLayout>
                                                    </TableCell>
                                                    <TableCell><TableCellLayout truncate=true>{status_norm}</TableCellLayout></TableCell>
                                                    <TableCell>
                                                        <TableCellLayout truncate=true>
                                                            <span title=org_ref style="font-family: monospace; font-size: 0.85em;">
                                                                {org_ref_short}
                                                            </span>
                                                        </TableCellLayout>
                                                    </TableCell>
                                                </TableRow>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </TableBody>
                                </Table>
                            </div>
                        }.into_any()
                    }}
                </div>
            </div>
        </div>
    }
}

fn export_to_csv(data: &[SalesRegisterDto]) -> Result<(), String> {
    use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

    // UTF-8 BOM для правильного отображения кириллицы в Excel
    let mut csv = String::from("\u{FEFF}");

    // Заголовок с точкой с запятой как разделитель
    csv.push_str(
        "Date;Marketplace;Document №;Product;SKU;Qty;Amount;Cost;Profit;Status;Organization\n",
    );

    for sale in data {
        let title = sale.title.as_deref().unwrap_or("").replace("\"", "\"\"");
        let seller_sku = sale
            .seller_sku
            .as_deref()
            .unwrap_or("")
            .replace("\"", "\"\"");
        let amount_line = sale.amount_line.unwrap_or(0.0);
        let org_ref_short = &sale.organization_ref[..8.min(sale.organization_ref.len())];

        // Себестоимость и прибыль
        let cost_total = sale.cost.map(|c| c * sale.qty);
        let profit = sale.cost.map(|c| amount_line - c * sale.qty);

        // Форматируем числа с запятой как десятичный разделитель
        let qty_str = format!("{:.2}", sale.qty).replace(".", ",");
        let amount_str = format!("{:.2}", amount_line).replace(".", ",");
        let cost_str = match cost_total {
            Some(c) => format!("{:.2}", c).replace(".", ","),
            None => "".to_string(),
        };
        let profit_str = match profit {
            Some(p) => format!("{:.2}", p).replace(".", ","),
            None => "".to_string(),
        };

        csv.push_str(&format!(
            "\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";{};{};{};{};\"{}\";\"{}\"\n",
            sale.sale_date,
            sale.marketplace,
            sale.document_no,
            title,
            seller_sku,
            qty_str,
            amount_str,
            cost_str,
            profit_str,
            sale.status_norm,
            org_ref_short
        ));
    }

    // Создаем Blob с CSV данными
    let blob_parts = js_sys::Array::new();
    blob_parts.push(&wasm_bindgen::JsValue::from_str(&csv));

    let blob_props = BlobPropertyBag::new();
    blob_props.set_type("text/csv;charset=utf-8;");

    let blob = Blob::new_with_str_sequence_and_options(&blob_parts, &blob_props)
        .map_err(|e| format!("Failed to create blob: {:?}", e))?;

    // Создаем URL для blob
    let url = Url::create_object_url_with_blob(&blob)
        .map_err(|e| format!("Failed to create URL: {:?}", e))?;

    // Создаем временную ссылку для скачивания
    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let document = window.document().ok_or_else(|| "no document".to_string())?;

    let a = document
        .create_element("a")
        .map_err(|e| format!("Failed to create element: {:?}", e))?
        .dyn_into::<HtmlAnchorElement>()
        .map_err(|e| format!("Failed to cast to anchor: {:?}", e))?;

    a.set_href(&url);
    let filename = format!(
        "sales_register_{}.csv",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    a.set_download(&filename);
    a.click();

    // Освобождаем URL
    Url::revoke_object_url(&url).map_err(|e| format!("Failed to revoke URL: {:?}", e))?;

    Ok(())
}

async fn fetch_sales(query_params: &str) -> Result<SalesRegisterListResponse, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("/api/p900/sales-register{}", query_params);
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
    let data: SalesRegisterListResponse =
        serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}
