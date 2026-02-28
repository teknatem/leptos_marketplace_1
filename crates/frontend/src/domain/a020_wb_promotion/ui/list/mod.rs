pub mod state;

use self::state::create_state;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::date_range_picker::DateRangePicker;
use crate::shared::components::pagination_controls::PaginationControls;
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
use std::cmp::Ordering;
use thaw::*;

use crate::shared::api_utils::api_base;
use crate::shared::list_utils::Sortable;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WbPromotionDto {
    pub id: String,
    pub document_no: String,
    pub promotion_id: i64,
    pub name: String,
    pub promotion_type: Option<String>,
    pub start_date_time: String,
    pub end_date_time: String,
    pub in_promo_action_total: Option<i32>,
    pub nomenclatures_count: i64,
    pub is_posted: bool,
    pub connection_id: String,
    pub organization_id: String,
    pub organization_name: Option<String>,
}

impl Sortable for WbPromotionDto {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "name" => self.name.to_lowercase().cmp(&other.name.to_lowercase()),
            "promotion_type" => match (&self.promotion_type, &other.promotion_type) {
                (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            "start_date_time" => self.start_date_time.cmp(&other.start_date_time),
            "end_date_time" => self.end_date_time.cmp(&other.end_date_time),
            "in_promo_action_total" => match (&self.in_promo_action_total, &other.in_promo_action_total) {
                (Some(a), Some(b)) => a.cmp(b),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            _ => Ordering::Equal,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse {
    pub items: Vec<serde_json::Value>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

const TABLE_ID: &str = "a020-wb-promotions-table";
const COLUMN_WIDTHS_KEY: &str = "a020_wb_promotion_column_widths";

fn format_date(dt: &str) -> String {
    if let Some(date_part) = dt.split('T').next() {
        if let Some((year, rest)) = date_part.split_once('-') {
            if let Some((month, day)) = rest.split_once('-') {
                return format!("{}.{}.{}", day, month, year);
            }
        }
    }
    dt.to_string()
}

#[component]
pub fn WbPromotionList() -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let state = create_state();
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (is_filter_expanded, set_is_filter_expanded) = signal(false);

    let open_detail = move |id: String, name: String| {
        tabs_store.open_tab(
            &format!("a020_wb_promotion_detail_{}", id),
            &format!("WB Акция: {}", name),
        );
    };

    let load_promotions = move || {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let date_from_val = state.with_untracked(|s| s.date_from.clone());
            let date_to_val = state.with_untracked(|s| s.date_to.clone());
            let conn_id = state.with_untracked(|s| s.selected_connection_id.clone());
            let search_query_val = state.with_untracked(|s| s.search_query.clone());
            let page = state.with_untracked(|s| s.page);
            let page_size = state.with_untracked(|s| s.page_size);
            let sort_field = state.with_untracked(|s| s.sort_field.clone());
            let sort_ascending = state.with_untracked(|s| s.sort_ascending);
            let offset = page * page_size;
            let cache_buster = js_sys::Date::now() as i64;

            let mut url = format!(
                "{}/api/a020/wb-promotions?date_from={}&date_to={}&limit={}&offset={}&sort_by={}&sort_desc={}&_ts={}",
                api_base(),
                date_from_val,
                date_to_val,
                page_size,
                offset,
                sort_field,
                !sort_ascending,
                cache_buster
            );

            if let Some(cid) = conn_id {
                url.push_str(&format!("&connection_id={}", cid));
            }
            if !search_query_val.is_empty() {
                url.push_str(&format!("&search_query={}", search_query_val));
            }

            log!("Loading WB promotions with URL: {}", url);

            match Request::get(&url).send().await {
                Ok(response) => {
                    if response.ok() {
                        match response.json::<PaginatedResponse>().await {
                            Ok(paginated) => {
                                let parsed: Vec<WbPromotionDto> = paginated
                                    .items
                                    .into_iter()
                                    .filter_map(|item| {
                                        let id = item.get("id")?.as_str()?.to_string();
                                        let document_no =
                                            item.get("document_no")?.as_str()?.to_string();
                                        let promotion_id =
                                            item.get("promotion_id")?.as_i64().unwrap_or(0);
                                        let name = item
                                            .get("name")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        let promotion_type = item
                                            .get("promotion_type")
                                            .and_then(|v| v.as_str())
                                            .map(String::from);
                                        let start_date_time = item
                                            .get("start_date_time")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        let end_date_time = item
                                            .get("end_date_time")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        let in_promo_action_total = item
                                            .get("in_promo_action_total")
                                            .and_then(|v| v.as_i64())
                                            .map(|v| v as i32);
                                        let nomenclatures_count = item
                                            .get("nomenclatures_count")
                                            .and_then(|v| v.as_i64())
                                            .unwrap_or(0);
                                        let is_posted = item
                                            .get("is_posted")
                                            .and_then(|v| v.as_bool())
                                            .unwrap_or(false);
                                        let connection_id = item
                                            .get("connection_id")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        let organization_id = item
                                            .get("organization_id")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        let organization_name = item
                                            .get("organization_name")
                                            .and_then(|v| v.as_str())
                                            .map(String::from);

                                        Some(WbPromotionDto {
                                            id,
                                            document_no,
                                            promotion_id,
                                            name,
                                            promotion_type,
                                            start_date_time,
                                            end_date_time,
                                            in_promo_action_total,
                                            nomenclatures_count,
                                            is_posted,
                                            connection_id,
                                            organization_id,
                                            organization_name,
                                        })
                                    })
                                    .collect();

                                state.update(|s| {
                                    s.promotions = parsed;
                                    s.total_count = paginated.total;
                                    s.total_pages = paginated.total_pages;
                                    s.page = paginated.page;
                                    s.page_size = paginated.page_size;
                                    s.is_loaded = true;
                                });
                                set_loading.set(false);
                            }
                            Err(e) => {
                                set_error.set(Some(format!("Failed to parse response: {}", e)));
                                set_loading.set(false);
                            }
                        }
                    } else {
                        set_error
                            .set(Some(format!("Server error: {}", response.status())));
                        set_loading.set(false);
                    }
                }
                Err(e) => {
                    set_error.set(Some(format!("Failed to fetch promotions: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            load_promotions();
        }
    });

    let search_query = RwSignal::new(state.get_untracked().search_query.clone());

    Effect::new(move || {
        let v = search_query.get();
        untrack(move || {
            state.update(|s| {
                s.search_query = v;
            });
        });
    });

    let resize_initialized = leptos::prelude::StoredValue::new(false);
    Effect::new(move |_| {
        if !resize_initialized.get_value() {
            resize_initialized.set_value(true);
            spawn_local(async move {
                gloo_timers::future::TimeoutFuture::new(100).await;
                init_column_resize(TABLE_ID, COLUMN_WIDTHS_KEY);
            });
        }
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
        load_promotions();
    };

    let go_to_page = move |new_page: usize| {
        state.update(|s| s.page = new_page);
        load_promotions();
    };

    let change_page_size = move |new_size: usize| {
        state.update(|s| {
            s.page_size = new_size;
            s.page = 0;
        });
        load_promotions();
    };

    let active_filters_count = Signal::derive(move || {
        let s = state.get();
        let mut count = 0usize;
        if !s.date_from.is_empty() { count += 1; }
        if !s.date_to.is_empty() { count += 1; }
        if s.selected_connection_id.is_some() { count += 1; }
        if !s.search_query.is_empty() { count += 1; }
        count
    });

    view! {
        <PageFrame page_id="a020_wb_promotion--list" category="list">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Акции WB (Календарь)"</h1>
                    <UiBadge variant="primary".to_string()>
                        {move || state.get().total_count.to_string()}
                    </UiBadge>
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=move |_| load_promotions()
                        disabled=Signal::derive(move || loading.get())
                    >
                        {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
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
                                page_size_options=vec![50, 100, 200]
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
                                            load_promotions();
                                        })
                                        label="Период акций:".to_string()
                                    />
                                </div>
                                <div style="flex: 1; max-width: 320px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Поиск:"</Label>
                                        <Input
                                            value=search_query
                                            placeholder="Название акции, тип..."
                                        />
                                    </Flex>
                                </div>
                                <Button
                                    appearance=ButtonAppearance::Secondary
                                    on_click=move |_| {
                                        state.update(|s| {
                                            s.search_query = String::new();
                                            s.page = 0;
                                        });
                                        search_query.set(String::new());
                                        load_promotions();
                                    }
                                >
                                    "Сбросить"
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
                    <Table attr:id=TABLE_ID attr:style="width: 100%; min-width: 900px;">
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCell resizable=false min_width=280.0 class="resizable">
                                    <div
                                        class="table__sortable-header"
                                        style="cursor: pointer;"
                                        on:click=move |_| toggle_sort("name")
                                    >
                                        "Название акции"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "name"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "name", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=140.0 class="resizable">
                                    <div
                                        class="table__sortable-header"
                                        style="cursor: pointer;"
                                        on:click=move |_| toggle_sort("promotion_type")
                                    >
                                        "Тип"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "promotion_type"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "promotion_type", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=120.0 class="resizable">
                                    <div
                                        class="table__sortable-header"
                                        style="cursor: pointer;"
                                        on:click=move |_| toggle_sort("start_date_time")
                                    >
                                        "Начало"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "start_date_time"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "start_date_time", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=120.0 class="resizable">
                                    <div
                                        class="table__sortable-header"
                                        style="cursor: pointer;"
                                        on:click=move |_| toggle_sort("end_date_time")
                                    >
                                        "Окончание"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "end_date_time"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "end_date_time", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    <div
                                        class="table__sortable-header"
                                        style="cursor: pointer;"
                                        on:click=move |_| toggle_sort("in_promo_action_total")
                                    >
                                        "Товаров WB"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "in_promo_action_total"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "in_promo_action_total", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    "Номенклатур"
                                </TableHeaderCell>
                                <TableHeaderCell resizable=false min_width=160.0 class="resizable">
                                    "Организация"
                                </TableHeaderCell>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            <For
                                each=move || state.get().promotions
                                key=|item| item.id.clone()
                                children=move |promo| {
                                    let promo_id = promo.id.clone();
                                    let promo_name = promo.name.clone();
                                    let promo_name_link = promo.name.clone();
                                    let start_fmt = format_date(&promo.start_date_time);
                                    let end_fmt = format_date(&promo.end_date_time);

                                    view! {
                                        <TableRow>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    <a
                                                        href="#"
                                                        class="table__link"
                                                        style="color: #0f6cbd; text-decoration: underline;"
                                                        on:click=move |e| {
                                                            e.prevent_default();
                                                            open_detail(promo_id.clone(), promo_name_link.clone());
                                                        }
                                                    >
                                                        {promo_name}
                                                    </a>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {promo.promotion_type.clone().unwrap_or_else(|| "—".to_string())}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    {start_fmt}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    {end_fmt}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    {promo.in_promo_action_total
                                                        .map(|v| v.to_string())
                                                        .unwrap_or_else(|| "—".to_string())}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    {promo.nomenclatures_count.to_string()}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {promo.organization_name.clone().unwrap_or_else(|| "—".to_string())}
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
