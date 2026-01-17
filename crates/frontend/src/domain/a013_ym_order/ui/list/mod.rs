pub mod state;

use self::state::create_state;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::date_input::DateInput;
use crate::shared::components::month_selector::MonthSelector;
use crate::shared::list_utils::{
    format_number, format_number_int, get_sort_class, get_sort_indicator, Sortable,
};
use contracts::domain::a013_ym_order::aggregate::YmOrderListDto;
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::cmp::Ordering;
use wasm_bindgen::JsCast;

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

#[component]
pub fn YmOrderList() -> impl IntoView {
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");

    let state = create_state();
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);

    // Batch operation state
    let (posting_in_progress, set_posting_in_progress) = signal(false);
    let (_, set_operation_results) = signal::<Vec<(String, bool, Option<String>)>>(Vec::new());
    let (current_operation, set_current_operation) = signal::<Option<(usize, usize)>>(None);

    // Organizations
    let (organizations, set_organizations) = signal::<Vec<Organization>>(Vec::new());

    // State for save settings notification
    let (save_notification, set_save_notification) = signal(None::<String>);

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
                "http://localhost:3000/api/a013/ym-order/list?limit={}&offset={}&sort_by={}&sort_desc={}",
                page_size, offset, sort_field, !sort_ascending
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
                                            "Parsed paginated response: total={}",
                                            paginated.total
                                        );

                                        let total = paginated.total;
                                        let total_pages = if page_size > 0 {
                                            (total + page_size - 1) / page_size
                                        } else {
                                            0
                                        };

                                        state.update(|s| {
                                            s.orders = paginated.items;
                                            s.total_count = total;
                                            s.total_pages = total_pages;
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

    // Get items (sorting is now done on server) - no clone, returns reference via signal
    let get_items = move || -> Vec<YmOrderListDto> { state.with(|s| s.orders.clone()) };

    // Мемоизированные итоги - вычисляются только при изменении orders
    let totals = Memo::new(move |_| {
        state.with(|s| {
            let total_amount: f64 = s.orders.iter().map(|item| item.total_amount).sum();
            let total_qty: f64 = s.orders.iter().map(|item| item.total_qty).sum();
            let total_delivery: f64 = s.orders.iter().filter_map(|item| item.delivery_total).sum();
            let total_subsidies: f64 = s.orders.iter().map(|item| item.subsidies_total).sum();
            (
                s.orders.len(),
                total_amount,
                total_qty,
                total_delivery,
                total_subsidies,
            )
        })
    });

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

    // Переключение выбора одного документа
    let toggle_selection = move |id: String| {
        state.update(|s| {
            if s.selected_ids.contains(&id) {
                s.selected_ids.retain(|x| x != &id);
            } else {
                s.selected_ids.push(id.clone());
            }
        });
    };

    // Выбрать все / снять все
    let toggle_all = move |_| {
        let items = get_items();
        let all_ids: Vec<String> = items.iter().map(|item| item.id.clone()).collect();
        state.update(|s| {
            if s.selected_ids.len() == all_ids.len() && !all_ids.is_empty() {
                s.selected_ids.clear();
            } else {
                s.selected_ids = all_ids;
            }
        });
    };

    // Проверка, выбраны ли все
    let all_selected = move || {
        let items = get_items();
        let selected_len = state.with(|s| s.selected_ids.len());
        !items.is_empty() && selected_len == items.len()
    };

    // Проверка, выбран ли конкретный документ
    let is_selected = move |id: &str| state.with(|s| s.selected_ids.contains(&id.to_string()));

    // Массовое проведение
    let post_selected = move |_| {
        let ids = state.with(|s| s.selected_ids.clone());
        if ids.is_empty() {
            return;
        }

        set_posting_in_progress.set(true);
        set_operation_results.set(Vec::new());
        set_current_operation.set(Some((0, ids.len())));

        spawn_local(async move {
            let mut results = Vec::new();
            let total = ids.len();

            // Разбиваем на чанки по 100
            for (chunk_idx, chunk) in ids.chunks(100).enumerate() {
                set_current_operation.set(Some((chunk_idx * 100 + chunk.len(), total)));

                let payload = json!({ "ids": chunk });
                let response = Request::post("http://localhost:3000/api/a013/ym-order/batch-post")
                    .header("Content-Type", "application/json")
                    .body(serde_json::to_string(&payload).unwrap_or_default())
                    .map(|req| req.send());

                match response {
                    Ok(future) => match future.await {
                        Ok(resp) => {
                            if resp.status() == 200 {
                                for id in chunk {
                                    results.push((id.clone(), true, None));
                                }
                            } else {
                                for id in chunk {
                                    results.push((
                                        id.clone(),
                                        false,
                                        Some(format!("HTTP {}", resp.status())),
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            for id in chunk {
                                results.push((id.clone(), false, Some(format!("{:?}", e))));
                            }
                        }
                    },
                    Err(e) => {
                        for id in chunk {
                            results.push((id.clone(), false, Some(format!("{:?}", e))));
                        }
                    }
                }
            }

            set_operation_results.set(results);
            set_posting_in_progress.set(false);
            set_current_operation.set(None);
            state.update(|s| s.selected_ids.clear());

            // Перезагрузить список
            load_orders();
        });
    };

    // Массовая отмена проведения
    let unpost_selected = move |_| {
        let ids = state.with(|s| s.selected_ids.clone());
        if ids.is_empty() {
            return;
        }

        set_posting_in_progress.set(true);
        set_operation_results.set(Vec::new());
        set_current_operation.set(Some((0, ids.len())));

        spawn_local(async move {
            let mut results = Vec::new();
            let total = ids.len();

            // Разбиваем на чанки по 100
            for (chunk_idx, chunk) in ids.chunks(100).enumerate() {
                set_current_operation.set(Some((chunk_idx * 100 + chunk.len(), total)));

                let payload = json!({ "ids": chunk });
                let response =
                    Request::post("http://localhost:3000/api/a013/ym-order/batch-unpost")
                        .header("Content-Type", "application/json")
                        .body(serde_json::to_string(&payload).unwrap_or_default())
                        .map(|req| req.send());

                match response {
                    Ok(future) => match future.await {
                        Ok(resp) => {
                            if resp.status() == 200 {
                                for id in chunk {
                                    results.push((id.clone(), true, None));
                                }
                            } else {
                                for id in chunk {
                                    results.push((
                                        id.clone(),
                                        false,
                                        Some(format!("HTTP {}", resp.status())),
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            for id in chunk {
                                results.push((id.clone(), false, Some(format!("{:?}", e))));
                            }
                        }
                    },
                    Err(e) => {
                        for id in chunk {
                            results.push((id.clone(), false, Some(format!("{:?}", e))));
                        }
                    }
                }
            }

            set_operation_results.set(results);
            set_posting_in_progress.set(false);
            set_current_operation.set(None);
            state.update(|s| s.selected_ids.clear());

            // Перезагрузить список
            load_orders();
        });
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

    // Открыть детальный просмотр
    let open_detail = move |id: String, document_no: String| {
        tabs_store.open_tab(
            &format!("a013_ym_order_detail_{}", id),
            &format!("YM Order {}", document_no),
        );
    };

    view! {
        <div class="ym-order-list" style="background: #f8f9fa; padding: 12px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
            // Header - Row 1: Title with Pagination, Post/Unpost and Settings Buttons
            <div style="background: linear-gradient(135deg, #1976d2 0%, #0d47a1 100%); padding: 8px 12px; border-radius: 6px 6px 0 0; margin: -12px -12px 0 -12px; display: flex; align-items: center; justify-content: space-between;">
                <div style="display: flex; align-items: center; gap: 12px;">
                    <h2 style="margin: 0; font-size: 1.1rem; font-weight: 600; color: white; letter-spacing: 0.5px;">"📦 YM Orders"</h2>

                    // === PAGINATION CONTROLS ===
                    <div style="display: flex; align-items: center; gap: 6px; background: rgba(255,255,255,0.15); padding: 4px 10px; border-radius: 6px;">
                        // First page button
                        <button
                            style="background: none; border: none; color: white; cursor: pointer; padding: 4px 6px; border-radius: 4px; font-size: 12px; opacity: 0.9; transition: all 0.2s;"
                            prop:disabled=move || state.with(|s| s.page == 0) || loading.get()
                            on:click=move |_| go_to_page(0)
                            title="Первая страница"
                        >
                            "⏮"
                        </button>

                        // Previous page button
                        <button
                            style="background: none; border: none; color: white; cursor: pointer; padding: 4px 6px; border-radius: 4px; font-size: 12px; opacity: 0.9; transition: all 0.2s;"
                            prop:disabled=move || state.with(|s| s.page == 0) || loading.get()
                            on:click=move |_| {
                                let current = state.with(|s| s.page);
                                if current > 0 {
                                    go_to_page(current - 1);
                                }
                            }
                            title="Предыдущая страница"
                        >
                            "◀"
                        </button>

                        // Page info
                        <span style="color: white; font-size: 12px; font-weight: 500; min-width: 100px; text-align: center;">
                            {move || {
                                let page = state.with(|s| s.page);
                                let total_pages = state.with(|s| s.total_pages);
                                let total = state.with(|s| s.total_count);
                                format!("{} / {} ({})", page + 1, total_pages.max(1), total)
                            }}
                        </span>

                        // Next page button
                        <button
                            style="background: none; border: none; color: white; cursor: pointer; padding: 4px 6px; border-radius: 4px; font-size: 12px; opacity: 0.9; transition: all 0.2s;"
                            prop:disabled=move || state.with(|s| s.page >= s.total_pages.saturating_sub(1)) || loading.get()
                            on:click=move |_| {
                                let current = state.with(|s| s.page);
                                let max_page = state.with(|s| s.total_pages.saturating_sub(1));
                                if current < max_page {
                                    go_to_page(current + 1);
                                }
                            }
                            title="Следующая страница"
                        >
                            "▶"
                        </button>

                        // Last page button
                        <button
                            style="background: none; border: none; color: white; cursor: pointer; padding: 4px 6px; border-radius: 4px; font-size: 12px; opacity: 0.9; transition: all 0.2s;"
                            prop:disabled=move || state.with(|s| s.page >= s.total_pages.saturating_sub(1)) || loading.get()
                            on:click=move |_| {
                                let max_page = state.with(|s| s.total_pages.saturating_sub(1));
                                go_to_page(max_page);
                            }
                            title="Последняя страница"
                        >
                            "⏭"
                        </button>

                        // Divider
                        <div style="width: 1px; height: 18px; background: rgba(255,255,255,0.3); margin: 0 4px;"></div>

                        // Page size selector
                        <select
                            style="background: rgba(255,255,255,0.2); color: white; border: 1px solid rgba(255,255,255,0.3); border-radius: 4px; padding: 3px 6px; font-size: 11px; cursor: pointer;"
                            prop:value=move || state.with(|s| s.page_size.to_string())
                            on:change=move |ev| {
                                if let Ok(size) = event_target_value(&ev).parse::<usize>() {
                                    change_page_size(size);
                                }
                            }
                        >
                            <option value="50" style="color: black;">"50"</option>
                            <option value="100" style="color: black;">"100"</option>
                            <option value="200" style="color: black;">"200"</option>
                            <option value="500" style="color: black;">"500"</option>
                            <option value="10000" style="color: black;">"10000"</option>
                        </select>
                        <span style="color: rgba(255,255,255,0.8); font-size: 10px;">"на стр."</span>
                    </div>
                    // === END PAGINATION ===

                    // Post/Unpost buttons
                    <button
                        class="button button--primary"
                        prop:disabled=move || state.with(|s| s.selected_ids.is_empty()) || posting_in_progress.get()
                        on:click=post_selected
                    >
                        {move || format!("✓ Post ({})", state.with(|s| s.selected_ids.len()))}
                    </button>
                    <button
                        class="button button--warning"
                        prop:disabled=move || state.with(|s| s.selected_ids.is_empty()) || posting_in_progress.get()
                        on:click=unpost_selected
                    >
                        {move || format!("✗ Unpost ({})", state.with(|s| s.selected_ids.len()))}
                    </button>
                </div>

                <div style="display: flex; gap: 8px; align-items: center;">
                    {move || {
                        if let Some(msg) = save_notification.get() {
                            view! {
                                <span style="font-size: 0.75rem; color: white; font-weight: 500; margin-right: 8px;">{msg}</span>
                            }.into_any()
                        } else {
                            view! { <></> }.into_any()
                        }
                    }}
                    <button
                        class="button button--ghost button--small"
                        on:click=restore_settings
                        title="Восстановить настройки из базы данных"
                    >
                        "🔄"
                    </button>
                    <button
                        class="button button--ghost button--small"
                        on:click=save_settings_to_db
                        title="Сохранить настройки в базу данных"
                    >
                        "💾"
                    </button>
                </div>
            </div>

            // Header - Row 2: Filters and Actions - All in one row
            <div style="background: white; padding: 8px 12px; margin: 0 -12px 10px -12px; border-bottom: 1px solid #e9ecef; display: flex; align-items: center; gap: 12px; flex-wrap: wrap;">
                // Period section
                <div style="display: flex; align-items: center; gap: 8px;">
                    <label style="margin: 0; font-size: 0.875rem; font-weight: 500; color: #495057; white-space: nowrap;">"Период:"</label>
                    <DateInput
                        value=Signal::derive(move || state.get().date_from)
                        on_change=move |val| state.update(|s| s.date_from = val)
                    />
                    <span style="color: #6c757d;">"—"</span>
                    <DateInput
                        value=Signal::derive(move || state.get().date_to)
                        on_change=move |val| state.update(|s| s.date_to = val)
                    />
                    <MonthSelector
                        on_select=Callback::new(move |(from, to)| {
                            state.update(|s| {
                                s.date_from = from;
                                s.date_to = to;
                            });
                        })
                    />
                </div>

                // Organization filter
                <div style="display: flex; align-items: center; gap: 8px;">
                    <label style="margin: 0; font-size: 0.875rem; font-weight: 500; color: #495057; white-space: nowrap;">"Организация:"</label>
                    <select
                        prop:value=move || state.get().selected_organization_id.clone().unwrap_or_default()
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            state.update(|s| {
                                if value.is_empty() {
                                    s.selected_organization_id = None;
                                } else {
                                    s.selected_organization_id = Some(value);
                                }
                            });
                        }
                        style="padding: 6px 10px; border: 1px solid #ced4da; border-radius: 4px; font-size: 0.875rem; min-width: 200px; background: #fff;"
                    >
                        <option value="">"Все организации"</option>
                        {move || organizations.get().into_iter().map(|org| {
                            let org_id = org.id.clone();
                            let org_id_for_selected = org.id.clone();
                            let org_desc = org.description.clone();
                            view! {
                                <option value=org_id selected=move || {
                                    state.get().selected_organization_id.as_ref() == Some(&org_id_for_selected)
                                }>
                                    {org_desc}
                                </option>
                            }
                        }).collect_view()}
                    </select>
                </div>

                // Search by Order №
                <div style="display: flex; align-items: center; gap: 8px;">
                    <label style="margin: 0; font-size: 0.875rem; font-weight: 500; color: #495057; white-space: nowrap;">"Order №:"</label>
                    <input
                        type="text"
                        placeholder="Поиск..."
                        prop:value=move || state.get().search_order_no
                        on:input=move |ev| {
                            let value = event_target_value(&ev);
                            state.update(|s| s.search_order_no = value);
                        }
                        style="padding: 6px 10px; border: 1px solid #ced4da; border-radius: 4px; font-size: 0.875rem; width: 150px; background: #fff;"
                    />
                </div>

                // Status filter
                <div style="display: flex; align-items: center; gap: 8px;">
                    <label style="margin: 0; font-size: 0.875rem; font-weight: 500; color: #495057; white-space: nowrap;">"Статус:"</label>
                    <select
                        style="padding: 6px 10px; border: 1px solid #ced4da; border-radius: 4px; font-size: 0.875rem; min-width: 150px; background: #fff;"
                        prop:value=move || state.get().filter_status
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            state.update(|s| s.filter_status = value);
                        }
                    >
                        <option value="">"Все"</option>
                        <option value="DELIVERED">"DELIVERED"</option>
                        <option value="PROCESSING">"PROCESSING"</option>
                        <option value="CANCELLED">"CANCELLED"</option>
                        <option value="PARTIALLY_RETURNED">"PARTIALLY_RETURNED"</option>
                    </select>
                </div>

                // Update button
                <button
                    class="button button--primary"
                    on:click=move |_| {
                        load_orders();
                    }
                    prop:disabled=move || loading.get()
                >
                    "↻ Обновить"
                </button>
            </div>

            // Totals display (for current page)
            {move || if !loading.get() {
                let (count, total_amount, total_qty, total_delivery, total_subsidies) = totals.get();
                let total_count = state.with(|s| s.total_count);
                view! {
                    <div style="margin-bottom: 10px; padding: 3px 12px; background: var(--color-background-alt, #f5f5f5); border-radius: 4px; display: flex; align-items: center; flex-wrap: wrap;">
                        <span style="font-size: 0.875rem; font-weight: 600; color: var(--color-text);">
                            "На странице: " {format_number_int(count as f64)} " из " {format_number_int(total_count as f64)} " | "
                            "Сумма: " {format_number(total_amount)} " | "
                            "Кол-во: " {format_number(total_qty)} " | "
                            "Доставка: " {format_number(total_delivery)} " | "
                            "Субсидии: " {format_number(total_subsidies)}
                        </span>
                    </div>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }}

            // Selection summary panel (shows when items are selected)
            {move || {
                let selected_count = state.with(|s| s.selected_ids.len());
                let is_processing = current_operation.get().is_some();

                if selected_count > 0 || is_processing {
                    let selected_totals = move || {
                        let sel_ids = state.with(|s| s.selected_ids.clone());
                        let all_items = get_items();
                        let selected_items: Vec<_> = all_items.into_iter()
                            .filter(|item| sel_ids.contains(&item.id))
                            .collect();

                        let count = selected_items.len();
                        let total_qty: f64 = selected_items.iter().map(|s| s.total_qty).sum();
                        let total_amount: f64 = selected_items.iter().map(|s| s.total_amount).sum();
                        let total_delivery: f64 = selected_items.iter().filter_map(|s| s.delivery_total).sum();
                        let total_subsidies: f64 = selected_items.iter().map(|s| s.subsidies_total).sum();

                        (count, total_qty, total_amount, total_delivery, total_subsidies)
                    };

                    let (sel_count, sel_qty, sel_amount, sel_delivery, sel_subsidies) = selected_totals();

                    let progress_percent = if let Some((processed, total)) = current_operation.get() {
                        if total > 0 {
                            (processed as f64 / total as f64 * 100.0) as i32
                        } else {
                            0
                        }
                    } else {
                        0
                    };

                    let background_style = if is_processing {
                        if progress_percent == 0 {
                            "background: #ffffff; border: 1px solid #4CAF50; border-radius: 4px; padding: 8px 12px; margin-bottom: 8px;".to_string()
                        } else {
                            format!("background: linear-gradient(to right, #c8e6c9 {}%, #ffffff {}%); border: 1px solid #4CAF50; border-radius: 4px; padding: 8px 12px; margin-bottom: 8px;", progress_percent, progress_percent)
                        }
                    } else {
                        "background: #c8e6c9; border: 1px solid #4CAF50; border-radius: 4px; padding: 8px 12px; margin-bottom: 8px;".to_string()
                    };

                    view! {
                        <div style=background_style>
                            <div style="display: flex; align-items: center; gap: 10px; flex-wrap: wrap;">
                                <span style="font-weight: 600; color: #2e7d32; font-size: 0.875rem;">
                                    "Выделено: " {sel_count} " строк"
                                </span>
                                <span style="font-size: 0.875rem; color: #424242;">
                                    "Кол-во: " {format_number(sel_qty)} " | "
                                    "Сумма: " {format_number(sel_amount)} " | "
                                    "Доставка: " {format_number(sel_delivery)} " | "
                                    "Субсидии: " {format_number(sel_subsidies)}
                                </span>
                                <div style="margin-left: auto;">
                                    <button
                                        class="button button--secondary"
                                        on:click=move |_| state.update(|s| s.selected_ids.clear())
                                        prop:disabled=move || state.with(|s| s.selected_ids.is_empty()) || posting_in_progress.get()
                                    >
                                        "✕ Clear"
                                    </button>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}

            {move || error.get().map(|err| view! {
                <div class="error-message" style="padding: 12px; background: #ffebee; border: 1px solid #ffcdd2; border-radius: 4px; color: #c62828; margin-bottom: 10px;">{err}</div>
            })}

            {move || {
                if loading.get() {
                    view! {
                        <div class="loading-spinner" style="text-align: center; padding: 40px;">"Загрузка заказов..."</div>
                    }.into_any()
                } else {
                    let items = get_items();
                    let current_sort_field = state.with(|s| s.sort_field.clone());
                    let current_sort_asc = state.with(|s| s.sort_ascending);

                    view! {
                        <div class="table-container" style="overflow: auto; max-height: calc(100vh - 240px); position: relative;">
                            <table id="ym-orders-table" class="table__data table--striped" style="min-width: 1400px; table-layout: fixed;">
                                <thead>
                                    <tr>
                                        <th class="table__cell--checkbox" style="width: 40px; min-width: 40px;">
                                            <input
                                                type="checkbox"
                                                on:change=toggle_all
                                                prop:checked=move || all_selected()
                                            />
                                        </th>
                                        <th style="width: 130px; min-width: 80px; cursor: pointer;" on:click=move |_| toggle_sort("document_no")>
                                            <span class="table__sortable-header">"Order №" <span class={get_sort_class("document_no", &current_sort_field)}>{get_sort_indicator("document_no", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th style="width: 100px; min-width: 80px; cursor: pointer;" on:click=move |_| toggle_sort("creation_date")>
                                            <span class="table__sortable-header">"Дата заказа" <span class={get_sort_class("creation_date", &current_sort_field)}>{get_sort_indicator("creation_date", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th style="width: 100px; min-width: 80px; cursor: pointer;" on:click=move |_| toggle_sort("delivery_date")>
                                            <span class="table__sortable-header">"Дата доставки" <span class={get_sort_class("delivery_date", &current_sort_field)}>{get_sort_indicator("delivery_date", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th style="width: 130px; min-width: 100px; cursor: pointer;" on:click=move |_| toggle_sort("status_norm")>
                                            <span class="table__sortable-header">"Статус" <span class={get_sort_class("status_norm", &current_sort_field)}>{get_sort_indicator("status_norm", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th style="width: 70px; min-width: 60px; text-align: center; cursor: pointer;" on:click=move |_| toggle_sort("is_error")>
                                            <span class="table__sortable-header" style="justify-content: center;">"Ошибка" <span class={get_sort_class("is_error", &current_sort_field)}>{get_sort_indicator("is_error", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th style="width: 60px; min-width: 50px; text-align: right; cursor: pointer;" on:click=move |_| toggle_sort("lines_count")>
                                            <span class="table__sortable-header">"Строк" <span class={get_sort_class("lines_count", &current_sort_field)}>{get_sort_indicator("lines_count", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th style="width: 70px; min-width: 60px; text-align: right; cursor: pointer;" on:click=move |_| toggle_sort("total_qty")>
                                            <span class="table__sortable-header">"Кол-во" <span class={get_sort_class("total_qty", &current_sort_field)}>{get_sort_indicator("total_qty", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th style="width: 100px; min-width: 80px; text-align: right; cursor: pointer;" on:click=move |_| toggle_sort("total_amount")>
                                            <span class="table__sortable-header">"Сумма" <span class={get_sort_class("total_amount", &current_sort_field)}>{get_sort_indicator("total_amount", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th style="width: 100px; min-width: 80px; text-align: right; cursor: pointer;" on:click=move |_| toggle_sort("delivery_total")>
                                            <span class="table__sortable-header">"Доставка" <span class={get_sort_class("delivery_total", &current_sort_field)}>{get_sort_indicator("delivery_total", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                        <th style="width: 100px; min-width: 80px; text-align: right; cursor: pointer;" on:click=move |_| toggle_sort("subsidies_total")>
                                            <span class="table__sortable-header">"Субсидии" <span class={get_sort_class("subsidies_total", &current_sort_field)}>{get_sort_indicator("subsidies_total", &current_sort_field, current_sort_asc)}</span></span>
                                        </th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {items.into_iter().map(|item| {
                                        // Pre-compute all values once
                                        let id = item.id.clone();
                                        let doc_no = item.document_no.clone();
                                        let creation_date = if !item.creation_date.is_empty() {
                                            format_date(&item.creation_date)
                                        } else {
                                            "—".to_string()
                                        };
                                        let delivery_date = if !item.delivery_date.is_empty() {
                                            format_date(&item.delivery_date)
                                        } else {
                                            "—".to_string()
                                        };
                                        let status = item.status_norm.clone();
                                        let lines_count = item.lines_count;
                                        let qty = format!("{:.0}", item.total_qty);
                                        let amount = format!("{:.2}", item.total_amount);
                                        let delivery = item.delivery_total.map(|d| format!("{:.2}", d)).unwrap_or_else(|| "—".to_string());
                                        let subsidies = if item.subsidies_total > 0.0 {
                                            format!("{:.2}", item.subsidies_total)
                                        } else {
                                            "—".to_string()
                                        };

                                        let is_posted_flag = item.is_posted;
                                        let is_error_flag = item.is_error;

                                        let status_style = match status.as_str() {
                                            "DELIVERED" => "background: #e8f5e9; color: #2e7d32;",
                                            "CANCELLED" => "background: #ffebee; color: #c62828;",
                                            "PROCESSING" => "background: #fff3e0; color: #e65100;",
                                            "PARTIALLY_RETURNED" => "background: #e3f2fd; color: #1565c0;",
                                            _ => "background: #f5f5f5; color: #666;",
                                        };

                                        // Clone once for closures
                                        let id_check = id.clone();
                                        let id_toggle = id.clone();
                                        let id_row = id.clone();
                                        let doc_row = doc_no.clone();

                                        // Single click handler for entire row
                                        let on_row_click = move |_| {
                                            open_detail(id_row.clone(), doc_row.clone());
                                        };

                                        // Row style based on posted/error flags
                                        let row_style = if is_error_flag {
                                            "background: #ffebee;"
                                        } else if is_posted_flag {
                                            "background: #e8f5e9;"
                                        } else {
                                            ""
                                        };

                                        view! {
                                            <tr on:click=on_row_click.clone() style=row_style>
                                                <td class="table__cell--checkbox" on:click=move |e| e.stop_propagation()>
                                                    <input
                                                        type="checkbox"
                                                        prop:checked=move || is_selected(&id_check)
                                                        on:change=move |_| toggle_selection(id_toggle.clone())
                                                    />
                                                </td>
                                                <td class="cell-truncate" style="color: #1976d2; font-weight: 600;">{doc_no}</td>
                                                <td>{creation_date}</td>
                                                <td style="color: #c62828; font-weight: 500;">{delivery_date}</td>
                                                <td>
                                                    <span style={format!("padding: 3px 10px; border-radius: 4px; font-size: 0.85em; font-weight: 500; {}", status_style)}>
                                                        {status}
                                                    </span>
                                                </td>
                                                <td class="text-center">
                                                    {if is_error_flag {
                                                        view! {
                                                            <span style="padding: 3px 8px; border-radius: 4px; font-size: 0.85em; font-weight: 500; background: #ffebee; color: #c62828;">
                                                                "Да"
                                                            </span>
                                                        }.into_any()
                                                    } else {
                                                        view! {
                                                            <span style="color: #999;">
                                                                "—"
                                                            </span>
                                                        }.into_any()
                                                    }}
                                                </td>
                                                <td class="text-right">{lines_count}</td>
                                                <td class="text-right">{qty}</td>
                                                <td class="text-right" style="font-weight: 500;">{amount}</td>
                                                <td class="text-right" style="color: #0288d1;">{delivery}</td>
                                                <td class="text-right" style="color: #7b1fa2;">{subsidies}</td>
                                            </tr>
                                        }
                                    }).collect::<Vec<_>>()}
                                </tbody>
                            </table>
                        </div>
                    }.into_any()
                }
            }}
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

    let url = "http://localhost:3000/api/organization";
    let request = WebRequest::new_with_str_and_init(url, &opts).map_err(|e| format!("{e:?}"))?;
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
