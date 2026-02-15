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
use crate::shared::list_utils::{get_sort_class, get_sort_indicator, Sortable};
use crate::shared::table_utils::init_column_resize;
use contracts::domain::a013_ym_order::aggregate::YmOrderListDto;
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::cmp::Ordering;
use thaw::*;
use wasm_bindgen::JsCast;

use crate::shared::api_utils::api_base;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub code: String,
    pub description: String,
}

/// Paginated response from backend API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse {
    pub items: Vec<YmOrderListDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

/// Форматирует ISO 8601 дату в dd.mm.yyyy
fn format_date(iso_date: &str) -> String {
    // Парсим ISO 8601: "2025-11-05T16:52:58.585775200Z"
    if let Some(date_part) = iso_date.split('T').next() {
        if let Some((year, rest)) = date_part.split_once('-') {
            if let Some((month, day)) = rest.split_once('-') {
                return format!("{}.{}.{}", day, month, year);
            }
        }
    }
    iso_date.to_string() // fallback
}

// YmOrderListDto imported from contracts::domain::a013_ym_order::aggregate

impl Sortable for YmOrderListDto {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "document_no" => self
                .document_no
                .to_lowercase()
                .cmp(&other.document_no.to_lowercase()),
            "status_changed_at" => self.status_changed_at.cmp(&other.status_changed_at),
            "creation_date" => self.creation_date.cmp(&other.creation_date),
            "delivery_date" => self.delivery_date.cmp(&other.delivery_date),
            "campaign_id" => self
                .campaign_id
                .to_lowercase()
                .cmp(&other.campaign_id.to_lowercase()),
            "status_norm" => self
                .status_norm
                .to_lowercase()
                .cmp(&other.status_norm.to_lowercase()),
            "is_error" => self.is_error.cmp(&other.is_error),
            "total_qty" => self
                .total_qty
                .partial_cmp(&other.total_qty)
                .unwrap_or(Ordering::Equal),
            "total_amount" => self
                .total_amount
                .partial_cmp(&other.total_amount)
                .unwrap_or(Ordering::Equal),
            "delivery_total" => match (&self.delivery_total, &other.delivery_total) {
                (Some(a), Some(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            "subsidies_total" => self
                .subsidies_total
                .partial_cmp(&other.subsidies_total)
                .unwrap_or(Ordering::Equal),
            "lines_count" => self.lines_count.cmp(&other.lines_count),
            _ => Ordering::Equal,
        }
    }
}

const FORM_KEY: &str = "a013_ym_order";
const TABLE_ID: &str = "ym-orders-table";
const COLUMN_WIDTHS_KEY: &str = "a013_ym_order_column_widths";

#[component]
pub fn YmOrderList() -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let state = create_state();
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);

    // Batch operation state
    let (posting_in_progress, set_posting_in_progress) = signal(false);

    // Organizations
    let (organizations, set_organizations) = signal::<Vec<Organization>>(Vec::new());

    // State for save settings notification
    let (save_notification, set_save_notification) = signal(None::<String>);

    let open_detail = move |id: String, document_no: String| {
        tabs_store.open_tab(
            &format!("a013_ym_order_detail_{}", id),
            &format!("YM Order {}", document_no),
        );
    };

    let load_orders = move || {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let date_from_val = state.with(|s| s.date_from.clone());
            let date_to_val = state.with(|s| s.date_to.clone());
            let org_id = state.with(|s| s.selected_organization_id.clone());
            let page = state.with(|s| s.page);
            let page_size = state.with(|s| s.page_size);
            let sort_field = state.with(|s| s.sort_field.clone());
            let sort_ascending = state.with(|s| s.sort_ascending);
            let search_order_no = state.with(|s| s.search_order_no.clone());
            let filter_status = state.with(|s| s.filter_status.clone());
            let offset = page * page_size;

            // Build URL with pagination parameters
            let mut url = format!(
                "{}/api/a013/ym-order/list?limit={}&offset={}&sort_by={}&sort_desc={}",
                api_base(),
                page_size,
                offset,
                sort_field,
                !sort_ascending
            );

            // Add date filter if specified
            if !date_from_val.is_empty() {
                url.push_str(&format!("&date_from={}", date_from_val));
            }
            if !date_to_val.is_empty() {
                url.push_str(&format!("&date_to={}", date_to_val));
            }

            // Add organization filter if selected
            if let Some(org_id) = org_id {
                if !org_id.is_empty() {
                    url.push_str(&format!("&organization_id={}", org_id));
                }
            }

            // Add search filters
            if !search_order_no.is_empty() {
                url.push_str(&format!("&search_document_no={}", search_order_no));
            }
            if !filter_status.is_empty() {
                url.push_str(&format!("&status_norm={}", filter_status));
            }

            log!("Loading YM orders with URL: {}", url);

            match Request::get(&url).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status == 200 {
                        match response.text().await {
                            Ok(text) => {
                                // Parse paginated response
                                match serde_json::from_str::<PaginatedResponse>(&text) {
                                    Ok(paginated) => {
                                        log!(
                                            "Parsed paginated response: total={}, page={}, total_pages={}",
                                            paginated.total, paginated.page, paginated.total_pages
                                        );

                                        state.update(|s| {
                                            s.orders = paginated.items;
                                            s.total_count = paginated.total;
                                            s.page = paginated.page;
                                            s.page_size = paginated.page_size;
                                            s.total_pages = paginated.total_pages;
                                            s.is_loaded = true;
                                        });
                                        set_loading.set(false);
                                    }
                                    Err(e) => {
                                        log!("Failed to parse paginated response: {:?}", e);
                                        set_error
                                            .set(Some(format!("Failed to parse response: {}", e)));
                                        set_loading.set(false);
                                    }
                                }
                            }
                            Err(e) => {
                                log!("Failed to read response text: {:?}", e);
                                set_error.set(Some(format!("Failed to read response: {}", e)));
                                set_loading.set(false);
                            }
                        }
                    } else {
                        set_error.set(Some(format!("Server error: {}", status)));
                        set_loading.set(false);
                    }
                }
                Err(e) => {
                    log!("Failed to fetch orders: {:?}", e);
                    set_error.set(Some(format!("Failed to fetch orders: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    // Load saved settings from database on mount IF not already loaded in memory
    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            // Load organizations first
            spawn_local(async move {
                match fetch_organizations().await {
                    Ok(orgs) => {
                        set_organizations.set(orgs);
                    }
                    Err(e) => {
                        log!("Failed to load organizations: {}", e);
                    }
                }
            });

            spawn_local(async move {
                match load_saved_settings(FORM_KEY).await {
                    Ok(Some(settings)) => {
                        state.update(|s| {
                            if let Some(date_from_val) =
                                settings.get("date_from").and_then(|v| v.as_str())
                            {
                                s.date_from = date_from_val.to_string();
                            }
                            if let Some(date_to_val) =
                                settings.get("date_to").and_then(|v| v.as_str())
                            {
                                s.date_to = date_to_val.to_string();
                            }
                            if let Some(org_id) = settings
                                .get("selected_organization_id")
                                .and_then(|v| v.as_str())
                            {
                                if !org_id.is_empty() {
                                    s.selected_organization_id = Some(org_id.to_string());
                                }
                            }
                        });
                        log!("Loaded saved settings for A013");
                        load_orders();
                    }
                    Ok(None) => {
                        log!("No saved settings found for A013");
                        load_orders();
                    }
                    Err(e) => {
                        log!("Failed to load saved settings: {}", e);
                        load_orders();
                    }
                }
            });
        } else {
            log!("Used cached data for A013");
        }
    });

    // Функция для изменения сортировки (сбрасывает на первую страницу)
    let toggle_sort = move |field: &'static str| {
        state.update(|s| {
            if s.sort_field == field {
                s.sort_ascending = !s.sort_ascending;
            } else {
                s.sort_field = field.to_string();
                s.sort_ascending = true;
            }
            s.page = 0; // Reset to first page on sort change
        });
        load_orders();
    };

    // Pagination: go to specific page
    let go_to_page = move |new_page: usize| {
        state.update(|s| s.page = new_page);
        load_orders();
    };

    // Pagination: change page size
    let change_page_size = move |new_size: usize| {
        state.update(|s| {
            s.page_size = new_size;
            s.page = 0; // Reset to first page
        });
        load_orders();
    };

    // Save current settings to database
    let save_settings_to_db = move |_| {
        let settings = json!({
            "date_from": state.with(|s| s.date_from.clone()),
            "date_to": state.with(|s| s.date_to.clone()),
            "selected_organization_id": state.with(|s| s.selected_organization_id.clone()).unwrap_or_default(),
        });

        spawn_local(async move {
            match save_settings_to_database(FORM_KEY, settings).await {
                Ok(_) => {
                    set_save_notification.set(Some("✓ Настройки сохранены".to_string()));
                    spawn_local(async move {
                        gloo_timers::future::TimeoutFuture::new(3000).await;
                        set_save_notification.set(None);
                    });
                }
                Err(e) => {
                    set_save_notification.set(Some(format!("✗ Ошибка: {}", e)));
                    log!("Failed to save settings: {}", e);
                }
            }
        });
    };

    // Load and restore settings from database
    let restore_settings = move |_| {
        spawn_local(async move {
            match load_saved_settings(FORM_KEY).await {
                Ok(Some(settings)) => {
                    state.update(|s| {
                        if let Some(date_from_val) =
                            settings.get("date_from").and_then(|v| v.as_str())
                        {
                            s.date_from = date_from_val.to_string();
                        }
                        if let Some(date_to_val) = settings.get("date_to").and_then(|v| v.as_str())
                        {
                            s.date_to = date_to_val.to_string();
                        }
                        if let Some(org_id) = settings
                            .get("selected_organization_id")
                            .and_then(|v| v.as_str())
                        {
                            if !org_id.is_empty() {
                                s.selected_organization_id = Some(org_id.to_string());
                            } else {
                                s.selected_organization_id = None;
                            }
                        }
                    });
                    set_save_notification.set(Some("✓ Настройки восстановлены".to_string()));
                    spawn_local(async move {
                        gloo_timers::future::TimeoutFuture::new(3000).await;
                        set_save_notification.set(None);
                    });
                    log!("Restored saved settings for A013");
                    load_orders();
                }
                Ok(None) => {
                    set_save_notification.set(Some("ℹ Нет сохраненных настроек".to_string()));
                    spawn_local(async move {
                        gloo_timers::future::TimeoutFuture::new(3000).await;
                        set_save_notification.set(None);
                    });
                    log!("No saved settings found for A013");
                }
                Err(e) => {
                    set_save_notification.set(Some(format!("✗ Ошибка: {}", e)));
                    log!("Failed to load saved settings: {}", e);
                }
            }
        });
    };

    // Filter panel collapsible state
    let (is_filter_expanded, set_is_filter_expanded) = signal(true);

    // Active filters count for badge
    let active_filters_count = Memo::new(move |_| {
        let mut count = 0;
        state.with(|s| {
            if !s.date_from.is_empty() || !s.date_to.is_empty() {
                count += 1;
            }
            if s.selected_organization_id.is_some() {
                count += 1;
            }
            if !s.search_order_no.is_empty() {
                count += 1;
            }
            if !s.filter_status.is_empty() {
                count += 1;
            }
        });
        count
    });

    // Selection management for Thaw components
    let selected = RwSignal::new(state.with_untracked(|s| s.selected_ids.clone()));

    let toggle_selection = move |id: String, checked: bool| {
        selected.update(|s| {
            if checked {
                s.insert(id.clone());
            } else {
                s.remove(&id);
            }
        });
        state.update(|s| {
            if checked {
                s.selected_ids.insert(id);
            } else {
                s.selected_ids.remove(&id);
            }
        });
    };

    let toggle_all = move |check_all: bool| {
        if check_all {
            let items = state.get().orders;
            selected.update(|s| {
                s.clear();
                for item in items.iter() {
                    s.insert(item.id.clone());
                }
            });
            state.update(|s| {
                s.selected_ids.clear();
                for item in items.iter() {
                    s.selected_ids.insert(item.id.clone());
                }
            });
        } else {
            selected.update(|s| s.clear());
            state.update(|s| s.selected_ids.clear());
        }
    };

    let items_signal = Signal::derive(move || state.get().orders);
    let selected_signal = Signal::derive(move || selected.get());

    // RwSignals for Thaw components
    let selected_org_id = RwSignal::new(
        state.with_untracked(|s| s.selected_organization_id.clone().unwrap_or_default()),
    );

    Effect::new(move |_| {
        let val = selected_org_id.get();
        state.update(|s| {
            if val.is_empty() {
                s.selected_organization_id = None;
            } else {
                s.selected_organization_id = Some(val.clone());
            }
            s.page = 0;
        });
        load_orders();
    });

    let search_order_no = RwSignal::new(state.with_untracked(|s| s.search_order_no.clone()));

    Effect::new(move |_| {
        let val = search_order_no.get();
        state.update(|s| {
            s.search_order_no = val;
            s.page = 0;
        });
        load_orders();
    });

    let filter_status = RwSignal::new(state.with_untracked(|s| s.filter_status.clone()));

    Effect::new(move |_| {
        let val = filter_status.get();
        state.update(|s| {
            s.filter_status = val;
            s.page = 0;
        });
        load_orders();
    });

    // Initialize column resize
    Effect::new(move |_| {
        init_column_resize(TABLE_ID, COLUMN_WIDTHS_KEY);
    });

    view! {
        <div class="page">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Заказы Yandex Market"</h1>
                    <UiBadge variant="primary".to_string()>
                        {move || state.get().total_count.to_string()}
                    </UiBadge>
                </div>

                <div class="page__header-right">
                    <Space>
                        <Button
                            appearance=ButtonAppearance::Subtle
                            on_click=move |e| restore_settings(e)
                        >
                            {icon("refresh")}
                        </Button>
                        <Button
                            appearance=ButtonAppearance::Subtle
                            on_click=move |e| save_settings_to_db(e)
                        >
                            {icon("save")}
                        </Button>
                        {move || save_notification.get().map(|msg| view! {
                            <span style="font-size: 12px; color: var(--colorNeutralForeground2, #666);">{msg}</span>
                        })}
                    </Space>
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
                                page_size_options=vec![50, 100, 200, 500]
                            />
                        </div>

                        <div class="filter-panel-header__right">
                            <Space>
                                <Button
                                    appearance=ButtonAppearance::Primary
                                    disabled=Signal::derive(move || state.get().selected_ids.is_empty() || posting_in_progress.get())
                                    on_click=move |_| {
                                        let ids: Vec<String> = state.with(|s| s.selected_ids.iter().cloned().collect());
                                        if ids.is_empty() {
                                            return;
                                        }
                                        set_posting_in_progress.set(true);
                                        spawn_local(async move {
                                            let payload = json!({ "ids": ids });
                                            match Request::post(&format!("{}/api/a013/ym-order/batch-post", api_base()))
                                                .header("Content-Type", "application/json")
                                                .body(serde_json::to_string(&payload).unwrap_or_default())
                                                .unwrap()
                                                .send()
                                                .await
                                            {
                                                Ok(_) => {
                                                    state.update(|s| s.selected_ids.clear());
                                                    selected.update(|s| s.clear());
                                                    load_orders();
                                                }
                                                Err(e) => {
                                                    log!("Failed to post: {:?}", e);
                                                }
                                            }
                                            set_posting_in_progress.set(false);
                                        });
                                    }
                                >
                                    {move || format!("✓ Post ({})", state.get().selected_ids.len())}
                                </Button>
                                <Button
                                    appearance=ButtonAppearance::Secondary
                                    disabled=Signal::derive(move || state.get().selected_ids.is_empty() || posting_in_progress.get())
                                    on_click=move |_| {
                                        let ids: Vec<String> = state.with(|s| s.selected_ids.iter().cloned().collect());
                                        if ids.is_empty() {
                                            return;
                                        }
                                        set_posting_in_progress.set(true);
                                        spawn_local(async move {
                                            let payload = json!({ "ids": ids });
                                            match Request::post(&format!("{}/api/a013/ym-order/batch-unpost", api_base()))
                                                .header("Content-Type", "application/json")
                                                .body(serde_json::to_string(&payload).unwrap_or_default())
                                                .unwrap()
                                                .send()
                                                .await
                                            {
                                                Ok(_) => {
                                                    state.update(|s| s.selected_ids.clear());
                                                    selected.update(|s| s.clear());
                                                    load_orders();
                                                }
                                                Err(e) => {
                                                    log!("Failed to unpost: {:?}", e);
                                                }
                                            }
                                            set_posting_in_progress.set(false);
                                        });
                                    }
                                >
                                    {move || format!("✗ Unpost ({})", state.get().selected_ids.len())}
                                </Button>
                            </Space>
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
                                            load_orders();
                                        })
                                        label="Период:".to_string()
                                    />
                                </div>

                                <div style="width: 260px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Организация:"</Label>
                                        <Select value=selected_org_id>
                                            <option value="">"Все организации"</option>
                                            {move || organizations.get().into_iter().map(|org| {
                                                let id = org.id.clone();
                                                view! {
                                                    <option value=id>{org.description}</option>
                                                }
                                            }).collect_view()}
                                        </Select>
                                    </Flex>
                                </div>

                                <div style="flex: 1; max-width: 300px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Поиск (Order №):"</Label>
                                        <Input
                                            value=search_order_no
                                            placeholder="Номер заказа..."
                                        />
                                    </Flex>
                                </div>

                                <div style="width: 180px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>"Статус:"</Label>
                                        <Select value=filter_status>
                                            <option value="">"Все"</option>
                                            <option value="DELIVERED">"DELIVERED"</option>
                                            <option value="PROCESSING">"PROCESSING"</option>
                                            <option value="CANCELLED">"CANCELLED"</option>
                                            <option value="PARTIALLY_RETURNED">"PARTIALLY_RETURNED"</option>
                                        </Select>
                                    </Flex>
                                </div>

                                <div style="width: 120px;">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        <Label>" "</Label>
                                        <Button
                                            appearance=ButtonAppearance::Primary
                                            on_click=move |_| load_orders()
                                            disabled=Signal::derive(move || loading.get())
                                        >
                                            {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                                        </Button>
                                    </Flex>
                                </div>
                            </Flex>
                        </div>
                    </Show>
                </div>

                {move || {
                    error
                        .get()
                        .map(|err| {
                            view! {
                                <div class="alert alert--error">
                                    {err}
                                </div>
                            }
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
                                    get_id=Callback::new(|row: YmOrderListDto| row.id.clone())
                                    on_change=Callback::new(toggle_all)
                                />

                                <TableHeaderCell resizable=false min_width=140.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("document_no")>
                                        "Order №"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "document_no"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "document_no", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("creation_date")>
                                        "Дата заказа"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "creation_date"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "creation_date", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=110.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("delivery_date")>
                                        "Дата доставки"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "delivery_date"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "delivery_date", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=150.0 class="resizable">
                                    "Организация"
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=130.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("status_norm")>
                                        "Статус"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "status_norm"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "status_norm", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=80.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("is_error")>
                                        "Ошибка"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "is_error"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "is_error", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=70.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("lines_count")>
                                        "Строк"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "lines_count"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "lines_count", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=80.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("total_qty")>
                                        "Кол-во"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "total_qty"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "total_qty", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=100.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("total_amount")>
                                        "Сумма"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "total_amount"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "total_amount", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=100.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("delivery_total")>
                                        "Доставка"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "delivery_total"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "delivery_total", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>

                                <TableHeaderCell resizable=false min_width=100.0 class="resizable">
                                    <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("subsidies_total")>
                                        "Субсидии"
                                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "subsidies_total"))>
                                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "subsidies_total", state.with(|s| s.sort_ascending))}
                                        </span>
                                    </div>
                                </TableHeaderCell>
                            </TableRow>
                        </TableHeader>

                        <TableBody>
                            <For
                                each=move || state.get().orders
                                key=|item| item.id.clone()
                                children=move |order| {
                                    let order_id = order.id.clone();
                                    let order_id_for_link = order_id.clone();
                                    let document_no_for_link = order.document_no.clone();
                                    let document_no_text = order.document_no.clone();
                                    let formatted_creation_date = format_date(&order.creation_date);
                                    let formatted_delivery_date = format_date(&order.delivery_date);

                                    view! {
                                        <TableRow>
                                            <TableCellCheckbox
                                                item_id=order_id.clone()
                                                selected=selected_signal
                                                on_change=Callback::new(move |(id, checked)| toggle_selection(id, checked))
                                            />

                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    <a
                                                        href="#"
                                                        class="table__link"
                                                        style="color: #0f6cbd; text-decoration: underline;"
                                                        on:click=move |e| {
                                                            e.prevent_default();
                                                            open_detail(order_id_for_link.clone(), document_no_for_link.clone());
                                                        }
                                                    >
                                                        {document_no_text}
                                                    </a>
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout>
                                                    {formatted_creation_date}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout>
                                                    {formatted_delivery_date}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {order.organization_name.clone().unwrap_or_else(|| "—".to_string())}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {order.status_norm.clone()}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout>
                                                    {if order.is_error { "Да" } else { "—" }}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout>
                                                    {order.lines_count.to_string()}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCell>
                                                <TableCellLayout>
                                                    {format!("{:.0}", order.total_qty)}
                                                </TableCellLayout>
                                            </TableCell>

                                            <TableCellMoney
                                                value=order.total_amount
                                                show_currency=false
                                                color_by_sign=false
                                            />

                                            <TableCellMoney
                                                value=order.delivery_total.unwrap_or(0.0)
                                                show_currency=false
                                                color_by_sign=false
                                            />

                                            <TableCellMoney
                                                value=order.subsidies_total
                                                show_currency=false
                                                color_by_sign=false
                                            />
                                        </TableRow>
                                    }
                                }
                            />
                        </TableBody>
                    </Table>
                </div>
            </div>
        </div>
    }
}

async fn load_saved_settings(form_key: &str) -> Result<Option<serde_json::Value>, String> {
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("/api/form-settings/{}", form_key);
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

    // Response is Option<FormSettings>
    let response: Option<serde_json::Value> =
        serde_json::from_str(&text).map_err(|e| format!("{e}"))?;

    if let Some(form_settings) = response {
        if let Some(settings_json) = form_settings.get("settings_json").and_then(|v| v.as_str()) {
            let settings: serde_json::Value =
                serde_json::from_str(settings_json).map_err(|e| format!("{e}"))?;
            return Ok(Some(settings));
        }
    }

    Ok(None)
}

async fn save_settings_to_database(
    form_key: &str,
    settings: serde_json::Value,
) -> Result<(), String> {
    use wasm_bindgen::JsValue;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let request_body = json!({
        "form_key": form_key,
        "settings": settings,
    });

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let body_str = serde_json::to_string(&request_body).map_err(|e| format!("{e}"))?;
    opts.set_body(&JsValue::from_str(&body_str));

    let url = "/api/form-settings";
    let request = Request::new_with_str_and_init(url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Content-Type", "application/json")
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

    Ok(())
}

/// Загрузка списка организаций
async fn fetch_organizations() -> Result<Vec<Organization>, String> {
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request as WebRequest, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/organization", api_base());
    let request = WebRequest::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
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
    let data: Vec<Organization> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}
