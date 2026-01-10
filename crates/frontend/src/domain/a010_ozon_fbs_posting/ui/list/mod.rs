use super::details::OzonFbsPostingDetail;
use crate::shared::list_utils::{get_sort_indicator, Sortable};
use crate::shared::modal_stack::ModalStackService;
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFbsPostingDto {
    pub id: String,
    pub code: String,
    pub description: String,
    pub document_no: String,
    pub status_norm: String, // Статус постинга (DELIVERED, CANCELLED и т.д.)
    pub delivered_at: Option<String>, // Дата доставки
    pub substatus_raw: Option<String>, // Подстатус
    pub total_amount: f64,
    pub line_count: usize,
    pub is_posted: bool, // Флаг проведения
}

impl Sortable for OzonFbsPostingDto {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "document_no" => self
                .document_no
                .to_lowercase()
                .cmp(&other.document_no.to_lowercase()),
            "delivered_at" => {
                // Сортировка с учетом None (None идут в конец)
                match (&self.delivered_at, &other.delivered_at) {
                    (Some(a), Some(b)) => a.cmp(b),
                    (Some(_), None) => Ordering::Less,
                    (None, Some(_)) => Ordering::Greater,
                    (None, None) => Ordering::Equal,
                }
            }
            "substatus_raw" => match (&self.substatus_raw, &other.substatus_raw) {
                (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            "total_amount" => self
                .total_amount
                .partial_cmp(&other.total_amount)
                .unwrap_or(Ordering::Equal),
            "line_count" => self.line_count.cmp(&other.line_count),
            "description" => self
                .description
                .to_lowercase()
                .cmp(&other.description.to_lowercase()),
            "is_posted" => self.is_posted.cmp(&other.is_posted),
            _ => Ordering::Equal,
        }
    }
}

#[component]
pub fn OzonFbsPostingList() -> impl IntoView {
    let modal_stack =
        use_context::<ModalStackService>().expect("ModalStackService not found in context");
    let (postings, set_postings) = signal::<Vec<OzonFbsPostingDto>>(Vec::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (selected_id, set_selected_id) = signal::<Option<String>>(None);
    let (detail_reload_trigger, set_detail_reload_trigger) = signal::<u32>(0);

    // Сортировка
    let (sort_field, set_sort_field) = signal::<String>("delivered_at".to_string());
    let (sort_ascending, set_sort_ascending) = signal(false); // По умолчанию - новые сначала

    // Множественный выбор
    let (selected_ids, set_selected_ids) = signal::<Vec<String>>(Vec::new());

    // Фильтр по периоду
    let (date_from, set_date_from) = signal::<Option<String>>(None);
    let (date_to, set_date_to) = signal::<Option<String>>(None);

    // Фильтр по статусу
    let (status_filter, set_status_filter) = signal::<Option<String>>(None);

    // Статус массовых операций
    let (posting_in_progress, set_posting_in_progress) = signal(false);
    let (operation_results, set_operation_results) =
        signal::<Vec<(String, bool, Option<String>)>>(Vec::new());
    let (current_operation, set_current_operation) = signal::<Option<(usize, usize)>>(None); // (current, total)

    let open_detail_modal = move |id: String| {
        let id_val = id.clone();
        let reload = detail_reload_trigger;
        modal_stack.push_with_frame(
            Some("max-width: min(1200px, 95vw); width: min(1200px, 95vw); height: calc(100vh - 80px); overflow: hidden;".to_string()),
            Some("ozon-fbs-posting-detail-modal".to_string()),
            move |handle| {
                view! {
                    <OzonFbsPostingDetail
                        id=id_val.clone()
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

    let open_results_modal = move |results: Vec<(String, bool, Option<String>)>| {
        modal_stack.push_with_frame(
            Some("max-width: min(800px, 95vw); width: min(800px, 95vw); max-height: 80vh; overflow-y: auto;".to_string()),
            Some("operation-results-modal".to_string()),
            move |handle| {
                let results = results.clone();
                view! {
                    <div class="details-container">
                        <div class="modal-header">
                            <h3 class="modal-title">"Результаты операции"</h3>
                            <div class="modal-header-actions">
                                <button class="button button--secondary" on:click=move |_| handle.close()>
                                    "Закрыть"
                                </button>
                            </div>
                        </div>
                        <div class="modal-body">
                            <table class="results-table">
                                <thead>
                                    <tr>
                                        <th>"ID"</th>
                                        <th>"Статус"</th>
                                        <th>"Ошибка"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    <For
                                        each=move || results.clone()
                                        key=|r| r.0.clone()
                                        let:result
                                    >
                                        {
                                            let short_id = result.0.chars().take(8).collect::<String>();
                                            let success = result.1;
                                            let error_msg = result.2.clone().unwrap_or_default();

                                            view! {
                                                <tr>
                                                    <td class="results-table__id">
                                                        <code>{short_id}"..."</code>
                                                    </td>
                                                    <td>
                                                        {if success {
                                                            view! { <span class="text-success">"✓ Успешно"</span> }
                                                        } else {
                                                            view! { <span class="text-error">"✗ Ошибка"</span> }
                                                        }}
                                                    </td>
                                                    <td class="text-muted">
                                                        {error_msg}
                                                    </td>
                                                </tr>
                                            }
                                        }
                                    </For>
                                </tbody>
                            </table>
                        </div>
                    </div>
                }
                .into_any()
            },
        );
    };

    let load_postings = move || {
        let set_postings = set_postings.clone();
        let set_loading = set_loading.clone();
        let set_error = set_error.clone();
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let url = "http://localhost:3000/api/a010/ozon-fbs-posting";

            match Request::get(url).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status == 200 {
                        match response.text().await {
                            Ok(text) => {
                                log!(
                                    "Received response text (first 500 chars): {}",
                                    text.chars().take(500).collect::<String>()
                                );

                                match serde_json::from_str::<Vec<serde_json::Value>>(&text) {
                                    Ok(data) => {
                                        let total_count = data.len();
                                        log!("Parsed {} items from JSON", total_count);

                                        // Упрощенная десериализация - берем только нужные поля
                                        let items: Vec<OzonFbsPostingDto> = data
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

                                                // Дата доставки (delivered_at из state)
                                                let delivered_at = v
                                                    .get("state")
                                                    .and_then(|s| s.get("delivered_at"))
                                                    .and_then(|d| d.as_str())
                                                    .map(|s| s.to_string());

                                                // Подстатус (substatus_raw из state)
                                                let substatus_raw = v
                                                    .get("state")
                                                    .and_then(|s| s.get("substatus_raw"))
                                                    .and_then(|d| d.as_str())
                                                    .map(|s| s.to_string());

                                                let lines = v.get("lines")?.as_array()?;
                                                let line_count = lines.len();
                                                let total_amount: f64 = lines
                                                    .iter()
                                                    .filter_map(|line| {
                                                        line.get("amount_line")?.as_f64()
                                                    })
                                                    .sum();

                                                let is_posted = v
                                                    .get("is_posted")
                                                    .and_then(|p| p.as_bool())
                                                    .unwrap_or(false);

                                                let result = Some(OzonFbsPostingDto {
                                                    id: v.get("id")?.as_str()?.to_string(),
                                                    code: v.get("code")?.as_str()?.to_string(),
                                                    description: v
                                                        .get("description")?
                                                        .as_str()?
                                                        .to_string(),
                                                    document_no: v
                                                        .get("header")?
                                                        .get("document_no")?
                                                        .as_str()?
                                                        .to_string(),
                                                    status_norm,
                                                    delivered_at,
                                                    substatus_raw,
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

                                        log!(
                                            "Successfully parsed {} postings out of {}",
                                            items.len(),
                                            total_count
                                        );
                                        set_postings.set(items);
                                        set_loading.set(false);
                                    }
                                    Err(e) => {
                                        log!("Failed to parse response: {:?}", e);
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
                    log!("Failed to fetch postings: {:?}", e);
                    set_error.set(Some(format!("Failed to fetch postings: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    // Функция для получения отфильтрованных и отсортированных данных
    let get_filtered_sorted_items = move || -> Vec<OzonFbsPostingDto> {
        let mut result = postings.get();

        // Фильтр по периоду
        let from = date_from.get();
        let to = date_to.get();
        if from.is_some() || to.is_some() {
            result.retain(|item| {
                if let Some(ref item_date) = item.delivered_at {
                    if let Some(ref from_date) = from {
                        if item_date < from_date {
                            return false;
                        }
                    }
                    if let Some(ref to_date) = to {
                        if item_date > to_date {
                            return false;
                        }
                    }
                    true
                } else {
                    false // Нет даты - не показываем при фильтрации
                }
            });
        }

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
            if ascending {
                cmp
            } else {
                cmp.reverse()
            }
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
            set_selected_ids.set(items.iter().map(|item| item.id.clone()).collect());
            // Выбрать все
        }
    };

    // Проверка, выбраны ли все
    let all_selected = move || {
        let items = get_filtered_sorted_items();
        let selected = selected_ids.get();
        !items.is_empty() && selected.len() == items.len()
    };

    // Проверка, выбран ли конкретный документ
    let is_selected = move |id: &str| selected_ids.get().contains(&id.to_string());

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
                let url = format!(
                    "http://localhost:3000/api/a010/ozon-fbs-posting/{}/post",
                    id
                );
                match Request::post(&url).send().await {
                    Ok(response) => {
                        if response.status() == 200 {
                            results.push((id.clone(), true, None));
                        } else {
                            results.push((
                                id.clone(),
                                false,
                                Some(format!("HTTP {}", response.status())),
                            ));
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

            let url = "http://localhost:3000/api/a010/ozon-fbs-posting";
            match Request::get(url).send().await {
                Ok(response) => {
                    if response.status() == 200 {
                        if let Ok(text) = response.text().await {
                            if let Ok(data) = serde_json::from_str::<Vec<serde_json::Value>>(&text)
                            {
                                let items: Vec<OzonFbsPostingDto> = data
                                    .into_iter()
                                    .filter_map(|v| {
                                        let status_norm = v
                                            .get("state")
                                            .and_then(|s| s.get("status_norm"))
                                            .and_then(|d| d.as_str())
                                            .map(|s| s.to_string())
                                            .unwrap_or_else(|| "UNKNOWN".to_string());

                                        let delivered_at = v
                                            .get("state")
                                            .and_then(|s| s.get("delivered_at"))
                                            .and_then(|d| d.as_str())
                                            .map(|s| s.to_string());

                                        let substatus_raw = v
                                            .get("state")
                                            .and_then(|s| s.get("substatus_raw"))
                                            .and_then(|d| d.as_str())
                                            .map(|s| s.to_string());

                                        let lines = v.get("lines")?.as_array()?;
                                        let line_count = lines.len();
                                        let total_amount: f64 = lines
                                            .iter()
                                            .filter_map(|line| line.get("amount_line")?.as_f64())
                                            .sum();

                                        let is_posted = v
                                            .get("is_posted")
                                            .and_then(|p| p.as_bool())
                                            .unwrap_or(false);

                                        Some(OzonFbsPostingDto {
                                            id: v.get("id")?.as_str()?.to_string(),
                                            code: v.get("code")?.as_str()?.to_string(),
                                            description: v
                                                .get("description")?
                                                .as_str()?
                                                .to_string(),
                                            document_no: v
                                                .get("header")?
                                                .get("document_no")?
                                                .as_str()?
                                                .to_string(),
                                            status_norm,
                                            delivered_at,
                                            substatus_raw,
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
                let url = format!(
                    "http://localhost:3000/api/a010/ozon-fbs-posting/{}/unpost",
                    id
                );
                match Request::post(&url).send().await {
                    Ok(response) => {
                        if response.status() == 200 {
                            results.push((id.clone(), true, None));
                        } else {
                            results.push((
                                id.clone(),
                                false,
                                Some(format!("HTTP {}", response.status())),
                            ));
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

            let url = "http://localhost:3000/api/a010/ozon-fbs-posting";
            match Request::get(url).send().await {
                Ok(response) => {
                    if response.status() == 200 {
                        if let Ok(text) = response.text().await {
                            if let Ok(data) = serde_json::from_str::<Vec<serde_json::Value>>(&text)
                            {
                                let items: Vec<OzonFbsPostingDto> = data
                                    .into_iter()
                                    .filter_map(|v| {
                                        let status_norm = v
                                            .get("state")
                                            .and_then(|s| s.get("status_norm"))
                                            .and_then(|d| d.as_str())
                                            .map(|s| s.to_string())
                                            .unwrap_or_else(|| "UNKNOWN".to_string());

                                        let delivered_at = v
                                            .get("state")
                                            .and_then(|s| s.get("delivered_at"))
                                            .and_then(|d| d.as_str())
                                            .map(|s| s.to_string());

                                        let substatus_raw = v
                                            .get("state")
                                            .and_then(|s| s.get("substatus_raw"))
                                            .and_then(|d| d.as_str())
                                            .map(|s| s.to_string());

                                        let lines = v.get("lines")?.as_array()?;
                                        let line_count = lines.len();
                                        let total_amount: f64 = lines
                                            .iter()
                                            .filter_map(|line| line.get("amount_line")?.as_f64())
                                            .sum();

                                        let is_posted = v
                                            .get("is_posted")
                                            .and_then(|p| p.as_bool())
                                            .unwrap_or(false);

                                        Some(OzonFbsPostingDto {
                                            id: v.get("id")?.as_str()?.to_string(),
                                            code: v.get("code")?.as_str()?.to_string(),
                                            description: v
                                                .get("description")?
                                                .as_str()?
                                                .to_string(),
                                            document_no: v
                                                .get("header")?
                                                .get("document_no")?
                                                .as_str()?
                                                .to_string(),
                                            status_norm,
                                            delivered_at,
                                            substatus_raw,
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
        <div class="ozon-fbs-posting-list">
            {move || {
                if let Some(id) = selected_id.get() {
                    open_detail_modal(id);
                    set_selected_id.set(None);
                    view! { <></> }.into_any()
                } else {
                    view! {
                        <div>
                            <div class="doc-list__toolbar">
                                <h2 class="doc-list__title">"OZON FBS Posting (A010)"</h2>
                                <button class="button button--secondary" on:click=move |_| load_postings()>
                                    "Обновить"
                                </button>
                            </div>

                            // Панель фильтров и массовых операций
                            <div class="doc-filters">
                                <div class="doc-filters__row">
                                    // Фильтр периода
                                    <div class="doc-filter">
                                        <label class="doc-filter__label">"Период:"</label>
                                        <input
                                            type="date"
                                            on:input=move |e| {
                                                let value = event_target_value(&e);
                                                set_date_from.set(if value.is_empty() { None } else { Some(value) });
                                            }
                                            class="doc-filter__input"
                                        />
                                        <span>"—"</span>
                                        <input
                                            type="date"
                                            on:input=move |e| {
                                                let value = event_target_value(&e);
                                                set_date_to.set(if value.is_empty() { None } else { Some(value) });
                                            }
                                            class="doc-filter__input"
                                        />
                                    </div>

                                    // Фильтр по статусу
                                    <div class="doc-filter">
                                        <label class="doc-filter__label">"Статус:"</label>
                                        <select
                                            on:change=move |e| {
                                                let value = event_target_value(&e);
                                                set_status_filter.set(if value.is_empty() { None } else { Some(value) });
                                            }
                                            class="doc-filter__select"
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
                                            class="button button--info"
                                        >
                                            {move || format!("Post ({})", selected_ids.get().len())}
                                        </button>
                                        <button
                                            disabled=move || selected_ids.get().is_empty() || posting_in_progress.get()
                                            on:click=unpost_selected
                                            class="button button--warning"
                                        >
                                            {move || format!("Unpost ({})", selected_ids.get().len())}
                                        </button>
                                    </div>

                                    // Индикатор прогресса
                                    <Show when=move || posting_in_progress.get()>
                                        {move || {
                                            if let Some((current, total)) = current_operation.get() {
                                                view! {
                                                    <span class="doc-list__progress">
                                                        {format!("Обработка {}/{} документов...", current, total)}
                                                    </span>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <span class="doc-list__progress">"Обработка..."</span>
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
                                        <div class="doc-list__summary">
                                            "Показано: " {filtered.len()} " записей | "
                                            "Сумма: " {format!("{:.2}", total_sum)} " | "
                                            "Количество позиций: " {total_qty}
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <></> }.into_any()
                                }}
                            </div>

                            // Результаты операции открываем через ModalStackService
                            {move || {
                                let results = operation_results.get();
                                if results.is_empty() {
                                    view! { <></> }.into_any()
                                } else {
                                    open_results_modal(results);
                                    set_operation_results.set(Vec::new());
                                    view! { <></> }.into_any()
                                }
                            }}

            {move || {
                // Render summary and table; render filled rows only when not loading and no error
                if !loading.get() && error.get().is_none() {
                    view! {
                        <div>
                            <div class="table-container">
                                <table class="table__data" style="width: 100%; border-collapse: collapse;">
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
                                                on:click=toggle_sort("delivered_at")
                                                title="Сортировать"
                                            >
                                                {move || format!("Дата доставки{}", get_sort_indicator(&sort_field.get(), "delivered_at", sort_ascending.get()))}
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
                                                {move || format!("Статус{}", get_sort_indicator(&sort_field.get(), "is_posted", sort_ascending.get()))}
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
                                            let formatted_date = posting.delivered_at
                                                .as_ref()
                                                .map(|d| format_date(d))
                                                .unwrap_or_else(|| "-".to_string());
                                            let substatus = posting.substatus_raw.clone().unwrap_or_else(|| "-".to_string());
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
                                                        {formatted_date}
                                                    </td>
                                                    <td
                                                        style="border: 1px solid #ddd; padding: 8px; cursor: pointer;"
                                                        on:click=move |_| {
                                                            set_selected_id.set(Some(id4.clone()));
                                                        }
                                                    >
                                                        {substatus}
                                                    </td>
                                                    <td
                                                        style="border: 1px solid #ddd; padding: 8px; text-align: center; cursor: pointer;"
                                                        on:click=move |_| {
                                                            set_selected_id.set(Some(id5.clone()));
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
                                                            set_selected_id.set(Some(id6.clone()));
                                                        }
                                                    >
                                                        {formatted_amount}
                                                    </td>
                                                    <td
                                                        style="border: 1px solid #ddd; padding: 8px; text-align: center; cursor: pointer;"
                                                        on:click=move |_| {
                                                            set_selected_id.set(Some(id7.clone()));
                                                        }
                                                    >
                                                        {posting.line_count}
                                                    </td>
                                                    <td
                                                        style="border: 1px solid #ddd; padding: 8px; cursor: pointer;"
                                                        on:click=move |_| {
                                                            set_selected_id.set(Some(id8.clone()));
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
                                <table class="table__data" style="width: 100%; border-collapse: collapse;">
                                    <thead>
                                        <tr style="background: #f5f5f5;">
                                            <th style="border: 1px solid #ddd; padding: 8px; width: 40px;"></th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"ID"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Document №"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Дата"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Substatus"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: center;">"Статус"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Сумма"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: center;">"Количество позиций"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Description"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        <tr><td colspan="9"></td></tr>
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
