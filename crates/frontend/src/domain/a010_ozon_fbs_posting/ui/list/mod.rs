use leptos::prelude::*;
use leptos::logging::log;
use serde::{Deserialize, Serialize};
use gloo_net::http::Request;
use super::details::OzonFbsPostingDetail;
use crate::shared::list_utils::{get_sort_indicator, Sortable};
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
    pub delivered_at: Option<String>, // Дата продажи (доставки)
    pub total_amount: f64,
    pub line_count: usize,
}

impl Sortable for OzonFbsPostingDto {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "document_no" => self.document_no.to_lowercase().cmp(&other.document_no.to_lowercase()),
            "delivered_at" => {
                // Сортировка с учетом None (None идут в конец)
                match (&self.delivered_at, &other.delivered_at) {
                    (Some(a), Some(b)) => a.cmp(b),
                    (Some(_), None) => Ordering::Less,
                    (None, Some(_)) => Ordering::Greater,
                    (None, None) => Ordering::Equal,
                }
            },
            "total_amount" => self.total_amount.partial_cmp(&other.total_amount).unwrap_or(Ordering::Equal),
            "line_count" => self.line_count.cmp(&other.line_count),
            "description" => self.description.to_lowercase().cmp(&other.description.to_lowercase()),
            _ => Ordering::Equal,
        }
    }
}

#[component]
pub fn OzonFbsPostingList() -> impl IntoView {
    let (postings, set_postings) = signal::<Vec<OzonFbsPostingDto>>(Vec::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (selected_id, set_selected_id) = signal::<Option<String>>(None);
    
    // Сортировка
    let (sort_field, set_sort_field) = signal::<String>("delivered_at".to_string());
    let (sort_ascending, set_sort_ascending) = signal(false); // По умолчанию - новые сначала

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
                                log!("Received response text (first 500 chars): {}", 
                                    text.chars().take(500).collect::<String>());
                                
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
                                                
                                                // Дата продажи (delivered_at из state)
                                                let delivered_at = v
                                                    .get("state")
                                                    .and_then(|s| s.get("delivered_at"))
                                                    .and_then(|d| d.as_str())
                                                    .map(|s| s.to_string());
                                                
                                                let lines = v.get("lines")?.as_array()?;
                                                let line_count = lines.len();
                                                let total_amount: f64 = lines
                                                    .iter()
                                                    .filter_map(|line| line.get("amount_line")?.as_f64())
                                                    .sum();
                                                
                                                let result = Some(OzonFbsPostingDto {
                                                    id: v.get("id")?.as_str()?.to_string(),
                                                    code: v.get("code")?.as_str()?.to_string(),
                                                    description: v.get("description")?.as_str()?.to_string(),
                                                    document_no: v
                                                        .get("header")?
                                                        .get("document_no")?
                                                        .as_str()?
                                                        .to_string(),
                                                    delivered_at,
                                                    total_amount,
                                                    line_count,
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

    // Функция для получения отсортированных данных
    let get_sorted_items = move || -> Vec<OzonFbsPostingDto> {
        let mut result = postings.get();
        let field = sort_field.get();
        let ascending = sort_ascending.get();
        
        result.sort_by(|a, b| {
            let cmp = a.compare_by_field(b, &field);
            if ascending { cmp } else { cmp.reverse() }
        });
        
        result
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

    // Автоматическая загрузка при открытии
    load_postings();

    view! {
        <div class="ozon-fbs-posting-list">
            {move || {
                if let Some(id) = selected_id.get() {
                    view! {
                        <div class="modal-overlay" style="align-items: flex-start; padding-top: 40px;">
                            <div class="modal-content" style="max-width: 1200px; height: calc(100vh - 80px); overflow: hidden; margin: 0;">
                                <OzonFbsPostingDetail
                                    id=id
                                    on_close=move || set_selected_id.set(None)
                                />
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div>
                            <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px;">
                                <h2 style="margin: 0;">"OZON FBS Posting (A010)"</h2>
                                <button
                                    on:click=move |_| {
                                        load_postings();
                                    }
                                    style="padding: 6px 12px; background: #4CAF50; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 14px;"
                                >
                                    "Обновить"
                                </button>
                            </div>

            {move || {
                let msg = if loading.get() {
                    "Loading...".to_string()
                } else if let Some(err) = error.get() {
                    err.clone()
                } else {
                    format!("Total: {} records", postings.get().len())
                };

                // Render summary and table; render filled rows only when not loading and no error
                if !loading.get() && error.get().is_none() {
                    view! {
                        <div>
                            <p style="margin: 4px 0 8px 0; font-size: 13px; color: #666;">{msg}</p>
                            <div class="table-container">
                                <table class="data-table" style="width: 100%; border-collapse: collapse;">
                                    <thead>
                                        <tr style="background: #f5f5f5;">
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
                                                {move || format!("Дата продажи{}", get_sort_indicator(&sort_field.get(), "delivered_at", sort_ascending.get()))}
                                            </th>
                                            <th 
                                                style="border: 1px solid #ddd; padding: 8px; text-align: right; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("total_amount")
                                                title="Сортировать"
                                            >
                                                {move || format!("Сумма{}", get_sort_indicator(&sort_field.get(), "total_amount", sort_ascending.get()))}
                                            </th>
                                            <th 
                                                style="border: 1px solid #ddd; padding: 8px; text-align: center; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("line_count")
                                                title="Сортировать"
                                            >
                                                {move || format!("Количество позиций{}", get_sort_indicator(&sort_field.get(), "line_count", sort_ascending.get()))}
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
                                        {move || get_sorted_items().into_iter().map(|posting| {
                                            let short_id = posting.id.chars().take(8).collect::<String>();
                                            let posting_id = posting.id.clone();
                                            let formatted_date = posting.delivered_at
                                                .as_ref()
                                                .map(|d| format_date(d))
                                                .unwrap_or_else(|| "-".to_string());
                                            let formatted_amount = format!("{:.2}", posting.total_amount);
                                            view! {
                                                <tr
                                                    on:click=move |_| {
                                                        set_selected_id.set(Some(posting_id.clone()));
                                                    }
                                                    style="cursor: pointer; transition: background 0.2s;"
                                                    onmouseenter="this.style.background='#f5f5f5'"
                                                    onmouseleave="this.style.background='white'"
                                                >
                                                    <td style="border: 1px solid #ddd; padding: 8px;">
                                                        <code style="font-size: 0.85em;">{format!("{}...", short_id)}</code>
                                                    </td>
                                                    <td style="border: 1px solid #ddd; padding: 8px;">{posting.document_no}</td>
                                                    <td style="border: 1px solid #ddd; padding: 8px;">{formatted_date}</td>
                                                    <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{formatted_amount}</td>
                                                    <td style="border: 1px solid #ddd; padding: 8px; text-align: center;">{posting.line_count}</td>
                                                    <td style="border: 1px solid #ddd; padding: 8px;">{posting.description}</td>
                                                </tr>
                                            }
                                        }).collect_view()}
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div>
                            <p style="margin: 4px 0 8px 0; font-size: 13px; color: #666;">{msg}</p>
                            <div class="table-container">
                                <table class="data-table" style="width: 100%; border-collapse: collapse;">
                                    <thead>
                                        <tr style="background: #f5f5f5;">
                                            <th style="border: 1px solid #ddd; padding: 8px;">"ID"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Document №"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Дата"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Сумма"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: center;">"Количество позиций"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Description"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        <tr><td colspan="6"></td></tr>
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

