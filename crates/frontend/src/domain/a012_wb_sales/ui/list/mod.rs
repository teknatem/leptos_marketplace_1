use super::details::WbSalesDetail;
use crate::shared::list_utils::{get_sort_indicator, Sortable};
use chrono::{Datelike, Utc};
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use wasm_bindgen::JsCast;
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

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

/// Форматирует число с разделителем тысяч (пробел)
fn format_number_with_separator(num: f64, decimals: usize) -> String {
    let formatted = format!("{:.prec$}", num, prec = decimals);
    let parts: Vec<&str> = formatted.split('.').collect();

    let integer_part = parts[0];
    let decimal_part = if parts.len() > 1 { parts[1] } else { "" };

    // Добавляем пробелы каждые 3 цифры справа налево
    let mut result = String::new();
    let chars: Vec<char> = integer_part.chars().collect();
    for (i, ch) in chars.iter().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.insert(0, ' ');
        }
        result.insert(0, *ch);
    }

    if decimals > 0 && !decimal_part.is_empty() {
        result.push('.');
        result.push_str(decimal_part);
    }

    result
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSalesDto {
    pub id: String,
    pub document_no: String,
    pub sale_date: String,
    pub supplier_article: String,
    pub name: String,
    pub qty: f64,
    pub amount_line: Option<f64>,
    pub total_price: Option<f64>,
    pub finished_price: Option<f64>,
    pub event_type: String,
    pub organization_name: Option<String>,
    pub marketplace_article: Option<String>,
    pub nomenclature_code: Option<String>,
    pub nomenclature_article: Option<String>,
}

impl Sortable for WbSalesDto {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "document_no" => self
                .document_no
                .to_lowercase()
                .cmp(&other.document_no.to_lowercase()),
            "sale_date" => self.sale_date.cmp(&other.sale_date),
            "supplier_article" => self
                .supplier_article
                .to_lowercase()
                .cmp(&other.supplier_article.to_lowercase()),
            "name" => self.name.to_lowercase().cmp(&other.name.to_lowercase()),
            "qty" => self.qty.partial_cmp(&other.qty).unwrap_or(Ordering::Equal),
            "amount_line" => match (&self.amount_line, &other.amount_line) {
                (Some(a), Some(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            "total_price" => match (&self.total_price, &other.total_price) {
                (Some(a), Some(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            "finished_price" => match (&self.finished_price, &other.finished_price) {
                (Some(a), Some(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            "event_type" => self
                .event_type
                .to_lowercase()
                .cmp(&other.event_type.to_lowercase()),
            "organization_name" => match (&self.organization_name, &other.organization_name) {
                (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            "marketplace_article" => {
                match (&self.marketplace_article, &other.marketplace_article) {
                    (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
                    (Some(_), None) => Ordering::Less,
                    (None, Some(_)) => Ordering::Greater,
                    (None, None) => Ordering::Equal,
                }
            }
            "nomenclature_code" => match (&self.nomenclature_code, &other.nomenclature_code) {
                (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            "nomenclature_article" => {
                match (&self.nomenclature_article, &other.nomenclature_article) {
                    (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
                    (Some(_), None) => Ordering::Less,
                    (None, Some(_)) => Ordering::Greater,
                    (None, None) => Ordering::Equal,
                }
            }
            _ => Ordering::Equal,
        }
    }
}

#[component]
pub fn WbSalesList() -> impl IntoView {
    let (sales, set_sales) = signal::<Vec<WbSalesDto>>(Vec::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (selected_id, set_selected_id) = signal::<Option<String>>(None);
    let (selected_ids, set_selected_ids) = signal::<Vec<String>>(Vec::new());
    let (batch_progress, set_batch_progress) = signal::<Option<(usize, usize)>>(None); // (processed, total)
    let (is_batch_processing, set_is_batch_processing) = signal(false); // Синхронный флаг для блокировки

    // Сортировка
    let (sort_field, set_sort_field) = signal::<String>("sale_date".to_string());
    let (sort_ascending, set_sort_ascending) = signal(false); // По умолчанию - новые сначала

    // Фильтры - период по умолчанию: текущий месяц
    let now = Utc::now().date_naive();
    let year = now.year();
    let month = now.month();
    let month_start =
        chrono::NaiveDate::from_ymd_opt(year, month, 1).expect("Invalid month start date");
    let month_end = if month == 12 {
        chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
            .map(|d| d - chrono::Duration::days(1))
            .expect("Invalid month end date")
    } else {
        chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
            .map(|d| d - chrono::Duration::days(1))
            .expect("Invalid month end date")
    };

    let (date_from, set_date_from) = signal(month_start.format("%Y-%m-%d").to_string());
    let (date_to, set_date_to) = signal(month_end.format("%Y-%m-%d").to_string());

    let load_sales = move || {
        let set_sales = set_sales.clone();
        let set_loading = set_loading.clone();
        let set_error = set_error.clone();
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let date_from_val = date_from.get();
            let date_to_val = date_to.get();

            // Ограничиваем количество записей для оптимизации
            let url = format!(
                "http://localhost:3000/api/a012/wb-sales?date_from={}&date_to={}&limit=20000",
                date_from_val, date_to_val
            );

            log!("Loading WB sales with URL: {}", url);

            match Request::get(&url).send().await {
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
                                        let items: Vec<WbSalesDto> = data
                                            .into_iter()
                                            .enumerate()
                                            .filter_map(|(idx, v)| {
                                                // Organization name из верхнего уровня
                                                let organization_name = v
                                                    .get("organization_name")
                                                    .and_then(|o| o.as_str())
                                                    .map(|s| s.to_string());

                                                // Дата продажи из state.sale_dt
                                                let sale_date = v
                                                    .get("state")
                                                    .and_then(|s| s.get("sale_dt"))
                                                    .and_then(|d| d.as_str())
                                                    .unwrap_or("")
                                                    .to_string();

                                                // Данные из line
                                                let line = v.get("line")?;
                                                let supplier_article = line
                                                    .get("supplier_article")?
                                                    .as_str()?
                                                    .to_string();
                                                let name = line.get("name")?.as_str()?.to_string();
                                                let qty = line.get("qty")?.as_f64()?;
                                                let amount_line = line
                                                    .get("amount_line")
                                                    .and_then(|a| a.as_f64());
                                                let total_price = line
                                                    .get("total_price")
                                                    .and_then(|a| a.as_f64());
                                                let finished_price = line
                                                    .get("finished_price")
                                                    .and_then(|a| a.as_f64());

                                                // Event type из state
                                                let event_type = v
                                                    .get("state")
                                                    .and_then(|s| s.get("event_type"))
                                                    .and_then(|e| e.as_str())
                                                    .unwrap_or("unknown")
                                                    .to_string();

                                                // Новые поля из БД
                                                let marketplace_article = v
                                                    .get("marketplace_article")
                                                    .and_then(|a| a.as_str())
                                                    .map(|s| s.to_string());
                                                let nomenclature_code = v
                                                    .get("nomenclature_code")
                                                    .and_then(|a| a.as_str())
                                                    .map(|s| s.to_string());
                                                let nomenclature_article = v
                                                    .get("nomenclature_article")
                                                    .and_then(|a| a.as_str())
                                                    .map(|s| s.to_string());

                                                let result = Some(WbSalesDto {
                                                    id: v.get("id")?.as_str()?.to_string(),
                                                    document_no: v
                                                        .get("header")?
                                                        .get("document_no")?
                                                        .as_str()?
                                                        .to_string(),
                                                    sale_date,
                                                    supplier_article,
                                                    name,
                                                    qty,
                                                    amount_line,
                                                    total_price,
                                                    finished_price,
                                                    event_type,
                                                    organization_name,
                                                    marketplace_article,
                                                    nomenclature_code,
                                                    nomenclature_article,
                                                });

                                                if result.is_none() {
                                                    log!("Failed to parse item {}", idx);
                                                }

                                                result
                                            })
                                            .collect();

                                        log!(
                                            "Successfully parsed {} sales out of {}",
                                            items.len(),
                                            total_count
                                        );
                                        set_sales.set(items);
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
                    log!("Failed to fetch sales: {:?}", e);
                    set_error.set(Some(format!("Failed to fetch sales: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    // Функция для получения отсортированных данных
    let get_sorted_items = move || -> Vec<WbSalesDto> {
        let mut result = sales.get();
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

    // Функция для вычисления итогов
    let totals = move || {
        let data = get_sorted_items();
        let total_qty: f64 = data.iter().map(|s| s.qty).sum();
        let total_amount: f64 = data.iter().filter_map(|s| s.amount_line).sum();
        let total_price: f64 = data.iter().filter_map(|s| s.total_price).sum();
        let total_finished: f64 = data.iter().filter_map(|s| s.finished_price).sum();
        (
            data.len(),
            total_qty,
            total_amount,
            total_price,
            total_finished,
        )
    };

    // Selection helpers
    let is_selected =
        move |id: &str| -> bool { selected_ids.with(|ids| ids.iter().any(|x| x == id)) };
    let toggle_row = move |id: String| {
        set_selected_ids.update(|ids| {
            if let Some(pos) = ids.iter().position(|x| x == &id) {
                ids.remove(pos);
            } else {
                ids.push(id);
            }
        });
    };
    let clear_selection = move || set_selected_ids.set(Vec::new());
    let select_all_current = move |checked: bool| {
        if checked {
            let all_ids: Vec<String> = get_sorted_items().into_iter().map(|s| s.id).collect();
            set_selected_ids.set(all_ids);
        } else {
            clear_selection();
        }
    };

    // Batch post/unpost actions - разбивает на чанки по 100 документов
    let batch_update = move |post: bool| {
        let ids = selected_ids.get();
        if ids.is_empty() {
            return;
        }
        let total = ids.len();
        let reload = load_sales.clone();
        let set_progress = set_batch_progress.clone();
        let set_processing = set_is_batch_processing.clone();

        // Сразу устанавливаем флаг обработки и начальный прогресс
        set_processing.set(true);
        set_progress.set(Some((0, total)));

        wasm_bindgen_futures::spawn_local(async move {
            let endpoint = if post {
                "http://localhost:3000/api/a012/wb-sales/batch-post"
            } else {
                "http://localhost:3000/api/a012/wb-sales/batch-unpost"
            };

            let mut processed = 0;

            // Разбиваем на чанки по 100
            for chunk in ids.chunks(100) {
                let payload = serde_json::json!({
                    "ids": chunk
                });

                let response = Request::post(endpoint)
                    .header("Content-Type", "application/json")
                    .body(serde_json::to_string(&payload).unwrap_or_default())
                    .map(|req| req.send());

                match response {
                    Ok(future) => match future.await {
                        Ok(_) => {
                            processed += chunk.len();
                            set_progress.set(Some((processed, total)));
                        }
                        Err(e) => {
                            log!("Failed to send batch request: {:?}", e);
                            processed += chunk.len();
                            set_progress.set(Some((processed, total)));
                        }
                    },
                    Err(e) => {
                        log!("Failed to create batch request: {:?}", e);
                        processed += chunk.len();
                        set_progress.set(Some((processed, total)));
                    }
                }
            }

            // Сбросить прогресс и флаг, перезагрузить (выделение НЕ сбрасываем)
            set_progress.set(None);
            set_processing.set(false);
            reload();
        });
    };

    // Автоматическая загрузка при открытии
    load_sales();

    view! {
        <div class="wb-sales-list">
            {move || {
                if let Some(id) = selected_id.get() {
                    view! {
                        <div class="modal-overlay" style="align-items: flex-start; padding-top: 40px;">
                            <div class="modal-content" style="max-width: 1200px; height: calc(100vh - 80px); overflow: hidden; margin: 0;">
                                <WbSalesDetail
                                    id=id
                                    on_close=move || set_selected_id.set(None)
                                />
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div>
                            <div style="display: flex; align-items: center; gap: 12px; margin-bottom: 8px; flex-wrap: wrap;">
                                <h2 style="margin: 0; font-size: var(--font-size-h3); line-height: 1.2;">"Wildberries Sales (A012)"</h2>

                                <label style="margin: 0; font-size: var(--font-size-sm); white-space: nowrap;">"От:"</label>
                                <input
                                    type="date"
                                    prop:value=date_from
                                    on:input=move |ev| {
                                        set_date_from.set(event_target_value(&ev));
                                    }
                                    disabled=move || is_batch_processing.get()
                                    style="padding: 4px 8px; border: 1px solid #ddd; border-radius: 4px; font-size: var(--font-size-sm);"
                                />

                                <label style="margin: 0; font-size: var(--font-size-sm); white-space: nowrap;">"До:"</label>
                                <input
                                    type="date"
                                    prop:value=date_to
                                    on:input=move |ev| {
                                        set_date_to.set(event_target_value(&ev));
                                    }
                                    disabled=move || is_batch_processing.get()
                                    style="padding: 4px 8px; border: 1px solid #ddd; border-radius: 4px; font-size: var(--font-size-sm);"
                                />

                                <button
                                    on:click=move |_| {
                                        load_sales();
                                    }
                                    style="padding: 4px 12px; background: #4CAF50; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: var(--font-size-sm); transition: all 0.2s;"
                                    onmouseenter="this.style.opacity='0.85'; this.style.transform='translateY(-2px)'"
                                    onmouseleave="this.style.opacity='1'; this.style.transform='translateY(0)'"
                                    disabled=move || loading.get() || is_batch_processing.get()
                                >
                                    {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                                </button>

                                <button
                                    on:click=move |_| {
                                        let data = get_sorted_items();
                                        if let Err(e) = export_to_csv(&data) {
                                            log!("Failed to export: {}", e);
                                        }
                                    }
                                    style="padding: 4px 12px; background: #2196F3; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: var(--font-size-sm); transition: all 0.2s;"
                                    onmouseenter="this.style.opacity='0.85'; this.style.transform='translateY(-2px)'"
                                    onmouseleave="this.style.opacity='1'; this.style.transform='translateY(0)'"
                                    disabled=move || loading.get() || sales.get().is_empty() || is_batch_processing.get()
                                >
                                    "Экспорт в Excel"
                                </button>

                                <div style="margin-bottom: 8px;">
                                    {move || {
                                        let (count, total_qty, total_amount, total_price, total_finished) = totals();
                                        let limit_warning = if !loading.get() && count >= 20000 {
                                            view! {
                                                <span style="margin-left: 8px; padding: 6px 12px; background: #fff3cd; color: #856404; border-radius: 4px; font-size: var(--font-size-sm);">
                                                    "⚠️ Показаны первые 20000 записей. Уточните период для полной загрузки."
                                                </span>
                                            }.into_any()
                                        } else {
                                            view! { <></> }.into_any()
                                        };
                                        view! {
                                            <>
                                                <span style="font-size: var(--font-size-base); font-weight: 600; color: var(--color-text); background: var(--color-background-alt, #f5f5f5); padding: 6px 12px; border-radius: 4px;">
                                                    "Total: " {format_number_with_separator(count as f64, 0)} " records | "
                                                    "Кол-во: " {format_number_with_separator(total_qty, 0)} " | "
                                                    "К выплате: " {format_number_with_separator(total_amount, 2)} " | "
                                                    "Полная цена: " {format_number_with_separator(total_price, 2)} " | "
                                                    "Итоговая: " {format_number_with_separator(total_finished, 2)}
                                                </span>
                                                {limit_warning}
                                            </>
                                        }
                                    }}
                                </div>
                            </div>

                            {move || {
                                // Панель управления выделением - показываем только если есть выделенные строки или идет обработка
                                let selected_count = selected_ids.get().len();
                                let is_processing = batch_progress.get().is_some();

                                if selected_count > 0 || is_processing {
                                    // Подсчитываем итоги по выделенным строкам
                                    let selected_totals = move || {
                                        let sel_ids = selected_ids.get();
                                        let all_items = get_sorted_items();
                                        let selected_items: Vec<_> = all_items.into_iter()
                                            .filter(|item| sel_ids.contains(&item.id))
                                            .collect();

                                        let count = selected_items.len();
                                        let total_qty: f64 = selected_items.iter().map(|s| s.qty).sum();
                                        let total_amount: f64 = selected_items.iter().filter_map(|s| s.amount_line).sum();
                                        let total_price: f64 = selected_items.iter().filter_map(|s| s.total_price).sum();
                                        let total_finished: f64 = selected_items.iter().filter_map(|s| s.finished_price).sum();

                                        (count, total_qty, total_amount, total_price, total_finished)
                                    };

                                    let (sel_count, sel_qty, sel_amount, sel_price, sel_finished) = selected_totals();

                                    // Вычисляем прогресс для фонового градиента
                                    let progress_percent = if let Some((processed, total)) = batch_progress.get() {
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
                                            // В начале обработки - полностью белый фон
                                            "background: #ffffff; border: 1px solid #4CAF50; border-radius: 4px; padding: 8px 12px; margin-bottom: 8px; transition: background 0.3s ease;".to_string()
                                        } else {
                                            format!("background: linear-gradient(to right, #c8e6c9 {}%, #ffffff {}%); border: 1px solid #4CAF50; border-radius: 4px; padding: 8px 12px; margin-bottom: 8px; transition: background 0.3s ease;", progress_percent, progress_percent)
                                        }
                                    } else {
                                        "background: #c8e6c9; border: 1px solid #4CAF50; border-radius: 4px; padding: 8px 12px; margin-bottom: 8px;".to_string()
                                    };

                                    view! {
                                        <div style=background_style>
                                            <div style="display: flex; align-items: center; gap: 10px; flex-wrap: wrap;">
                                                <span style="font-weight: 600; color: #2e7d32; font-size: var(--font-size-sm);">
                                                    "Выделено: " {sel_count} " строк"
                                                </span>

                                                <span style="font-size: var(--font-size-sm); color: #424242;">
                                                    "Кол-во: " {format_number_with_separator(sel_qty, 0)} " | "
                                                    "К выплате: " {format_number_with_separator(sel_amount, 2)} " | "
                                                    "Полная цена: " {format_number_with_separator(sel_price, 2)} " | "
                                                    "Итоговая: " {format_number_with_separator(sel_finished, 2)}
                                                </span>

                                                <div style="margin-left: auto; display: flex; gap: 6px; align-items: center;">
                                                    <button
                                                        on:click=move |_| batch_update(true)
                                                        disabled=move || selected_ids.get().is_empty() || is_batch_processing.get()
                                                        style="padding: 4px 12px; background: #4CAF50; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: var(--font-size-sm); transition: all 0.2s;"
                                                        onmouseenter="this.style.opacity='0.85'; this.style.transform='translateY(-2px)'"
                                                        onmouseleave="this.style.opacity='1'; this.style.transform='translateY(0)'"
                                                        title="Провести выбранные документы"
                                                    >
                                                        {move || format!("✓ Post ({})", selected_ids.get().len())}
                                                    </button>
                                                    <button
                                                        on:click=move |_| batch_update(false)
                                                        disabled=move || selected_ids.get().is_empty() || is_batch_processing.get()
                                                        style="padding: 4px 12px; background: #FF9800; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: var(--font-size-sm); transition: all 0.2s;"
                                                        onmouseenter="this.style.opacity='0.85'; this.style.transform='translateY(-2px)'"
                                                        onmouseleave="this.style.opacity='1'; this.style.transform='translateY(0)'"
                                                        title="Снять проведение выбранных документов"
                                                    >
                                                        {move || format!("✗ Unpost ({})", selected_ids.get().len())}
                                                    </button>
                                                    <button
                                                        on:click=move |_| clear_selection()
                                                        disabled=move || selected_ids.get().is_empty() || is_batch_processing.get()
                                                        style="padding: 4px 12px; background: #9e9e9e; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: var(--font-size-sm); transition: all 0.2s;"
                                                        onmouseenter="this.style.opacity='0.85'; this.style.transform='translateY(-2px)'"
                                                        onmouseleave="this.style.opacity='1'; this.style.transform='translateY(0)'"
                                                        title="Очистить выделение"
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

            {move || {
                // Render summary and table; render filled rows only when not loading and no error
                if !loading.get() && error.get().is_none() {
                    view! {
                        <div>
                            <div class="table-container">
                                <table class="data-table" style="width: 100%; border-collapse: collapse;">
                                    <thead>
                                        <tr style="background: #f5f5f5;">
                                                    <th style="border: 1px solid #ddd; padding: 8px; text-align: center; width: 38px;">
                                                        <input
                                                            type="checkbox"
                                                            on:change=move |ev| {
                                                                let checked = event_target_checked(&ev);
                                                                select_all_current(checked);
                                                            }
                                                            prop:checked=move || {
                                                                let total = get_sorted_items().len();
                                                                let sel = selected_ids.with(|ids| ids.len());
                                                                total > 0 && sel == total
                                                            }
                                                            disabled=move || is_batch_processing.get()
                                                        />
                                                    </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("document_no")
                                                title="Сортировать"
                                            >
                                                {move || format!("Document №{}", get_sort_indicator(&sort_field.get(), "document_no", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("sale_date")
                                                title="Сортировать"
                                            >
                                                {move || format!("Дата продажи{}", get_sort_indicator(&sort_field.get(), "sale_date", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("organization_name")
                                                title="Сортировать"
                                            >
                                                {move || format!("Организация{}", get_sort_indicator(&sort_field.get(), "organization_name", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("supplier_article")
                                                title="Сортировать"
                                            >
                                                {move || format!("Артикул{}", get_sort_indicator(&sort_field.get(), "supplier_article", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("marketplace_article")
                                                title="Сортировать"
                                            >
                                                {move || format!("Артикул МП{}", get_sort_indicator(&sort_field.get(), "marketplace_article", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("nomenclature_article")
                                                title="Сортировать"
                                            >
                                                {move || format!("Артикул 1С{}", get_sort_indicator(&sort_field.get(), "nomenclature_article", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("nomenclature_code")
                                                title="Сортировать"
                                            >
                                                {move || format!("Код 1С{}", get_sort_indicator(&sort_field.get(), "nomenclature_code", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("name")
                                                title="Сортировать"
                                            >
                                                {move || format!("Название{}", get_sort_indicator(&sort_field.get(), "name", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; text-align: right; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("qty")
                                                title="Сортировать"
                                            >
                                                {move || format!("Кол-во{}", get_sort_indicator(&sort_field.get(), "qty", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; text-align: right; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("amount_line")
                                                title="Сортировать"
                                            >
                                                {move || format!("К выплате{}", get_sort_indicator(&sort_field.get(), "amount_line", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; text-align: right; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("total_price")
                                                title="Сортировать"
                                            >
                                                {move || format!("Полная цена{}", get_sort_indicator(&sort_field.get(), "total_price", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; text-align: right; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("finished_price")
                                                title="Сортировать"
                                            >
                                                {move || format!("Итоговая цена{}", get_sort_indicator(&sort_field.get(), "finished_price", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("event_type")
                                                title="Сортировать"
                                            >
                                                {move || format!("Тип{}", get_sort_indicator(&sort_field.get(), "event_type", sort_ascending.get()))}
                                            </th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {move || get_sorted_items().into_iter().map(|sale| {
                                            let sale_id = sale.id.clone();
                                            let sale_id_for_check = sale_id.clone();
                                            let sale_id_for_change = sale_id.clone();
                                            let sale_id_for_row = sale_id.clone();
                                            let formatted_date = format_date(&sale.sale_date);
                                            let formatted_amount = sale.amount_line
                                                .map(|a| format!("{:.2}", a))
                                                .unwrap_or_else(|| "-".to_string());
                                            let formatted_total_price = sale.total_price
                                                .map(|a| format!("{:.2}", a))
                                                .unwrap_or_else(|| "-".to_string());
                                            let formatted_finished_price = sale.finished_price
                                                .map(|a| format!("{:.2}", a))
                                                .unwrap_or_else(|| "-".to_string());
                                            let formatted_qty = format!("{:.0}", sale.qty);
                                            view! {
                                                <tr
                                                    on:click=move |_| {
                                                        set_selected_id.set(Some(sale_id_for_row.clone()));
                                                    }
                                                    style="cursor: pointer; transition: background 0.2s;"
                                                    onmouseenter="this.style.background='#f5f5f5'"
                                                    onmouseleave="this.style.background='white'"
                                                >
                                                    <td
                                                        style="border: 1px solid #ddd; padding: 8px; text-align: center;"
                                                        on:click=move |ev| {
                                                            ev.stop_propagation();
                                                        }
                                                    >
                                                        <input
                                                            type="checkbox"
                                                            prop:checked=move || is_selected(&sale_id_for_check)
                                                            on:change=move |_| toggle_row(sale_id_for_change.clone())
                                                            disabled=move || is_batch_processing.get()
                                                        />
                                                    </td>
                                                    <td style="border: 1px solid #ddd; padding: 8px;">{sale.document_no}</td>
                                                    <td style="border: 1px solid #ddd; padding: 8px;">{formatted_date}</td>
                                                    <td style="border: 1px solid #ddd; padding: 8px;">{sale.organization_name.clone().unwrap_or_else(|| "—".to_string())}</td>
                                                    <td style="border: 1px solid #ddd; padding: 8px;">{sale.supplier_article}</td>
                                                    <td style="border: 1px solid #ddd; padding: 8px;"><span style="color: #1976d2; font-weight: 600;">{sale.marketplace_article.clone().unwrap_or_else(|| "—".to_string())}</span></td>
                                                    <td style="border: 1px solid #ddd; padding: 8px;"><span style="color: #2e7d32; font-weight: 600;">{sale.nomenclature_article.clone().unwrap_or_else(|| "—".to_string())}</span></td>
                                                    <td style="border: 1px solid #ddd; padding: 8px;"><span style="color: #2e7d32; font-weight: 600;">{sale.nomenclature_code.clone().unwrap_or_else(|| "—".to_string())}</span></td>
                                                    <td style="border: 1px solid #ddd; padding: 8px;">{sale.name}</td>
                                                    <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{formatted_qty}</td>
                                                    <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{formatted_amount}</td>
                                                    <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{formatted_total_price}</td>
                                                    <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{formatted_finished_price}</td>
                                                    <td style="border: 1px solid #ddd; padding: 8px;">{sale.event_type}</td>
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
                        "No data".to_string()
                    };
                    view! {
                        <div>
                            <p style="margin: 4px 0 8px 0; font-size: 13px; color: #666;">{msg}</p>
                            <div class="table-container">
                                <table class="data-table" style="width: 100%; border-collapse: collapse;">
                                    <thead>
                                        <tr style="background: #f5f5f5;">
                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: center; width: 38px;"></th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Document №"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Дата продажи"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Организация"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Артикул"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Артикул МП"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Артикул 1С"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Код 1С"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Название"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Кол-во"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"К выплате"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Полная цена"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Итоговая цена"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Тип"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                                <tr><td colspan="14"></td></tr>
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

/// Экспорт WB Sales в CSV для Excel
fn export_to_csv(data: &[WbSalesDto]) -> Result<(), String> {
    // UTF-8 BOM для правильного отображения кириллицы в Excel
    let mut csv = String::from("\u{FEFF}");

    // Заголовок с точкой с запятой как разделитель
    csv.push_str("Document №;Дата продажи;Организация;Артикул;Артикул МП;Артикул 1С;Код 1С;Название;Количество;К выплате;Полная цена;Итоговая цена;Тип\n");

    for sale in data {
        let sale_date = format_date(&sale.sale_date);
        let org_name = sale
            .organization_name
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("—");
        let mp_article = sale
            .marketplace_article
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("—");
        let nom_code = sale
            .nomenclature_code
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("—");
        let nom_article = sale
            .nomenclature_article
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("—");

        // Форматируем суммы с запятой как десятичный разделитель
        let qty_str = format!("{:.0}", sale.qty);
        let amount_str = sale
            .amount_line
            .map(|a| format!("{:.2}", a).replace(".", ","))
            .unwrap_or_else(|| "—".to_string());
        let total_price_str = sale
            .total_price
            .map(|a| format!("{:.2}", a).replace(".", ","))
            .unwrap_or_else(|| "—".to_string());
        let finished_price_str = sale
            .finished_price
            .map(|a| format!("{:.2}", a).replace(".", ","))
            .unwrap_or_else(|| "—".to_string());

        csv.push_str(&format!(
            "\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";{};{};{};{};\"{}\"\n",
            sale.document_no.replace('\"', "\"\""),
            sale_date,
            org_name.replace('\"', "\"\""),
            sale.supplier_article.replace('\"', "\"\""),
            mp_article.replace('\"', "\"\""),
            nom_article.replace('\"', "\"\""),
            nom_code.replace('\"', "\"\""),
            sale.name.replace('\"', "\"\""),
            qty_str,
            amount_str,
            total_price_str,
            finished_price_str,
            sale.event_type.replace('\"', "\"\"")
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
        "wb_sales_{}.csv",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    a.set_download(&filename);
    a.click();

    // Освобождаем URL
    Url::revoke_object_url(&url).map_err(|e| format!("Failed to revoke URL: {:?}", e))?;

    Ok(())
}
