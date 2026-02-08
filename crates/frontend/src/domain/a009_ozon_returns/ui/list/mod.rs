use super::details::OzonReturnsDetail;
use crate::shared::api_utils::api_base;
use crate::shared::list_utils::{get_sort_indicator, Sortable};
use crate::shared::modal_stack::ModalStackService;
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Форматирует ISO 8601 дату в dd.mm.yyyy
fn format_date(iso_date: &str) -> String {
    // Парсим ISO 8601: "2025-11-05"
    if let Some((year, rest)) = iso_date.split_once('-') {
        if let Some((month, day)) = rest.split_once('-') {
            return format!("{}.{}.{}", day, month, year);
        }
    }
    iso_date.to_string() // fallback
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonReturnsDto {
    pub id: String,
    #[serde(rename = "returnId")]
    pub return_id: String,
    #[serde(rename = "returnDate")]
    pub return_date: String,
    #[serde(rename = "returnType")]
    pub return_type: String,
    #[serde(rename = "returnReasonName")]
    pub return_reason_name: String,
    #[serde(rename = "orderNumber")]
    pub order_number: String,
    #[serde(rename = "postingNumber")]
    pub posting_number: String,
    pub sku: String,
    #[serde(rename = "productName")]
    pub product_name: String,
    pub quantity: i32,
    pub price: f64,
    #[serde(rename = "isPosted")]
    pub is_posted: bool, // Флаг проведения
}

impl Sortable for OzonReturnsDto {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "return_id" => self
                .return_id
                .to_lowercase()
                .cmp(&other.return_id.to_lowercase()),
            "return_date" => self.return_date.cmp(&other.return_date),
            "return_type" => self
                .return_type
                .to_lowercase()
                .cmp(&other.return_type.to_lowercase()),
            "return_reason" => self
                .return_reason_name
                .to_lowercase()
                .cmp(&other.return_reason_name.to_lowercase()),
            "order_number" => self
                .order_number
                .to_lowercase()
                .cmp(&other.order_number.to_lowercase()),
            "posting_number" => self
                .posting_number
                .to_lowercase()
                .cmp(&other.posting_number.to_lowercase()),
            "sku" => self.sku.to_lowercase().cmp(&other.sku.to_lowercase()),
            "product_name" => self
                .product_name
                .to_lowercase()
                .cmp(&other.product_name.to_lowercase()),
            "quantity" => self.quantity.cmp(&other.quantity),
            "price" => self
                .price
                .partial_cmp(&other.price)
                .unwrap_or(Ordering::Equal),
            "is_posted" => self.is_posted.cmp(&other.is_posted),
            _ => Ordering::Equal,
        }
    }
}

#[component]
pub fn OzonReturnsList() -> impl IntoView {
    let modal_stack =
        use_context::<ModalStackService>().expect("ModalStackService not found in context");
    let (returns, set_returns) = signal::<Vec<OzonReturnsDto>>(Vec::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (selected_id, set_selected_id) = signal::<Option<String>>(None);
    let (detail_reload_trigger, set_detail_reload_trigger) = signal::<u32>(0);

    // Сортировка
    let (sort_field, set_sort_field) = signal::<String>("return_date".to_string());
    let (sort_ascending, set_sort_ascending) = signal(false); // По умолчанию - новые сначала

    // Множественный выбор
    let (selected_ids, set_selected_ids) = signal::<Vec<String>>(Vec::new());

    // Фильтр по периоду
    let (date_from, set_date_from) = signal::<Option<String>>(None);
    let (date_to, set_date_to) = signal::<Option<String>>(None);

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
            Some("ozon-returns-detail-modal".to_string()),
            move |handle| {
                view! {
                    <OzonReturnsDetail
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

    let load_returns = move || {
        let set_returns = set_returns.clone();
        let set_loading = set_loading.clone();
        let set_error = set_error.clone();
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let url = format!("{}/api/ozon_returns", api_base());

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

                                match serde_json::from_str::<Vec<OzonReturnsDto>>(&text) {
                                    Ok(items) => {
                                        log!("Successfully parsed {} OZON returns", items.len());
                                        set_returns.set(items);
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
                    log!("Failed to fetch returns: {:?}", e);
                    set_error.set(Some(format!("Failed to fetch returns: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    // Функция для получения отфильтрованных и отсортированных данных
    let get_filtered_sorted_items = move || -> Vec<OzonReturnsDto> {
        let mut result = returns.get();

        // Фильтр по периоду
        let from = date_from.get();
        let to = date_to.get();
        if from.is_some() || to.is_some() {
            result.retain(|item| {
                let item_date = &item.return_date;
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
            });
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

        let set_returns = set_returns.clone();
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
                let url = format!("{}/api/a009/ozon-returns/{}/post", api_base(), id);
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

            let url = format!("{}/api/ozon_returns", api_base());
            match Request::get(&url).send().await {
                Ok(response) => {
                    if response.status() == 200 {
                        if let Ok(text) = response.text().await {
                            if let Ok(items) = serde_json::from_str::<Vec<OzonReturnsDto>>(&text) {
                                set_returns.set(items);
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

        let set_returns = set_returns.clone();
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
                let url = format!("{}/api/a009/ozon-returns/{}/unpost", api_base(), id);
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

            let url = format!("{}/api/ozon_returns", api_base());
            match Request::get(&url).send().await {
                Ok(response) => {
                    if response.status() == 200 {
                        if let Ok(text) = response.text().await {
                            if let Ok(items) = serde_json::from_str::<Vec<OzonReturnsDto>>(&text) {
                                set_returns.set(items);
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
    load_returns();

    view! {
        <div class="ozon-returns-list">
            {move || {
                if let Some(id) = selected_id.get() {
                    open_detail_modal(id);
                    set_selected_id.set(None);
                    view! { <></> }.into_any()
                } else {
                    view! {
                        <div>
                            <div class="doc-list__toolbar">
                                <h2 class="doc-list__title">"OZON Returns (A009)"</h2>
                                <button class="button button--secondary" on:click=move |_| load_returns()>
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

                                    // Кнопки массовых операций
                                    <div style="display: flex; gap: 10px;">
                                        <button
                                            disabled=move || selected_ids.get().is_empty() || posting_in_progress.get()
                                            on:click=post_selected
                                            class="button button--info"
                                        >
                                            {move || format!("Провести ({})", selected_ids.get().len())}
                                        </button>
                                        <button
                                            disabled=move || selected_ids.get().is_empty() || posting_in_progress.get()
                                            on:click=unpost_selected
                                            class="button button--warning"
                                        >
                                            {move || format!("Отменить ({})", selected_ids.get().len())}
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
                let msg = if loading.get() {
                    "Loading...".to_string()
                } else if let Some(err) = error.get() {
                    err.clone()
                } else {
                    let filtered = get_filtered_sorted_items();
                    format!("Показано: {} записей", filtered.len())
                };

                // Render summary and table
                if !loading.get() && error.get().is_none() {
                    view! {
                        <div>
                            <p style="margin: 4px 0 8px 0; font-size: 13px; color: #666;">{msg}</p>
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
                                                on:click=toggle_sort("return_id")
                                                title="Сортировать"
                                            >
                                                {move || format!("Return ID{}", get_sort_indicator(&sort_field.get(), "return_id", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("return_date")
                                                title="Сортировать"
                                            >
                                                {move || format!("Дата возврата{}", get_sort_indicator(&sort_field.get(), "return_date", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("posting_number")
                                                title="Сортировать"
                                            >
                                                {move || format!("Номер постинга{}", get_sort_indicator(&sort_field.get(), "posting_number", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("return_type")
                                                title="Сортировать"
                                            >
                                                {move || format!("Тип возврата{}", get_sort_indicator(&sort_field.get(), "return_type", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("product_name")
                                                title="Сортировать"
                                            >
                                                {move || format!("Товар{}", get_sort_indicator(&sort_field.get(), "product_name", sort_ascending.get()))}
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
                                                on:click=toggle_sort("quantity")
                                                title="Сортировать"
                                            >
                                                {move || format!("Кол-во{}", get_sort_indicator(&sort_field.get(), "quantity", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; text-align: right; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("price")
                                                title="Сортировать"
                                            >
                                                {move || format!("Цена{}", get_sort_indicator(&sort_field.get(), "price", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=toggle_sort("return_reason")
                                                title="Сортировать"
                                            >
                                                {move || format!("Причина{}", get_sort_indicator(&sort_field.get(), "return_reason", sort_ascending.get()))}
                                            </th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {move || get_filtered_sorted_items().into_iter().map(|ret| {
                                            let short_id = ret.id.chars().take(8).collect::<String>();
                                            let formatted_date = format_date(&ret.return_date);
                                            let formatted_price = format!("{:.2} ₽", ret.price);
                                            let is_posted_flag = ret.is_posted;
                                            let posting_number_display = ret.posting_number.clone();
                                            let return_type_display = ret.return_type.clone();

                                            // Создаем отдельные клоны для каждого обработчика
                                            let id_for_checkbox_change = ret.id.clone();
                                            let id_for_checkbox_check = ret.id.clone();
                                            let id1 = ret.id.clone();
                                            let id2 = ret.id.clone();
                                            let id3 = ret.id.clone();
                                            let id4 = ret.id.clone();
                                            let id5 = ret.id.clone();
                                            let id6 = ret.id.clone();
                                            let id7 = ret.id.clone();
                                            let id8 = ret.id.clone();
                                            let id9 = ret.id.clone();
                                            let id10 = ret.id.clone();

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
                                                        {ret.return_id}
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
                                                        {posting_number_display}
                                                    </td>
                                                    <td
                                                        style="border: 1px solid #ddd; padding: 8px; cursor: pointer;"
                                                        on:click=move |_| {
                                                            set_selected_id.set(Some(id9.clone()));
                                                        }
                                                    >
                                                        {return_type_display}
                                                    </td>
                                                    <td
                                                        style="border: 1px solid #ddd; padding: 8px; cursor: pointer;"
                                                        on:click=move |_| {
                                                            set_selected_id.set(Some(id10.clone()));
                                                        }
                                                    >
                                                        {ret.product_name}
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
                                                        {ret.quantity}
                                                    </td>
                                                    <td
                                                        style="border: 1px solid #ddd; padding: 8px; text-align: right; cursor: pointer;"
                                                        on:click=move |_| {
                                                            set_selected_id.set(Some(id7.clone()));
                                                        }
                                                    >
                                                        {formatted_price}
                                                    </td>
                                                    <td
                                                        style="border: 1px solid #ddd; padding: 8px; cursor: pointer;"
                                                        on:click=move |_| {
                                                            set_selected_id.set(Some(id8.clone()));
                                                        }
                                                    >
                                                        {ret.return_reason_name}
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
                    view! {
                        <div>
                            <p style="margin: 4px 0 8px 0; font-size: 13px; color: #666;">{msg}</p>
                            <div class="table-container">
                                <table class="table__data" style="width: 100%; border-collapse: collapse;">
                                    <thead>
                                        <tr style="background: #f5f5f5;">
                                            <th style="border: 1px solid #ddd; padding: 8px; width: 40px;"></th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"ID"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Return ID"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Дата"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Номер постинга"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Тип возврата"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Товар"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: center;">"Статус"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Кол-во"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px; text-align: right;">"Цена"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Причина"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        <tr><td colspan="11"></td></tr>
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
