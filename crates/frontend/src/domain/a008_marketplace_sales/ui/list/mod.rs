pub mod state;

use self::state::create_state;
use crate::shared::api_utils::api_base;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::table::{
    TableCellCheckbox, TableCrosshairHighlight, TableHeaderCheckbox,
};
use crate::shared::components::ui::badge::Badge as UiBadge;
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator, Sortable};
use crate::shared::page_frame::PageFrame;
use crate::shared::table_utils::init_column_resize;
use contracts::domain::a002_organization::aggregate::Organization;
use contracts::domain::a005_marketplace::aggregate::Marketplace;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::a007_marketplace_product::aggregate::MarketplaceProduct;
use contracts::domain::a008_marketplace_sales::aggregate::MarketplaceSales;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cmp::Ordering;
use thaw::*;

#[derive(Clone, Debug)]
pub struct MarketplaceSalesRow {
    pub id: String,
    pub connection_name: String,
    pub organization_name: String,
    pub marketplace_name: String,
    pub accrual_date: String,
    pub product_name: String,
    pub quantity: i32,
    pub revenue: f64,
    pub operation_type: String,
}

impl MarketplaceSalesRow {
    fn from_sale(
        s: MarketplaceSales,
        conn_map: &std::collections::HashMap<String, String>,
        org_map: &std::collections::HashMap<String, String>,
        mp_map: &std::collections::HashMap<String, String>,
        product_map: &std::collections::HashMap<String, String>,
    ) -> Self {
        use contracts::domain::common::AggregateId;
        Self {
            id: s.base.id.as_string(),
            connection_name: conn_map
                .get(&s.connection_id)
                .cloned()
                .unwrap_or_else(|| "?".to_string()),
            organization_name: org_map
                .get(&s.organization_id)
                .cloned()
                .unwrap_or_else(|| "?".to_string()),
            marketplace_name: mp_map
                .get(&s.marketplace_id)
                .cloned()
                .unwrap_or_else(|| "?".to_string()),
            product_name: product_map
                .get(&s.product_id)
                .cloned()
                .unwrap_or_else(|| "?".to_string()),
            accrual_date: s.accrual_date.format("%Y-%m-%d").to_string(),
            quantity: s.quantity,
            revenue: s.revenue,
            operation_type: s.operation_type,
        }
    }
}

impl Sortable for MarketplaceSalesRow {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "connection" => self
                .connection_name
                .to_lowercase()
                .cmp(&other.connection_name.to_lowercase()),
            "organization" => self
                .organization_name
                .to_lowercase()
                .cmp(&other.organization_name.to_lowercase()),
            "marketplace" => self
                .marketplace_name
                .to_lowercase()
                .cmp(&other.marketplace_name.to_lowercase()),
            "accrual_date" => self.accrual_date.cmp(&other.accrual_date),
            "product" => self
                .product_name
                .to_lowercase()
                .cmp(&other.product_name.to_lowercase()),
            "quantity" => self.quantity.cmp(&other.quantity),
            "revenue" => self
                .revenue
                .partial_cmp(&other.revenue)
                .unwrap_or(Ordering::Equal),
            "operation_type" => self
                .operation_type
                .to_lowercase()
                .cmp(&other.operation_type.to_lowercase()),
            _ => Ordering::Equal,
        }
    }
}

const TABLE_ID: &str = "a008-marketplace-sales-table";
const COLUMN_WIDTHS_KEY: &str = "a008_marketplace_sales_column_widths";

#[component]
#[allow(non_snake_case)]
pub fn MarketplaceSalesList() -> impl IntoView {
    use std::collections::HashMap;
    let state = create_state();
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (is_filter_expanded, set_is_filter_expanded) = signal(false);

    // All loaded rows (unfiltered source of truth)
    let all_rows: RwSignal<Vec<MarketplaceSalesRow>> = RwSignal::new(Vec::new());

    let (connections, set_connections) = signal::<Vec<ConnectionMP>>(Vec::new());
    let (organizations, set_organizations) = signal::<Vec<Organization>>(Vec::new());
    let (marketplaces, set_marketplaces) = signal::<Vec<Marketplace>>(Vec::new());
    let (products, set_products) = signal::<Vec<MarketplaceProduct>>(Vec::new());

    let conn_map = move || -> HashMap<String, String> {
        connections.get().into_iter().map(|x| {
            use contracts::domain::common::AggregateId;
            (x.base.id.as_string(), x.base.description)
        }).collect()
    };
    let org_map = move || -> HashMap<String, String> {
        organizations.get().into_iter().map(|x| {
            use contracts::domain::common::AggregateId;
            (x.base.id.as_string(), x.base.description)
        }).collect()
    };
    let mp_map = move || -> HashMap<String, String> {
        marketplaces.get().into_iter().map(|x| {
            use contracts::domain::common::AggregateId;
            (x.base.id.as_string(), x.base.description)
        }).collect()
    };
    let product_map = move || -> HashMap<String, String> {
        products.get().into_iter().map(|x| {
            use contracts::domain::common::AggregateId;
            (x.base.id.as_string(), x.base.description.clone())
        }).collect()
    };

    let refresh_view = move || {
        let source = all_rows.get_untracked();
        let query = state.with_untracked(|s| s.search_query.to_lowercase());
        let field = state.with_untracked(|s| s.sort_field.clone());
        let ascending = state.with_untracked(|s| s.sort_ascending);
        let page_size = state.with_untracked(|s| s.page_size);
        let page = state.with_untracked(|s| s.page);

        let mut filtered: Vec<MarketplaceSalesRow> = source
            .into_iter()
            .filter(|row| {
                if query.is_empty() {
                    true
                } else {
                    row.connection_name.to_lowercase().contains(&query)
                        || row.organization_name.to_lowercase().contains(&query)
                        || row.marketplace_name.to_lowercase().contains(&query)
                        || row.product_name.to_lowercase().contains(&query)
                        || row.operation_type.to_lowercase().contains(&query)
                }
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

    let load_data = move || {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let sales_res = fetch_sales().await;
            let conn_res = fetch_connections().await;
            let org_res = fetch_organizations().await;
            let mp_res = fetch_marketplaces().await;
            let prod_res = fetch_products().await;

            if let Ok(v) = conn_res { set_connections.set(v); }
            if let Ok(v) = org_res { set_organizations.set(v); }
            if let Ok(v) = mp_res { set_marketplaces.set(v); }
            if let Ok(v) = prod_res { set_products.set(v); }

            match sales_res {
                Ok(v) => {
                    let rows: Vec<MarketplaceSalesRow> = v
                        .into_iter()
                        .map(|s| MarketplaceSalesRow::from_sale(s, &conn_map(), &org_map(), &mp_map(), &product_map()))
                        .collect();
                    all_rows.set(rows);
                    state.update(|s| { s.page = 0; s.is_loaded = true; });
                    refresh_view();
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

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

    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            load_data();
        }
    });

    let search_query = RwSignal::new(state.get_untracked().search_query.clone());

    let active_filters_count = Signal::derive(move || {
        let s = state.get();
        if s.search_query.is_empty() { 0usize } else { 1 }
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

    view! {
        <PageFrame page_id="a008_marketplace_sales--list" category="list">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Продажи маркетплейсов"</h1>
                    <UiBadge variant="primary".to_string()>
                        {move || state.get().total_count.to_string()}
                    </UiBadge>
                </div>
                <div class="page__header-right">
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
                                <div style="flex: 1; max-width: 320px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Поиск:"</Label>
                                        <Input
                                            value=search_query
                                            placeholder="Подключение, маркетплейс, товар..."
                                        />
                                    </Flex>
                                </div>

                                <Button
                                    appearance=ButtonAppearance::Secondary
                                    on_click=move |_| {
                                        state.update(|s| {
                                            s.search_query = search_query.get_untracked();
                                            s.page = 0;
                                        });
                                        refresh_view();
                                    }
                                    disabled=Signal::derive(move || loading.get())
                                >
                                    "Найти"
                                </Button>
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

                    <Table attr:id=TABLE_ID attr:style="width: 100%; min-width: 900px;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCheckbox
                                    items=items_signal
                                    selected=selected_signal
                                    get_id=Callback::new(|row: MarketplaceSalesRow| row.id.clone())
                                    on_change=Callback::new(toggle_all)
                                />

                                <TableHeaderCell resizable=false min_width=140.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("connection")>
                                        "Подключение"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "connection"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "connection", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=140.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("organization")>
                                        "Организация"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "organization"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "organization", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=120.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("marketplace")>
                                        "Маркетплейс"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "marketplace"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "marketplace", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=100.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("accrual_date")>
                                        "Дата начисл."
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "accrual_date"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "accrual_date", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=200.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("product")>
                                        "Позиция"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "product"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "product", state.with(|s| s.sort_ascending))}
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

                                <TableHeaderCell resizable=false min_width=100.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("revenue")>
                                        "Выручка"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "revenue"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "revenue", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=130.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("operation_type")>
                                        "Тип операции"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "operation_type"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "operation_type", state.with(|s| s.sort_ascending))}
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
                                    view! {
                                        <TableRow>
                                            <TableCellCheckbox
                                                item_id=item_id.clone()
                                                selected=selected_signal
                                                on_change=Callback::new(move |(id, checked)| toggle_selection(id, checked))
                                            />
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {item.connection_name.clone()}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {item.organization_name.clone()}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {item.marketplace_name.clone()}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    {item.accrual_date.clone()}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {item.product_name.clone()}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    <span style="font-variant-numeric: tabular-nums;">
                                                        {item.quantity}
                                                    </span>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell class="text-right">
                                                <TableCellLayout>
                                                    <span style="font-variant-numeric: tabular-nums;">
                                                        {format!("{:.2}", item.revenue)}
                                                    </span>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {item.operation_type.clone()}
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

async fn fetch_sales() -> Result<Vec<MarketplaceSales>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};
    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);
    let url = format!("{}/api/marketplace_sales", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request.headers().set("Accept", "application/json").map_err(|e| format!("{e:?}"))?;
    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await.map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() { return Err(format!("HTTP {}", resp.status())); }
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await.map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("{e}"))
}

async fn fetch_connections() -> Result<Vec<ConnectionMP>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};
    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);
    let url = format!("{}/api/connection_mp", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request.headers().set("Accept", "application/json").map_err(|e| format!("{e:?}"))?;
    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await.map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() { return Err(format!("HTTP {}", resp.status())); }
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await.map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("{e}"))
}

async fn fetch_organizations() -> Result<Vec<Organization>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};
    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);
    let url = format!("{}/api/organization", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request.headers().set("Accept", "application/json").map_err(|e| format!("{e:?}"))?;
    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await.map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() { return Err(format!("HTTP {}", resp.status())); }
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await.map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("{e}"))
}

async fn fetch_marketplaces() -> Result<Vec<Marketplace>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};
    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);
    let url = format!("{}/api/marketplace", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request.headers().set("Accept", "application/json").map_err(|e| format!("{e:?}"))?;
    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await.map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() { return Err(format!("HTTP {}", resp.status())); }
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await.map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("{e}"))
}

async fn fetch_products() -> Result<Vec<MarketplaceProduct>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};
    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);
    let url = format!("{}/api/marketplace_product", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request.headers().set("Accept", "application/json").map_err(|e| format!("{e:?}"))?;
    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await.map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() { return Err(format!("HTTP {}", resp.status())); }
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await.map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("{e}"))
}
