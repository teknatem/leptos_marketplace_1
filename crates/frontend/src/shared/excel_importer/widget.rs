use super::parser::read_excel_from_file;
use super::types::{ColumnDef, ExcelData};
use crate::shared::icons::icon;
use leptos::prelude::*;
use serde_json::Value;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use thaw::*;

fn api_base() -> String {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return String::new(),
    };
    let location = window.location();
    let protocol = location.protocol().unwrap_or_else(|_| "http:".to_string());
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    format!("{}//{}:3000", protocol, hostname)
}

#[component]
pub fn ExcelImporter(
    /// Определение ожидаемых колонок
    columns: Vec<ColumnDef>,
    /// Backend endpoint for POST (if set, importer will call API and show result inside modal)
    #[prop(optional, into)]
    import_endpoint: Option<String>,
    /// Callback after successful import (e.g. refresh list)
    #[prop(optional)]
    on_success: Option<Callback<()>>,
    /// Fallback callback if import_endpoint is not provided
    #[prop(optional)]
    on_import: Option<Callback<ExcelData>>,
    /// Callback при отмене
    on_cancel: Callback<()>,
    /// Optional close lock shared with ModalStack guard.
    /// When true, overlay/Esc close should be blocked by the host.
    #[prop(optional)]
    close_lock: Option<Arc<std::sync::atomic::AtomicBool>>,
    /// Callback для просмотра JSON
    #[prop(optional)]
    _on_view_json: Option<Callback<String>>,
) -> impl IntoView {
    let (selected_file_name, set_selected_file_name) = signal(Option::<String>::None);
    let (selected_file_size, set_selected_file_size) = signal(0u64);
    let (excel_data, set_excel_data) = signal(Option::<ExcelData>::None);
    let (is_loading, set_is_loading) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);
    let (active_tab, set_active_tab) = signal(0); // 0: маппинг, 1: данные, 2: JSON

    // Import request state (API call)
    let (is_importing, set_is_importing) = signal(false);
    let (import_error, set_import_error) = signal(Option::<String>::None);
    let (import_result_json, set_import_result_json) = signal(Option::<Value>::None);

    // Store non-Copy props so we can safely use them inside reactive closures without FnOnce issues.
    let import_endpoint_sv = StoredValue::new_local(import_endpoint);
    let on_success_sv = StoredValue::new_local(on_success);
    let on_import_sv = StoredValue::new_local(on_import);
    let close_lock_sv = StoredValue::new_local(close_lock);

    // Клонируем columns для использования в разных closure
    let columns_for_file_select = columns.clone();

    // Создаём сигналы для columns чтобы использовать в реактивных контекстах
    let columns_signal = signal(columns.clone());

    // Обработка выбора файла
    let handle_file_select = move |ev: web_sys::Event| {
        let input = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok());

        if let Some(input) = input {
            if let Some(files) = input.files() {
                if let Some(file) = files.get(0) {
                    set_selected_file_name.set(Some(file.name()));
                    set_selected_file_size.set(file.size() as u64);
                    set_error.set(None);
                    set_excel_data.set(None);

                    // Автоматически парсим файл
                    let file_for_parse = file.clone();
                    let columns_clone = columns_for_file_select.clone();

                    set_is_loading.set(true);
                    leptos::task::spawn_local(async move {
                        match read_excel_from_file(file_for_parse.clone()).await {
                            Ok(raw_data) => {
                                let file_name = file_for_parse.name();
                                match ExcelData::from_raw(raw_data, columns_clone, file_name) {
                                    Ok(data) => {
                                        set_excel_data.set(Some(data));
                                        set_error.set(None);
                                    }
                                    Err(e) => {
                                        set_error.set(Some(e));
                                    }
                                }
                            }
                            Err(e) => {
                                set_error.set(Some(e));
                            }
                        }
                        set_is_loading.set(false);
                    });
                }
            }
        }
    };

    // Обработка отмены
    let handle_cancel = move |_| {
        if is_importing.get() {
            return;
        }
        if let Some(lock) = close_lock_sv.get_value() {
            if lock.load(Ordering::Relaxed) {
                return;
            }
        }
        on_cancel.run(());
    };

    // Функция для конвертации индекса в букву колонки Excel (0 -> A, 1 -> B, ..., 25 -> Z, 26 -> AA, ...)
    let index_to_excel_column = |idx: usize| -> String {
        let mut result = String::new();
        let mut n = idx + 1;
        while n > 0 {
            n -= 1;
            result.insert(0, (b'A' + (n % 26) as u8) as char);
            n /= 26;
        }
        result
    };

    view! {
        <div class="excel-importer">
            <div class="modal-header excel-importer__header">
                <h3 class="modal-title excel-importer__title">"Импорт из Excel"</h3>
                <div class="modal-header-actions">
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=handle_cancel
                        disabled=Signal::derive(move || is_importing.get())
                    >
                        {icon("x")}
                        " Закрыть"
                    </Button>
                </div>
            </div>

            <div class="modal-body excel-importer__body">
                <div class="excel-importer__filebar">
                    <div class="excel-importer__filebar-row">
                        <label class="button button--primary excel-importer__file-btn" for="excel-file-input">
                            {icon("file")}
                            " Выбрать файл xlsx"
                        </label>
                        <input
                            id="excel-file-input"
                            type="file"
                            accept=".xlsx"
                            on:change=handle_file_select
                            class="hidden"
                        />
                        {move || if let Some(name) = selected_file_name.get() {
                            let size = selected_file_size.get();
                            view! {
                                <span class="excel-importer__fileinfo">
                                    <strong>{name}</strong>
                                    {" ("}
                                    {format!("{:.2} KB", size as f64 / 1024.0)}
                                    {")"}
                                </span>
                            }.into_any()
                        } else {
                            view! {
                                <span class="excel-importer__filehint">"Файл не выбран"</span>
                            }.into_any()
                        }}
                    </div>
                </div>

                {move || error.get().map(|e| {
                    view! {
                        <div class="warning-box warning-box--error excel-importer__error">
                            <span class="warning-box__icon">"⚠"</span>
                            <span class="warning-box__text">{e}</span>
                        </div>
                    }
                })}

                {move || import_error.get().map(|e| {
                    view! {
                        <div class="warning-box warning-box--error excel-importer__error">
                            <span class="warning-box__icon">"⚠"</span>
                            <span class="warning-box__text">{e}</span>
                        </div>
                    }
                })}

                {move || if is_loading.get() {
                    view! { <div class="loading">"Обработка файла..."</div> }.into_any()
                } else if excel_data.get().is_some() {
                    view! {
                        <>
                            <div class="excel-importer__tabs">
                                <button
                                    class=move || if active_tab.get() == 0 {
                                        "excel-importer__tab excel-importer__tab--active"
                                    } else {
                                        "excel-importer__tab"
                                    }
                                    on:click=move |_| set_active_tab.set(0)
                                >
                                    {icon("columns")}
                                    " Сопоставление"
                                </button>
                                <button
                                    class=move || if active_tab.get() == 1 {
                                        "excel-importer__tab excel-importer__tab--active"
                                    } else {
                                        "excel-importer__tab"
                                    }
                                    on:click=move |_| set_active_tab.set(1)
                                >
                                    {icon("eye")}
                                    " Данные"
                                </button>
                                <button
                                    class=move || if active_tab.get() == 2 {
                                        "excel-importer__tab excel-importer__tab--active"
                                    } else {
                                        "excel-importer__tab"
                                    }
                                    on:click=move |_| set_active_tab.set(2)
                                >
                                    {icon("code")}
                                    " JSON"
                                </button>
                            </div>

                            <div class="excel-importer__content">
                                {move || match active_tab.get() {
                                    0 => {
                                        if let Some(data) = excel_data.get() {
                                            view! {
                                                <div class="excel-importer__tab-pane">
                                                    <div class="excel-importer__actions-center">
                                                        <Button
                                                            appearance=ButtonAppearance::Primary
                                                            on_click={
                                                                move |_| {
                                                                    let Some(data) = excel_data.get() else {
                                                                        return;
                                                                    };

                                                                    // Reset last result on new attempt
                                                                    set_import_error.set(None);
                                                                    set_import_result_json.set(None);

                                                                    let import_endpoint = import_endpoint_sv.get_value();
                                                                    let on_success = on_success_sv.get_value();
                                                                    let on_import = on_import_sv.get_value();
                                                                    let close_lock = close_lock_sv.get_value();

                                                                    if let Some(endpoint) = import_endpoint {
                                                                        if let Some(lock) = &close_lock {
                                                                            lock.store(true, Ordering::Relaxed);
                                                                        }
                                                                        set_is_importing.set(true);
                                                                        let body = match serde_json::to_string(&data) {
                                                                            Ok(v) => v,
                                                                            Err(e) => {
                                                                                set_is_importing.set(false);
                                                                                if let Some(lock) = &close_lock {
                                                                                    lock.store(false, Ordering::Relaxed);
                                                                                }
                                                                                set_import_error.set(Some(format!("Ошибка сериализации: {e}")));
                                                                                return;
                                                                            }
                                                                        };

                                                                        leptos::task::spawn_local(async move {
                                                                            use wasm_bindgen::JsCast;
                                                                            use web_sys::{Request, RequestInit, RequestMode, Response};

                                                                            let opts = RequestInit::new();
                                                                            opts.set_method("POST");
                                                                            opts.set_mode(RequestMode::Cors);
                                                                            opts.set_body(&wasm_bindgen::JsValue::from_str(&body));

                                                                            let url = if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
                                                                                endpoint
                                                                            } else if endpoint.starts_with('/') {
                                                                                format!("{}{}", api_base(), endpoint)
                                                                            } else {
                                                                                format!("{}/{}", api_base(), endpoint)
                                                                            };

                                                                            let request = Request::new_with_str_and_init(&url, &opts)
                                                                                .map_err(|e| format!("{e:?}"));
                                                                            let Ok(request) = request else {
                                                                                set_is_importing.set(false);
                                                                                if let Some(lock) = &close_lock {
                                                                                    lock.store(false, Ordering::Relaxed);
                                                                                }
                                                                                set_import_error.set(Some("Ошибка формирования запроса".to_string()));
                                                                                return;
                                                                            };

                                                                            if request.headers().set("Content-Type", "application/json").is_err() {
                                                                                set_is_importing.set(false);
                                                                                if let Some(lock) = &close_lock {
                                                                                    lock.store(false, Ordering::Relaxed);
                                                                                }
                                                                                set_import_error.set(Some("Ошибка заголовков запроса".to_string()));
                                                                                return;
                                                                            }

                                                                            let window = match web_sys::window() {
                                                                                Some(w) => w,
                                                                                None => {
                                                                                    set_is_importing.set(false);
                                                                                    if let Some(lock) = &close_lock {
                                                                                        lock.store(false, Ordering::Relaxed);
                                                                                    }
                                                                                    set_import_error.set(Some("no window".to_string()));
                                                                                    return;
                                                                                }
                                                                            };

                                                                            let resp_value = match wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await {
                                                                                Ok(v) => v,
                                                                                Err(e) => {
                                                                                    set_is_importing.set(false);
                                                                                    if let Some(lock) = &close_lock {
                                                                                        lock.store(false, Ordering::Relaxed);
                                                                                    }
                                                                                    set_import_error.set(Some(format!("Fetch failed: {e:?}")));
                                                                                    return;
                                                                                }
                                                                            };

                                                                            let resp: Response = match resp_value.dyn_into() {
                                                                                Ok(r) => r,
                                                                                Err(e) => {
                                                                                    set_is_importing.set(false);
                                                                                    if let Some(lock) = &close_lock {
                                                                                        lock.store(false, Ordering::Relaxed);
                                                                                    }
                                                                                    set_import_error.set(Some(format!("{e:?}")));
                                                                                    return;
                                                                                }
                                                                            };

                                                                            let status = resp.status();
                                                                            let text = wasm_bindgen_futures::JsFuture::from(
                                                                                resp.text().unwrap_or_else(|_| js_sys::Promise::resolve(&"".into())),
                                                                            )
                                                                            .await
                                                                            .ok()
                                                                            .and_then(|v| v.as_string())
                                                                            .unwrap_or_default();

                                                                            if !resp.ok() {
                                                                                set_is_importing.set(false);
                                                                                if let Some(lock) = &close_lock {
                                                                                    lock.store(false, Ordering::Relaxed);
                                                                                }
                                                                                set_import_error.set(Some(if text.trim().is_empty() {
                                                                                    format!("HTTP {status}")
                                                                                } else {
                                                                                    format!("HTTP {status}: {text}")
                                                                                }));
                                                                                return;
                                                                            }

                                                                            match serde_json::from_str::<Value>(&text) {
                                                                                Ok(v) => {
                                                                                    set_import_result_json.set(Some(v));
                                                                                }
                                                                                Err(_) => {
                                                                                    set_import_result_json.set(Some(Value::String(text)));
                                                                                }
                                                                            }

                                                                            if let Some(cb) = on_success {
                                                                                cb.run(());
                                                                            }

                                                                            set_is_importing.set(false);
                                                                            if let Some(lock) = &close_lock {
                                                                                lock.store(false, Ordering::Relaxed);
                                                                            }
                                                                        });
                                                                    } else if let Some(cb) = on_import {
                                                                        cb.run(data);
                                                                    }
                                                                }
                                                            }
                                                            disabled=Signal::derive(move || excel_data.get().is_none() || is_importing.get())
                                                        >
                                                            {icon("upload")}
                                                            {move || {
                                                                if let Some(data) = excel_data.get() {
                                                                    format!(" Импортировать {} строк", data.metadata.row_count)
                                                                } else {
                                                                    " Импортировать".to_string()
                                                                }
                                                            }}
                                                        </Button>
                                                        <Show when=move || is_importing.get()>
                                                            <Space gap=SpaceGap::Small>
                                                                <Spinner />
                                                                <span style="color: var(--color-text-tertiary);">"Импорт..."</span>
                                                            </Space>
                                                        </Show>
                                                    </div>

                                                    {move || import_result_json.get().map(|v| {
                                                        // Best-effort summary (works for a004 nomenclature ImportResult)
                                                        let updated_count = v.get("updated_count").and_then(|x| x.as_u64());
                                                        let not_found = v.get("not_found_articles")
                                                            .and_then(|x| x.as_array())
                                                            .map(|arr| arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect::<Vec<_>>())
                                                            .unwrap_or_default();

                                                        view! {
                                                            <div style="margin-top: var(--spacing-sm); padding: var(--spacing-sm) var(--spacing-md); border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-surface);">
                                                                <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center>
                                                                    <Space gap=SpaceGap::Small>
                                                                        <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Success>
                                                                            "Импорт выполнен"
                                                                        </Badge>
                                                                        {updated_count.map(|n| view! { <span>{format!("Обновлено: {}", n)}</span> })}
                                                                    </Space>
                                                                </Flex>
                                                                {(!not_found.is_empty()).then(|| view! {
                                                                    <div style="margin-top: 8px; color: var(--color-text-secondary); font-size: 12px;">
                                                                        {format!("Не найдено артикулов: {}", not_found.len())}
                                                                    </div>
                                                                    <div style="margin-top: 6px; font-size: 12px;">
                                                                        {not_found.join(", ")}
                                                                    </div>
                                                                })}
                                                            </div>
                                                        }
                                                    })}

                                                    <div class="excel-importer__table-wrap">
                                                        <table class="table__data table--striped excel-importer__table">
                                                            <thead class="table__head">
                                                                <tr>
                                                                    <th class="table__header-cell excel-importer__status-col"></th>
                                                                    <th class="table__header-cell">"Формат"</th>
                                                                    <th class="table__header-cell">"Колонка"</th>
                                                                </tr>
                                                            </thead>
                                                            <tbody>
                                                                {data.column_mapping.iter().map(|mapping| {
                                                                    let is_exact = mapping.found.as_ref()
                                                                        .map(|f| f.to_lowercase() == mapping.expected.to_lowercase())
                                                                        .unwrap_or(false);
                                                                    let is_mapped = mapping.found.is_some();
                                                                    let row_class = if is_exact {
                                                                        "excel-importer__map-row excel-importer__map-row--exact"
                                                                    } else if is_mapped {
                                                                        "excel-importer__map-row excel-importer__map-row--mapped"
                                                                    } else {
                                                                        "excel-importer__map-row excel-importer__map-row--missing"
                                                                    };
                                                                    let status = if is_exact { "✓" } else if is_mapped { "!" } else { "✗" };
                                                                    let excel_col = if let Some(file_idx) = mapping.file_index {
                                                                        index_to_excel_column(file_idx)
                                                                    } else {
                                                                        String::new()
                                                                    };

                                                                    view! {
                                                                        <tr class=row_class>
                                                                            <td class="table__cell excel-importer__status-cell">{status}</td>
                                                                            <td class="table__cell"><strong>{mapping.expected.clone()}</strong></td>
                                                                            <td class="table__cell">
                                                                                {if let Some(found) = &mapping.found {
                                                                                    view! {
                                                                                        <span>
                                                                                            <strong>{found.clone()}</strong>
                                                                                            {if !excel_col.is_empty() {
                                                                                                view! { <span class="excel-importer__excel-col">{"("}{excel_col.clone()}{")"}</span> }.into_any()
                                                                                            } else {
                                                                                                view! { <></> }.into_any()
                                                                                            }}
                                                                                        </span>
                                                                                    }.into_any()
                                                                                } else {
                                                                                    view! { <span class="excel-importer__not-found">"не найдено"</span> }.into_any()
                                                                                }}
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
                                            view! { <></> }.into_any()
                                        }
                                    }
                                    1 => {
                                        if let Some(data) = excel_data.get() {
                                            let preview_rows = data.rows.iter().take(50).collect::<Vec<_>>();
                                            view! {
                                                <div class="excel-importer__tab-pane">
                                                    <div class="excel-importer__pane-header">
                                                        <h3 class="excel-importer__pane-title">"Просмотр данных"</h3>
                                                        <div class="excel-importer__pane-meta">
                                                            "Показано: " <strong>{preview_rows.len()}</strong>
                                                            " из " <strong>{data.metadata.row_count}</strong>
                                                        </div>
                                                    </div>
                                                    <div class="excel-importer__table-wrap">
                                                        <table class="table__data table--striped excel-importer__table">
                                                            <thead class="table__head">
                                                                <tr>
                                                                    <th class="table__header-cell excel-importer__index-col">"#"</th>
                                                                    {columns_signal.0.get().iter().map(|col| view! {
                                                                        <th class="table__header-cell">{col.title.clone()}</th>
                                                                    }).collect_view()}
                                                                </tr>
                                                            </thead>
                                                            <tbody>
                                                                {preview_rows.into_iter().enumerate().map(|(idx, row)| {
                                                                    let cols = columns_signal.0.get();
                                                                    view! {
                                                                        <tr class="table__row">
                                                                            <td class="table__cell excel-importer__index-cell">{idx + 1}</td>
                                                                            {cols.iter().map(|col| {
                                                                                let value = row.get(&col.field_name).cloned().unwrap_or_default();
                                                                                let value_for_title = value.clone();
                                                                                view! {
                                                                                    <td class="table__cell excel-importer__cell-ellipsis" title=value_for_title>
                                                                                        {value}
                                                                                    </td>
                                                                                }
                                                                            }).collect_view()}
                                                                        </tr>
                                                                    }
                                                                }).collect_view()}
                                                            </tbody>
                                                        </table>
                                                    </div>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <></> }.into_any()
                                        }
                                    }
                                    2 => {
                                        if let Some(data) = excel_data.get() {
                                            if let Ok(json) = data.to_json_pretty() {
                                                let json_for_display = json.clone();
                                                let json_for_stats = json.clone();
                                                view! {
                                                    <div class="excel-importer__tab-pane excel-importer__json-pane">
                                                        <div class="excel-importer__pane-header">
                                                            <h3 class="excel-importer__pane-title">"Результат JSON"</h3>
                                                            <div class="excel-importer__pane-meta">
                                                                "Размер: " <strong>{format!("{} символов", json_for_stats.len())}</strong>
                                                                " | Строк: " <strong>{json_for_stats.lines().count()}</strong>
                                                            </div>
                                                        </div>
                                                        <pre class="excel-importer__json-pre">{json_for_display}</pre>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! { <div class="loading">"Ошибка формирования JSON"</div> }.into_any()
                                            }
                                        } else {
                                            view! { <></> }.into_any()
                                        }
                                    }
                                    _ => view! { <></> }.into_any(),
                                }}
                            </div>
                        </>
                    }.into_any()
                } else {
                    view! {
                        <div class="excel-importer__empty">
                            <div class="excel-importer__empty-icon">"📁"</div>
                            <div class="excel-importer__empty-text">"Выберите файл Excel для импорта"</div>
                        </div>
                    }.into_any()
                }}
            </div>
        </div>
    }
}

