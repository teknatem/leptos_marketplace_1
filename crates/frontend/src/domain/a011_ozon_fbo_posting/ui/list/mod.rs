use leptos::prelude::*;
use leptos::logging::log;
use serde::{Deserialize, Serialize};
use gloo_net::http::Request;
use super::details::OzonFboPostingDetail;
use crate::shared::list_utils::{get_sort_indicator, Sortable};
use std::cmp::Ordering;

/// Форматирует ISO 8601 дату в dd.mm.yyyy HH:MM
fn format_datetime(iso_date: &str) -> String {
    // Парсим ISO 8601: "2025-11-05T16:52:58.585775200Z"
    if let Some(date_part) = iso_date.split('T').next() {
        if let Some((year, rest)) = date_part.split_once('-') {
            if let Some((month, day)) = rest.split_once('-') {
                // Извлекаем время
                if let Some(time_part) = iso_date.split('T').nth(1) {
                    if let Some((hour_min, _)) = time_part.split_once(':') {
                        if let Some((hour, min)) = hour_min.split_once(':') {
                            return format!("{}.{}.{} {}:{}", day, month, year, hour, min);
                        } else {
                            // Если только час
                            let parts: Vec<&str> = time_part.split(':').collect();
                            if parts.len() >= 2 {
                                return format!("{}.{}.{} {}:{}", day, month, year, parts[0], parts[1]);
                            }
                        }
                    }
                }
                return format!("{}.{}.{}", day, month, year);
            }
        }
    }
    iso_date.to_string() // fallback
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFboPostingDto {
    pub id: String,
    pub code: String,
    pub description: String,
    pub document_no: String,
    pub status_norm: String, // Статус постинга (DELIVERED, CANCELLED и т.д.)
    pub substatus_raw: Option<String>, // Подстатус
    pub created_at_source: Option<String>, // Дата создания заказа в источнике
    pub total_amount: f64,
    pub line_count: usize,
    pub is_posted: bool, // Флаг проведения
}

impl Sortable for OzonFboPostingDto {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "document_no" => self.document_no.to_lowercase().cmp(&other.document_no.to_lowercase()),
            "status_norm" => self.status_norm.to_lowercase().cmp(&other.status_norm.to_lowercase()),
            "substatus_raw" => {
                // Сортировка с учетом None (None идут в конец)
                match (&self.substatus_raw, &other.substatus_raw) {
                    (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
                    (Some(_), None) => Ordering::Less,
                    (None, Some(_)) => Ordering::Greater,
                    (None, None) => Ordering::Equal,
                }
            }
            "created_at_source" => {
                match (&self.created_at_source, &other.created_at_source) {
                    (Some(a), Some(b)) => a.cmp(b),
                    (Some(_), None) => Ordering::Less,
                    (None, Some(_)) => Ordering::Greater,
                    (None, None) => Ordering::Equal,
                }
            },
            "total_amount" => self.total_amount.partial_cmp(&other.total_amount).unwrap_or(Ordering::Equal),
            "line_count" => self.line_count.cmp(&other.line_count),
            "description" => self.description.to_lowercase().cmp(&other.description.to_lowercase()),
            "is_posted" => self.is_posted.cmp(&other.is_posted),
            _ => Ordering::Equal,
        }
    }
}

#[component]
pub fn OzonFboPostingList() -> impl IntoView {
    let (postings, set_postings) = signal::<Vec<OzonFboPostingDto>>(Vec::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (selected_id, set_selected_id) = signal::<Option<String>>(None);
    let (detail_reload_trigger, set_detail_reload_trigger) = signal::<u32>(0);

    // Сортировка
    let (sort_field, set_sort_field) = signal::<String>("document_no".to_string());
    let (sort_ascending, set_sort_ascending) = signal(true);

    // Множественный выбор
    let (selected_ids, set_selected_ids) = signal::<Vec<String>>(Vec::new());

    // Фильтр по статусу
    let (status_filter, set_status_filter) = signal::<Option<String>>(None);

    // Статус массовых операций
    let (posting_in_progress, set_posting_in_progress) = signal(false);
    let (operation_results, set_operation_results) = signal::<Vec<(String, bool, Option<String>)>>(Vec::new());
    let (current_operation, set_current_operation) = signal::<Option<(usize, usize)>>(None); // (current, total)

    let load_postings = move || {
        let set_postings = set_postings.clone();
        let set_loading = set_loading.clone();
        let set_error = set_error.clone();
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let url = "http://localhost:3000/api/a011/ozon-fbo-posting";

            match Request::get(url).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status == 200 {
                        match response.text().await {
                            Ok(text) => {
                                log!("Received response text (first 500 chars): {}",
                                    text.chars().take(500).collect::<String>());

                                match serde_json::from_str::<Vec<serde_json::Value>>(&text) {
                                    Ok(data) => {
                                        let total_count = data.len();
                                        log!("Parsed {} items from JSON", total_count);

                                        // Упрощенная десериализация - берем только нужные поля
                                        let items: Vec<OzonFboPostingDto> = data
                                            .into_iter()
                                            .enumerate()
                                            .filter_map(|(idx, v)| {
                                                // Поля из base сериализуются на верхнем уровне благодаря #[serde(flatten)]

                                                // Статус (status_norm из state)
                                                let status_norm = v
                                                    .get("state")
                                                    .and_then(|s| s.get("status_norm"))
                                                    .and_then(|d| d.as_str())
                                                    .map(|s| s.to_string())
                                                    .unwrap_or_else(|| "UNKNOWN".to_string());

                                                // Подстатус (substatus_raw из state)
                                                let substatus_raw = v
                                                    .get("state")
                                                    .and_then(|s| s.get("substatus_raw"))
                                                    .and_then(|d| d.as_str())
                                                    .map(|s| s.to_string());

                                                // Дата создания заказа (created_at из state)
                                                let created_at_source = v
                                                    .get("state")
                                                    .and_then(|s| s.get("created_at"))
                                                    .and_then(|d| d.as_str())
                                                    .map(|s| s.to_string());

                                                let lines = v.get("lines")?.as_array()?;
                                                let line_count = lines.len();
                                                let total_amount: f64 = lines
                                                    .iter()
                                                    .filter_map(|line| line.get("amount_line")?.as_f64())
                                                    .sum();

                                                let is_posted = v.get("is_posted")
                                                    .and_then(|p| p.as_bool())
                                                    .unwrap_or(false);

                                                let result = Some(OzonFboPostingDto {
                                                    id: v.get("id")?.as_str()?.to_string(),
                                                    code: v.get("code")?.as_str()?.to_string(),
                                                    description: v.get("description")?.as_str()?.to_string(),
                                                    document_no: v
                                                        .get("header")?
                                                        .get("document_no")?
                                                        .as_str()?
                                                        .to_string(),
                                                    status_norm,
                                                    substatus_raw,
                                                    created_at_source,
                                                    total_amount,
                                                    line_count,
                                                    is_posted,
                                                });

                                                if result.is_none() {
                                                    log!("Failed to parse item {}", idx);
                                                }

                                                result
                                            })
                                            .collect();

                                        log!("Successfully parsed {} postings out of {}", items.len(), total_count);
                                        set_postings.set(items);
                                        set_loading.set(false);
                                    }
                                    Err(e) => {
                                        log!("Failed to parse response: {:?}", e);
                                        set_error.set(Some(format!("Failed to parse response: {}", e)));
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
                    log!("Failed to fetch postings: {:?}", e);
                    set_error.set(Some(format!("Failed to fetch postings: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    // Функция для получения отфильтрованных и отсортированных данных
    let get_filtered_sorted_items = move || -> Vec<OzonFboPostingDto> {
        let mut result = postings.get();

        // Фильтр по статусу
        if let Some(ref status) = status_filter.get() {
            if !status.is_empty() {
                result.retain(|item| &item.status_norm == status);
            }
        }

        // Сортировка
        let field = sort_field.get();
        let ascending = sort_ascending.get();
        result.sort_by(|a, b| {
            let cmp = a.compare_by_field(b, &field);
            if ascending { cmp } else { cmp.reverse() }
        });

        result
    };

    // Вычисление итогов по сумме и количеству позиций
    let get_totals = move || -> (f64, usize) {
        let items = get_filtered_sorted_items();
        let total_sum: f64 = items.iter().map(|p| p.total_amount).sum();
        let total_qty: usize = items.iter().map(|p| p.line_count).sum();
        (total_sum, total_qty)
    };

    // Обработчик переключения сортировки
    let toggle_sort = move |field: &'static str| {
        move |_| {
            if sort_field.get() == field {
                set_sort_ascending.update(|v| *v = !*v);
            } else {
                set_sort_field.set(field.to_string());
                set_sort_ascending.set(true);
            }
        }
    };

    // Переключение выбора одного документа
    let toggle_selection = move |id: String| {
        set_selected_ids.update(|ids| {
            if ids.contains(&id) {
                ids.retain(|x| x != &id);
            } else {
                ids.push(id);
            }
        });
    };

    // Выбрать все / снять все
    let toggle_all = move |_| {
        let items = get_filtered_sorted_items();
        let selected = selected_ids.get();
        if selected.len() == items.len() && !items.is_empty() {
            set_selected_ids.set(Vec::new()); // Снять все
        } else {
            set_selected_ids.set(items.iter().map(|item| item.id.clone()).collect()); // Выбрать все
        }
    };

    // Проверка, выбраны ли все
    let all_selected = move || {
        let items = get_filtered_sorted_items();
        let selected = selected_ids.get();
        !items.is_empty() && selected.len() == items.len()
    };

    // Проверка, выбран ли конкретный документ
    let is_selected = move |id: &str| {
        selected_ids.get().contains(&id.to_string())
    };

    // Массовое проведение
    let post_selected = move |_| {
        let ids = selected_ids.get();
        if ids.is_empty() {
            return;
        }

        set_posting_in_progress.set(true);
        set_operation_results.set(Vec::new());
        set_current_operation.set(Some((0, ids.len())));

        let set_postings = set_postings.clone();
        let set_loading = set_loading.clone();
        let set_error = set_error.clone();
        let set_posting_in_progress = set_posting_in_progress.clone();
        let set_operation_results = set_operation_results.clone();
        let set_selected_ids = set_selected_ids.clone();
        let set_detail_reload_trigger = set_detail_reload_trigger.clone();
        let set_current_operation = set_current_operation.clone();

        wasm_bindgen_futures::spawn_local(async move {
            let mut results = Vec::new();
            let total = ids.len();

            for (index, id) in ids.iter().enumerate() {
                set_current_operation.set(Some((index + 1, total)));
                let url = format!("http://localhost:3000/api/a011/ozon-fbo-posting/{}/post", id);
                match Request::post(&url).send().await {
                    Ok(response) => {
                        if response.status() == 200 {
                            results.push((id.clone(), true, None));
                        } else {
                            results.push((id.clone(), false, Some(format!("HTTP {}", response.status()))));
                        }
                    }
                    Err(e) => {
                        results.push((id.clone(), false, Some(format!("{:?}", e))));
                    }
                }
            }

            set_operation_results.set(results);
            set_posting_in_progress.set(false);
            set_current_operation.set(None);
            set_selected_ids.set(Vec::new());

            // Перезагрузить список
            set_loading.set(true);
            set_error.set(None);

            let url = "http://localhost:3000/api/a011/ozon-fbo-posting";
            match Request::get(url).send().await {
                Ok(response) => {
                    if response.status() == 200 {
                        if let Ok(text) = response.text().await {
                            if let Ok(data) = serde_json::from_str::<Vec<serde_json::Value>>(&text) {
                                let items: Vec<OzonFboPostingDto> = data
                                    .into_iter()
                                    .filter_map(|v| {
                                        let status_norm = v
                                            .get("state")
                                            .and_then(|s| s.get("status_norm"))
                                            .and_then(|d| d.as_str())
                                            .map(|s| s.to_string())
                                            .unwrap_or_else(|| "UNKNOWN".to_string());

                                        let substatus_raw = v
                                            .get("state")
                                            .and_then(|s| s.get("substatus_raw"))
                                            .and_then(|d| d.as_str())
                                            .map(|s| s.to_string());

                                        let created_at_source = v
                                            .get("state")
                                            .and_then(|s| s.get("created_at"))
                                            .and_then(|d| d.as_str())
                                            .map(|s| s.to_string());

                                        let lines = v.get("lines")?.as_array()?;
                                        let line_count = lines.len();
                                        let total_amount: f64 = lines
                                            .iter()
                                            .filter_map(|line| line.get("amount_line")?.as_f64())
                                            .sum();

                                        let is_posted = v.get("is_posted")
                                            .and_then(|p| p.as_bool())
                                            .unwrap_or(false);

                                        Some(OzonFboPostingDto {
                                            id: v.get("id")?.as_str()?.to_string(),
                                            code: v.get("code")?.as_str()?.to_string(),
                                            description: v.get("description")?.as_str()?.to_string(),
                                            document_no: v
                                                .get("header")?
                                                .get("document_no")?
                                                .as_str()?
                                                .to_string(),
                                            status_norm,
                                            substatus_raw,
                                            created_at_source,
                                            total_amount,
                                            line_count,
                                            is_posted,
                                        })
                                    })
                                    .collect();
                                set_postings.set(items);
                            }
                        }
                    }
                }
                Err(_) => {}
            }
            set_loading.set(false);

            // Инкрементируем триггер для перезагрузки детальной формы
            set_detail_reload_trigger.update(|v| *v += 1);
        });
    };

    // Массовая отмена проведения
    let unpost_selected = move |_| {
        let ids = selected_ids.get();
        if ids.is_empty() {
            return;
        }

        set_posting_in_progress.set(true);
        set_operation_results.set(Vec::new());
        set_current_operation.set(Some((0, ids.len())));

        let set_postings = set_postings.clone();
        let set_loading = set_loading.clone();
        let set_error = set_error.clone();
        let set_posting_in_progress = set_posting_in_progress.clone();
        let set_operation_results = set_operation_results.clone();
        let set_selected_ids = set_selected_ids.clone();
        let set_detail_reload_trigger = set_detail_reload_trigger.clone();
        let set_current_operation = set_current_operation.clone();

        wasm_bindgen_futures::spawn_local(async move {
            let mut results = Vec::new();
            let total = ids.len();

            for (index, id) in ids.iter().enumerate() {
                set_current_operation.set(Some((index + 1, total)));
                let url = format!("http://localhost:3000/api/a011/ozon-fbo-posting/{}/unpost", id);
                match Request::post(&url).send().await {
                    Ok(response) => {
                        if response.status() == 200 {
                            results.push((id.clone(), true, None));
                        } else {
                            results.push((id.clone(), false, Some(format!("HTTP {}", response.status()))));
                        }
                    }
                    Err(e) => {
                        results.push((id.clone(), false, Some(format!("{:?}", e))));
                    }
                }
            }

            set_operation_results.set(results);
            set_posting_in_progress.set(false);
            set_current_operation.set(None);
            set_selected_ids.set(Vec::new());

            // Перезагрузить список
            set_loading.set(true);
            set_error.set(None);

            let url = "http://localhost:3000/api/a011/ozon-fbo-posting";
            match Request::get(url).send().await {
                Ok(response) => {
                    if response.status() == 200 {
                        if let Ok(text) = response.text().await {
                            if let Ok(data) = serde_json::from_str::<Vec<serde_json::Value>>(&text) {
                                let items: Vec<OzonFboPostingDto> = data
                                    .into_iter()
                                    .filter_map(|v| {
                                        let status_norm = v
                                            .get("state")
                                            .and_then(|s| s.get("status_norm"))
                                            .and_then(|d| d.as_str())
                                            .map(|s| s.to_string())
                                            .unwrap_or_else(|| "UNKNOWN".to_string());

                                        let substatus_raw = v
                                            .get("state")
                                            .and_then(|s| s.get("substatus_raw"))
                                            .and_then(|d| d.as_str())
                                            .map(|s| s.to_string());

                                        let created_at_source = v
                                            .get("state")
                                            .and_then(|s| s.get("created_at"))
                                            .and_then(|d| d.as_str())
                                            .map(|s| s.to_string());

                                        let lines = v.get("lines")?.as_array()?;
                                        let line_count = lines.len();
                                        let total_amount: f64 = lines
                                            .iter()
                                            .filter_map(|line| line.get("amount_line")?.as_f64())
                                            .sum();

                                        let is_posted = v.get("is_posted")
                                            .and_then(|p| p.as_bool())
                                            .unwrap_or(false);

                                        Some(OzonFboPostingDto {
                                            id: v.get("id")?.as_str()?.to_string(),
                                            code: v.get("code")?.as_str()?.to_string(),
                                            description: v.get("description")?.as_str()?.to_string(),
                                            document_no: v
                                                .get("header")?
                                                .get("document_no")?
                                                .as_str()?
                                                .to_string(),
                                            status_norm,
                                            substatus_raw,
                                            created_at_source,
                                            total_amount,
                                            line_count,
                                            is_posted,
                                        })
                                    })
                                    .collect();
                                set_postings.set(items);
                            }
                        }
                    }
                }
                Err(_) => {}
            }
            set_loading.set(false);

            // Инкрементируем триггер для перезагрузки детальной формы
            set_detail_reload_trigger.update(|v| *v += 1);
        });
    };

    // Автоматическая загрузка при открытии
    load_postings();

    view! {
        <div class="ozon-fbo-posting-list">
            {move || {
                if let Some(id) = selected_id.get() {
                    view! {
                        <div class="modal-overlay" style="align-items: flex-start; padding-top: 40px;">
                            <div class="modal-content" style="max-width: 1200px; height: calc(100vh - 80px); overflow: hidden; margin: 0;">
                                <OzonFboPostingDetail
                                    id=id
                                    on_close=move || set_selected_id.set(None)
                                    reload_trigger=detail_reload_trigger
                                />
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div>
                            <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px;">
                                <h2 style="margin: 0;">"OZON FBO Posting (A011)"</h2>
                                <button
                                    on:click=move |_| {
                                        load_postings();
                                    }
                                    style="padding: 6px 12px; background: #4CAF50; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 14px;"
                                >
                                    "Обновить"
                                </button>
                            </div>

                            // Панель фильтров и массовых операций
                            <div style="margin-bottom: 20px; padding: 15px; background: #f9f9f9; border-radius: 4px;">
                                <div style="display: flex; gap: 15px; align-items: center; flex-wrap: wrap;">
                                    // Фильтр по статусу
                                    <div style="display: flex; gap: 10px; align-items: center;">
                                        <label style="font-weight: 500;">"Статус:"</label>
                                        <select
                                            on:change=move |e| {
                                                let value = event_target_value(&e);
                                                set_status_filter.set(if value.is_empty() { None } else { Some(value) });
                                            }
                                            style="padding: 4px 8px; border: 1px solid #ddd; border-radius: 4px;"
                                        >
                                            <option value="">"Все"</option>
                                            <option value="DELIVERED">"DELIVERED"</option>
                                            <option value="CANCELLED">"CANCELLED"</option>
                                            <option value="DELIVERING">"DELIVERING"</option>
                                            <option value="AWAITING_DELIVER">"AWAITING_DELIVER"</option>
                                        </select>
                                    </div>

                                    // Кнопки массовых операций
                                    <div style="display: flex; gap: 10px;">
                                        <button
                                            disabled=move || selected_ids.get().is_empty() || posting_in_progress.get()
                                            on:click=post_selected
                                            style="padding: 6px 12px; background: #2196F3; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 14px;"
                                            style:opacity=move || if selected_ids.get().is_empty() || posting_in_progress.get() { "0.5" } else { "1" }
                                            style:cursor=move || if selected_ids.get().is_empty() || posting_in_progress.get() { "not-allowed" } else { "pointer" }
                                        >
                                            {move || format!("Post ({})", selected_ids.get().len())}
                                        </button>
                                        <button
                                            disabled=move || selected_ids.get().is_empty() || posting_in_progress.get()
                                            on:click=unpost_selected
                                            style="padding: 6px 12px; background: #FF9800; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 14px;"
                                            style:opacity=move || if selected_ids.get().is_empty() || posting_in_progress.get() { "0.5" } else { "1" }
                                            style:cursor=move || if selected_ids.get().is_empty() || posting_in_progress.get() { "not-allowed" } else { "pointer" }
                                        >
                                            {move || format!("Unpost ({})", selected_ids.get().len())}
                                        </button>
                                    </div>

                                    // Индикатор прогресса
                                    <Show when=move || posting_in_progress.get()>
                                        {move || {
                                            if let Some((current, total)) = current_operation.get() {
                                                view! {
                                                    <span style="color: #666; font-style: italic;">
                                                        {format!("Обработка {}/{} документов...", current, total)}
                                                    </span>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <span style="color: #666; font-style: italic;">"Обработка..."</span>
                                                }.into_any()
                                            }
                                        }}
                                    </Show>
                                </div>

                                // Строка с итогами
                                {move || if !loading.get() && error.get().is_none() {
                                    let filtered = get_filtered_sorted_items();
                                    let (total_sum, total_qty) = get_totals();
                                    view! {
                                        <div style="margin-top: 10px; padding-top: 10px; border-top: 1px solid #ddd; font-size: 13px; color: #666;">
                                            "Показано: " {filtered.len()} " записей | "
                                            "Сумма: " {format!("{:.2}", total_sum)} " | "
                                            "Количество позиций: " {total_qty}
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <></> }.into_any()
                                }}
                            </div>

                            // Модальное окно результатов
                            <Show when=move || !operation_results.get().is_empty()>
                                <div class="modal-overlay" style="z-index: 1001;">
                                    <div class="modal-content" style="max-width: 800px; max-height: 80vh; overflow-y: auto;">
                                        <h3>"Результаты операции"</h3>
                                        <table style="width: 100%; border-collapse: collapse; margin: 20px 0;">
                                            <thead>
                                                <tr style="background: #f5f5f5;">
                                                    <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"ID"</th>
                                                    <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"Статус"</th>
                                                    <th style="border: 1px solid #ddd; padding: 8px; text-align: left;">"Ошибка"</th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                <For
                                                    each=move || operation_results.get()
                                                    key=|r| r.0.clone()
                                                    let:result
                                                >
                                                    {
                                                        let short_id = result.0.chars().take(8).collect::<String>();
                                                        let success = result.1;
                                                        let error_msg = result.2.clone().unwrap_or_default();

                                                        view! {
                                                            <tr>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">
                                                                    <code style="font-size: 0.85em;">{short_id}"..."</code>
                                                                </td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">
                                                                    {if success {
                                                                        view! { <span style="color: green;">"✓ Успешно"</span> }
                                                                    } else {
                                                                        view! { <span style="color: red;">"✗ Ошибка"</span> }
                                                                    }}
                                                                </td>
                                                                <td style="border: 1px solid #ddd; padding: 8px; color: #666;">
                                                                    {error_msg}
                                                                </td>
                                                            </tr>
                                                        }
                                                    }
                                                </For>
                                            </tbody>
                                        </table>
                                        <button
                                            on:click=move |_| set_operation_results.set(Vec::new())
                                            style="padding: 8px 16px; background: #4CAF50; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 14px;"
                                        >
                                            "Закрыть"
                                        </button>
                                    </div>
                                </div>
                            </Show>

            {move || {
                // Render summary and table; render filled rows only when not loading and no error
                if !loading.get() && error.get().is_none() {
                    view! {
                        <div>
                            <div class="table-container">
                                <table class="data-table" style="width: 100%; border-collapse: collapse;">
                                    <thead>
                                        <tr style="background: #f5f5f5;">
                                            <th style="border: 1px solid #ddd; padding: 8px; width: 40px; text-align: center;">
                                                <input
                                                    type="checkbox"
                                                    on:change=toggle_all
                                                    prop:checked=all_selected
                                                />
                                            </th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"ID"</th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("document_no")
                                                title="Сортировать"
                                            >
                                                {move || format!("Document №{}", get_sort_indicator(&sort_field.get(), "document_no", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("created_at_source")
                                                title="Сортировать"
                                            >
                                                {move || format!("Дата создания{}", get_sort_indicator(&sort_field.get(), "created_at_source", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("status_norm")
                                                title="Сортировать"
                                            >
                                                {move || format!("Status{}", get_sort_indicator(&sort_field.get(), "status_norm", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("substatus_raw")
                                                title="Сортировать"
                                            >
                                                {move || format!("Substatus{}", get_sort_indicator(&sort_field.get(), "substatus_raw", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none; text-align: center;"
                                                on:click=toggle_sort("is_posted")
                                                title="Сортировать"
                                            >
                                                {move || format!("Проведен{}", get_sort_indicator(&sort_field.get(), "is_posted", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; text-align: right; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("total_amount")
                                                title="Сортировать"
                                            >
                                                {move || format!("Сумма{}", get_sort_indicator(&sort_field.get(), "total_amount", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; text-align: center; cursor: pointer; user-select: none; width: 80px;"
                                                on:click=toggle_sort("line_count")
                                                title="Сортировать"
                                            >
                                                {move || format!("Кол-во{}", get_sort_indicator(&sort_field.get(), "line_count", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("description")
                                                title="Сортировать"
                                            >
                                                {move || format!("Description{}", get_sort_indicator(&sort_field.get(), "description", sort_ascending.get()))}
                                            </th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {move || get_filtered_sorted_items().into_iter().map(|posting| {
                                            let short_id = posting.id.chars().take(8).collect::<String>();
                                            let substatus = posting.substatus_raw.clone().unwrap_or_else(|| "-".to_string());
                                            let formatted_created_at = posting.created_at_source
                                                .as_ref()
                                                .map(|d| format_datetime(d))
                                                .unwrap_or_else(|| "-".to_string());
                                            let formatted_amount = format!("{:.2}", posting.total_amount);
                                            let is_posted_flag = posting.is_posted;

                                            // Создаем отдельные клоны для каждого обработчика
                                            let id_for_checkbox_change = posting.id.clone();
                                            let id_for_checkbox_check = posting.id.clone();
                                            let id1 = posting.id.clone();
                                            let id2 = posting.id.clone();
                                            let id3 = posting.id.clone();
                                            let id4 = posting.id.clone();
                                            let id5 = posting.id.clone();
                                            let id6 = posting.id.clone();
                                            let id7 = posting.id.clone();
                                            let id8 = posting.id.clone();
                                            let id9 = posting.id.clone();

                                            view! {
                                                <tr
                                                    style="transition: background 0.2s;"
                                                    onmouseenter="this.style.background='#f5f5f5'"
                                                    onmouseleave="this.style.background='white'"
                                                >
                                                    <td
                                                        style="border: 1px solid #ddd; padding: 8px; text-align: center;"
                                                        on:click=move |e| {
                                                            e.stop_propagation();
                                                        }
                                                    >
                                                        <input
                                                            type="checkbox"
                                                            on:change=move |_| toggle_selection(id_for_checkbox_change.clone())
                                                            prop:checked=move || is_selected(&id_for_checkbox_check)
                                                        />
                                                    </td>
                                                    <td
                                                        style="border: 1px solid #ddd; padding: 8px; cursor: pointer;"
                                                        on:click=move |_| {
                                                            set_selected_id.set(Some(id1.clone()));
                                                        }
                                                    >
                                                        <code style="font-size: 0.85em;">{format!("{}...", short_id)}</code>
                                                    </td>
                                                    <td
                                                        style="border: 1px solid #ddd; padding: 8px; cursor: pointer;"
                                                        on:click=move |_| {
                                                            set_selected_id.set(Some(id2.clone()));
                                                        }
                                                    >
                                                        {posting.document_no}
                                                    </td>
                                                    <td
                                                        style="border: 1px solid #ddd; padding: 8px; cursor: pointer;"
                                                        on:click=move |_| {
                                                            set_selected_id.set(Some(id3.clone()));
                                                        }
                                                    >
                                                        {formatted_created_at}
                                                    </td>
                                                    <td
                                                        style="border: 1px solid #ddd; padding: 8px; cursor: pointer;"
                                                        on:click=move |_| {
                                                            set_selected_id.set(Some(id4.clone()));
                                                        }
                                                    >
                                                        {posting.status_norm.clone()}
                                                    </td>
                                                    <td
                                                        style="border: 1px solid #ddd; padding: 8px; cursor: pointer;"
                                                        on:click=move |_| {
                                                            set_selected_id.set(Some(id5.clone()));
                                                        }
                                                    >
                                                        {substatus}
                                                    </td>
                                                    <td
                                                        style="border: 1px solid #ddd; padding: 8px; text-align: center; cursor: pointer;"
                                                        on:click=move |_| {
                                                            set_selected_id.set(Some(id6.clone()));
                                                        }
                                                    >
                                                        {if is_posted_flag {
                                                            view! { <span style="color: green; font-weight: 500;">"✓ Проведен"</span> }
                                                        } else {
                                                            view! { <span style="color: #999;">"○ Не проведен"</span> }
                                                        }}
                                                    </td>
                                                    <td
                                                        style="border: 1px solid #ddd; padding: 8px; text-align: right; cursor: pointer;"
                                                        on:click=move |_| {
                                                            set_selected_id.set(Some(id7.clone()));
                                                        }
                                                    >
                                                        {formatted_amount}
                                                    </td>
                                                    <td
                                                        style="border: 1px solid #ddd; padding: 8px; text-align: center; cursor: pointer;"
                                                        on:click=move |_| {
                                                            set_selected_id.set(Some(id8.clone()));
                                                        }
                                                    >
                                                        {posting.line_count}
                                                    </td>
                                                    <td
                                                        style="border: 1px solid #ddd; padding: 8px; cursor: pointer;"
                                                        on:click=move |_| {
                                                            set_selected_id.set(Some(id9.clone()));
                                                        }
                                                    >
                                                        {posting.description}
                                                    </td>
                                                </tr>
                                            }
                                        }).collect_view()}
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    let msg = if loading.get() {
                        "Loading...".to_string()
                    } else if let Some(err) = error.get() {
                        err.clone()
                    } else {
                        String::new()
                    };

                    view! {
                        <div>
                            {move || if !msg.is_empty() {
                                view! {
                                    <p style="margin: 4px 0 8px 0; font-size: 13px; color: #666;">{msg.clone()}</p>
                                }.into_any()
                            } else {
                                view! { <></> }.into_any()
                            }}
                            <div class="table-container">
                                <table class="data-table" style="width: 100%; border-collapse: collapse;">
                                    <thead>
                                        <tr style="background: #f5f5f5;">
                                            <th style="border: 1px solid #ddd; padding: 8px; width: 40px;"></th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"ID"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Document №"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Дата"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: center;">"Статус"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Сумма"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: center;">"Количество позиций"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Description"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        <tr><td colspan="8"></td></tr>
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    }.into_any()
                }
            }}
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}
