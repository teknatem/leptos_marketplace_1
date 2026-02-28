pub mod state;

use self::state::create_state;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::date_range_picker::DateRangePicker;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::table::{
    TableCellCheckbox, TableCellMoney, TableCrosshairHighlight, TableHeaderCheckbox,
};
use crate::shared::components::ui::badge::Badge as UiBadge;
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator};
use crate::shared::page_frame::PageFrame;
use crate::shared::table_utils::init_column_resize;
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use thaw::*;
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};
use wasm_bindgen::JsCast;

use crate::shared::api_utils::api_base;

/// Форматирует ISO 8601 дату в dd.mm.yyyy
fn format_date(iso_date: &str) -> String {
    if let Some(date_part) = iso_date.split('T').next() {
        if let Some((year, rest)) = date_part.split_once('-') {
            if let Some((month, day)) = rest.split_once('-') {
                return format!("{}.{}.{}", day, month, year);
            }
        }
    }
    iso_date.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProductionOutputDto {
    pub id: String,
    pub document_no: String,
    pub document_date: String,
    pub description: String,
    pub article: String,
    pub count: i64,
    pub amount: f64,
    pub cost_of_production: Option<f64>,
    pub nomenclature_ref: Option<String>,
    pub nomenclature_code: Option<String>,
    pub nomenclature_article: Option<String>,
    pub connection_id: String,
    pub fetched_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse {
    pub items: Vec<serde_json::Value>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

const TABLE_ID: &str = "a021-production-output-table";
const COLUMN_WIDTHS_KEY: &str = "a021_production_output_column_widths";

#[component]
pub fn ProductionOutputList() -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let state = create_state();
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (is_filter_expanded, set_is_filter_expanded) = signal(false);

    const FORM_KEY: &str = "a021_production_output";

    let open_detail = move |id: String, document_no: String| {
        tabs_store.open_tab(
            &format!("a021_production_output_detail_{}", id),
            &format!("Выпуск {}", document_no),
        );
    };

    let load_items = move || {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let date_from_val = state.with_untracked(|s| s.date_from.clone());
            let date_to_val = state.with_untracked(|s| s.date_to.clone());
            let search_query_val = state.with_untracked(|s| s.search_query.clone());
            let page = state.with_untracked(|s| s.page);
            let page_size = state.with_untracked(|s| s.page_size);
            let sort_field = state.with_untracked(|s| s.sort_field.clone());
            let sort_ascending = state.with_untracked(|s| s.sort_ascending);
            let offset = page * page_size;
            let cache_buster = js_sys::Date::now() as i64;

            let mut url = format!(
                "{}/api/a021/production-output/list?date_from={}&date_to={}&limit={}&offset={}&sort_by={}&sort_desc={}&_ts={}",
                api_base(),
                date_from_val,
                date_to_val,
                page_size,
                offset,
                sort_field,
                !sort_ascending,
                cache_buster
            );

            if !search_query_val.is_empty() {
                url.push_str(&format!("&search_query={}", urlencoding::encode(&search_query_val)));
            }

            log!("Loading production output: {}", url);

            match Request::get(&url)
                .header("Cache-Control", "no-cache, no-store, must-revalidate")
                .header("Pragma", "no-cache")
                .send()
                .await
            {
                Ok(response) => {
                    if response.ok() {
                        match response.json::<PaginatedResponse>().await {
                            Ok(paginated) => {
                                let parsed: Vec<ProductionOutputDto> = paginated
                                    .items
                                    .into_iter()
                                    .filter_map(|v| {
                                        Some(ProductionOutputDto {
                                            id: v.get("id")?.as_str()?.to_string(),
                                            document_no: v
                                                .get("document_no")
                                                .and_then(|x| x.as_str())
                                                .unwrap_or("")
                                                .to_string(),
                                            document_date: v
                                                .get("document_date")
                                                .and_then(|x| x.as_str())
                                                .unwrap_or("")
                                                .to_string(),
                                            description: v
                                                .get("description")
                                                .and_then(|x| x.as_str())
                                                .unwrap_or("")
                                                .to_string(),
                                            article: v
                                                .get("article")
                                                .and_then(|x| x.as_str())
                                                .unwrap_or("")
                                                .to_string(),
                                            count: v
                                                .get("count")
                                                .and_then(|x| x.as_i64())
                                                .unwrap_or(0),
                                            amount: v
                                                .get("amount")
                                                .and_then(|x| x.as_f64())
                                                .unwrap_or(0.0),
                                            cost_of_production: v
                                                .get("cost_of_production")
                                                .and_then(|x| x.as_f64()),
                                            nomenclature_ref: v
                                                .get("nomenclature_ref")
                                                .and_then(|x| x.as_str())
                                                .map(String::from),
                                            nomenclature_code: v
                                                .get("nomenclature_code")
                                                .and_then(|x| x.as_str())
                                                .map(String::from),
                                            nomenclature_article: v
                                                .get("nomenclature_article")
                                                .and_then(|x| x.as_str())
                                                .map(String::from),
                                            connection_id: v
                                                .get("connection_id")
                                                .and_then(|x| x.as_str())
                                                .unwrap_or("")
                                                .to_string(),
                                            fetched_at: v
                                                .get("fetched_at")
                                                .and_then(|x| x.as_str())
                                                .unwrap_or("")
                                                .to_string(),
                                        })
                                    })
                                    .collect();

                                state.update(|s| {
                                    s.items = parsed;
                                    s.total_count = paginated.total;
                                    s.total_pages = paginated.total_pages;
                                    s.page = paginated.page;
                                    s.page_size = paginated.page_size;
                                    s.is_loaded = true;
                                });
                                set_loading.set(false);
                            }
                            Err(e) => {
                                set_error.set(Some(format!("Ошибка парсинга: {}", e)));
                                set_loading.set(false);
                            }
                        }
                    } else {
                        set_error.set(Some(format!("Ошибка сервера: {}", response.status())));
                        set_loading.set(false);
                    }
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
                            if let Some(v) = settings.get("date_from").and_then(|v| v.as_str()) {
                                s.date_from = v.to_string();
                            }
                            if let Some(v) = settings.get("date_to").and_then(|v| v.as_str()) {
                                s.date_to = v.to_string();
                            }
                        });
                        log!("Loaded saved settings for A021");
                        load_items();
                    }
                    Ok(None) => {
                        log!("No saved settings for A021");
                        load_items();
                    }
                    Err(e) => {
                        log!("Failed to load settings: {}", e);
                        load_items();
                    }
                }
            });
        } else {
            log!("Used cached data for A021");
        }
    });

    let search_query = RwSignal::new(state.get_untracked().search_query.clone());

    Effect::new(move || {
        let v = search_query.get();
        untrack(move || {
            state.update(|s| s.search_query = v);
        });
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
        if !s.search_query.is_empty() { count += 1; }
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
        load_items();
    };

    let go_to_page = move |new_page: usize| {
        state.update(|s| s.page = new_page);
        load_items();
    };

    let change_page_size = move |new_size: usize| {
        state.update(|s| {
            s.page_size = new_size;
            s.page = 0;
        });
        load_items();
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
        if check_all {
            let items = state.get().items;
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
        <PageFrame page_id="a021_production_output--list" category="list">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Выпуск продукции"</h1>
                    <UiBadge variant="primary".to_string()>
                        {move || state.get().total_count.to_string()}
                    </UiBadge>
                </div>

                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| {
                            let data = state.get().items;
                            if let Err(e) = export_to_csv(&data) {
                                log!("Failed to export: {}", e);
                            }
                        }
                        disabled=Signal::derive(move || loading.get() || state.get().items.is_empty())
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
                                on_click=move |_| load_items()
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
                                            load_items();
                                        })
                                        label="Период:".to_string()
                                    />
                                </div>

                                <div style="flex: 1; max-width: 320px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Поиск:"</Label>
                                        <Input
                                            value=search_query
                                            placeholder="Номер, наименование, артикул..."
                                        />
                                    </Flex>
                                </div>

                                <Button
                                    appearance=ButtonAppearance::Secondary
                                    on_click=move |_| {
                                        state.update(|s| s.page = 0);
                                        load_items();
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

                    <Table attr:id=TABLE_ID attr:style="width: 100%; min-width: 1100px;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCheckbox
                                    items=items_signal
                                    selected=selected_signal
                                    get_id=Callback::new(|row: ProductionOutputDto| row.id.clone())
                                    on_change=Callback::new(toggle_all)
                                />

                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("document_date")>
                                        "Дата"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "document_date"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "document_date", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=150.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("document_no")>
                                        "Номер документа"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "document_no"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "document_no", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=200.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("description")>
                                        "Наименование"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "description"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "description", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=120.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("article")>
                                        "Артикул"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "article"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "article", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=160.0 class="resizable">
                                    "Артикул 1С"
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=90.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("count")>
                                        "Кол-во"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "count"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "count", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("amount")>
                                        "Сумма"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "amount"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "amount", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("cost_of_production")>
                                        "С/с на 1 шт"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "cost_of_production"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "cost_of_production", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=130.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("nomenclature_article")>
                                        "Номенклатура"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "nomenclature_article"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "nomenclature_article", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                            </TableRow>
                        </TableHeader>

                        <TableBody>
                            <For
                                each=move || state.get().items
                                key=|item| format!("{}:{}", item.id, item.nomenclature_ref.as_deref().unwrap_or(""))
                                children=move |item| {
                                    let item_id = item.id.clone();
                                    let item_id_for_link = item_id.clone();
                                    let document_no_for_link = item.document_no.clone();
                                    let document_no_text = item.document_no.clone();
                                    let formatted_date = format_date(&item.document_date);

                                    let has_nom = item.nomenclature_ref.is_some();
                                    let nom_class = if has_nom { "badge badge--success" } else { "badge badge--warning" };
                                    let nom_text = match (&item.nomenclature_code, &item.nomenclature_article) {
                                        (_, Some(art)) if !art.is_empty() => art.clone(),
                                        (Some(code), _) if !code.is_empty() => code.clone(),
                                        _ => if has_nom { "Найдена".to_string() } else { "Не найдена".to_string() },
                                    };

                                    view! {
                                        <TableRow>
                                            <TableCellCheckbox
                                                item_id=item_id.clone()
                                                selected=selected_signal
                                                on_change=Callback::new(move |(id, checked)| toggle_selection(id, checked))
                                            />

                                            <TableCell>
                                                <TableCellLayout>
                                                    {formatted_date}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    <a
                                                        href="#"
                                                        class="table__link"
                                                        on:click=move |e| {
                                                            e.prevent_default();
                                                            open_detail(item_id_for_link.clone(), document_no_for_link.clone());
                                                        }
                                                    >
                                                        {document_no_text}
                                                    </a>
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {item.description.clone()}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {item.article.clone()}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {item.nomenclature_article.clone().unwrap_or_else(|| "—".to_string())}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout>
                                                    <span style="font-variant-numeric: tabular-nums;">
                                                        {item.count}
                                                    </span>
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCellMoney
                                                value=Some(item.amount)
                                                show_currency=false
                                                color_by_sign=false
                                            />

                                            <TableCellMoney
                                                value=item.cost_of_production
                                                show_currency=false
                                                color_by_sign=false
                                            />

                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    <span class=nom_class>{nom_text}</span>
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
    let url = format!("{}/api/user-form-settings/{}", api_base(), form_key);
    match Request::get(&url).send().await {
        Ok(response) => {
            if response.status() == 200 {
                match response.json::<serde_json::Value>().await {
                    Ok(settings) => Ok(Some(settings)),
                    Err(e) => {
                        log!("Failed to parse settings: {}", e);
                        Ok(None)
                    }
                }
            } else if response.status() == 404 {
                Ok(None)
            } else {
                Err(format!("HTTP {}", response.status()))
            }
        }
        Err(e) => {
            log!("Failed to fetch settings: {:?}", e);
            Err(format!("{:?}", e))
        }
    }
}

fn export_to_csv(data: &[ProductionOutputDto]) -> Result<(), String> {
    let mut csv = String::from("\u{FEFF}");
    csv.push_str("Дата;Номер документа;Наименование;Артикул;Артикул 1С;Кол-во;Сумма;С/с на 1 шт;Номенклатура\n");

    for item in data {
        let date = format_date(&item.document_date);
        let nom_article = item.nomenclature_article.as_deref().unwrap_or("—");
        let cost = item
            .cost_of_production
            .map(|v| format!("{:.2}", v).replace(".", ","))
            .unwrap_or_else(|| "—".to_string());
        let has_nom = if item.nomenclature_ref.is_some() { "Да" } else { "Нет" };

        csv.push_str(&format!(
            "\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";{};{};{};{}\n",
            date,
            item.document_no.replace('"', "\"\""),
            item.description.replace('"', "\"\""),
            item.article.replace('"', "\"\""),
            nom_article.replace('"', "\"\""),
            item.count,
            format!("{:.2}", item.amount).replace(".", ","),
            cost,
            has_nom,
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

    let a = document
        .create_element("a")
        .map_err(|e| format!("Failed to create element: {:?}", e))?
        .dyn_into::<HtmlAnchorElement>()
        .map_err(|e| format!("Failed to cast to anchor: {:?}", e))?;

    a.set_href(&url);
    a.set_download(&format!(
        "production_output_{}.csv",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    ));
    a.click();

    Url::revoke_object_url(&url).map_err(|e| format!("Failed to revoke URL: {:?}", e))?;

    Ok(())
}
