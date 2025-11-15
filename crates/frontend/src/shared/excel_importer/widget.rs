use super::parser::read_excel_from_file;
use super::types::{ColumnDef, ExcelData};
use crate::shared::icons::icon;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[component]
pub fn ExcelImporter(
    /// Определение ожидаемых колонок
    columns: Vec<ColumnDef>,
    /// Callback при успешном импорте
    on_import: Callback<ExcelData>,
    /// Callback при отмене
    on_cancel: Callback<()>,
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

    // Обработка импорта
    let handle_import = move |_| {
        if let Some(data) = excel_data.get() {
            on_import.run(data);
        }
    };

    // Обработка отмены
    let handle_cancel = move |_| {
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
        <div style="position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 10000;">
            <div style="background: white; border-radius: 8px; box-shadow: 0 4px 20px rgba(0,0,0,0.2); width: 85%; height: 85vh; display: flex; flex-direction: column; overflow: hidden; min-width: 700px; max-width: 1400px;">

                // Компактный заголовок с кнопкой Отмена
                <div style="padding: 12px 20px; border-bottom: 1px solid #ddd; background: #f9f9f9; display: flex; justify-content: space-between; align-items: center;">
                    <h2 style="margin: 0; font-size: 18px; color: #333; display: flex; align-items: center; gap: 8px;">
                        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="color: #217346;">
                            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                            <polyline points="14 2 14 8 20 8"/>
                            <line x1="9" y1="13" x2="15" y2="17"/>
                            <line x1="15" y1="13" x2="9" y2="17"/>
                        </svg>
                        {"Импорт из Excel"}
                    </h2>
                    <button
                        class="btn btn-secondary"
                        on:click=handle_cancel
                        style="padding: 6px 16px; font-size: 14px;"
                    >
                        {icon("x")}
                        {"Отмена"}
                    </button>
                </div>

                // Выбор файла
                <div style="padding: 12px 20px; border-bottom: 1px solid #eee; background: #fafafa;">
                    <div style="display: flex; align-items: center; gap: 12px;">
                        <label
                            class="btn"
                            style="background: #2196F3; color: white; padding: 8px 16px; font-size: 14px; cursor: pointer; margin: 0;"
                            for="excel-file-input"
                        >
                            {icon("file")}
                            {"Выбрать файл xlsx"}
                        </label>
                        <input
                            id="excel-file-input"
                            type="file"
                            accept=".xlsx"
                            on:change=handle_file_select
                            style="display: none;"
                        />
                        {move || if let Some(name) = selected_file_name.get() {
                            let size = selected_file_size.get();
                            view! {
                                <span style="color: #666; font-size: 14px;">
                                    <strong>{name}</strong>
                                    {" ("}
                                    {format!("{:.2} KB", size as f64 / 1024.0)}
                                    {")"}
                                </span>
                            }.into_any()
                        } else {
                            view! {
                                <span style="color: #999; font-size: 14px; font-style: italic;">
                                    {"Файл не выбран"}
                                </span>
                            }.into_any()
                        }}
                    </div>
                </div>

                // Ошибки
                {move || error.get().map(|e| {
                    view! {
                        <div style="margin: 0; padding: 12px 20px; background: #fee; border-bottom: 1px solid #fcc; color: #c33;">
                            <strong>{"Ошибка: "}</strong>
                            {e}
                        </div>
                    }
                })}

                // Загрузка
                {move || if is_loading.get() {
                    view! {
                        <div style="padding: 40px; text-align: center; color: #666;">
                            <div style="font-size: 24px; margin-bottom: 10px;">{"⏳"}</div>
                            <div>{"Обработка файла..."}</div>
                        </div>
                    }.into_any()
                } else if excel_data.get().is_some() {
                    // Закладки и содержимое
                    view! {
                        <>
                            // Закладки
                            <div style="display: flex; border-bottom: 2px solid #ddd; background: #f5f5f5;">
                                <button
                                    on:click=move |_| set_active_tab.set(0)
                                    style=move || format!(
                                        "padding: 10px 20px; border: none; background: {}; color: {}; font-size: 14px; font-weight: 500; cursor: pointer; border-bottom: 3px solid {}; transition: all 0.2s; display: flex; align-items: center; gap: 6px;",
                                        if active_tab.get() == 0 { "white" } else { "transparent" },
                                        if active_tab.get() == 0 { "#333" } else { "#666" },
                                        if active_tab.get() == 0 { "#2196F3" } else { "transparent" }
                                    )
                                >
                                    {icon("columns")}
                                    {"Сопоставление колонок"}
                                </button>
                                <button
                                    on:click=move |_| set_active_tab.set(1)
                                    style=move || format!(
                                        "padding: 10px 20px; border: none; background: {}; color: {}; font-size: 14px; font-weight: 500; cursor: pointer; border-bottom: 3px solid {}; transition: all 0.2s; display: flex; align-items: center; gap: 6px;",
                                        if active_tab.get() == 1 { "white" } else { "transparent" },
                                        if active_tab.get() == 1 { "#333" } else { "#666" },
                                        if active_tab.get() == 1 { "#2196F3" } else { "transparent" }
                                    )
                                >
                                    {icon("eye")}
                                    {"Просмотр данных"}
                                </button>
                                <button
                                    on:click=move |_| set_active_tab.set(2)
                                    style=move || format!(
                                        "padding: 10px 20px; border: none; background: {}; color: {}; font-size: 14px; font-weight: 500; cursor: pointer; border-bottom: 3px solid {}; transition: all 0.2s; display: flex; align-items: center; gap: 6px;",
                                        if active_tab.get() == 2 { "white" } else { "transparent" },
                                        if active_tab.get() == 2 { "#333" } else { "#666" },
                                        if active_tab.get() == 2 { "#2196F3" } else { "transparent" }
                                    )
                                >
                                    {icon("code")}
                                    {"Результат JSON"}
                                </button>
                            </div>

                            // Содержимое закладок
                            <div style="flex: 1; overflow: hidden; display: flex; flex-direction: column;">
                                {move || match active_tab.get() {
                                    0 => {
                                        // Закладка: Сопоставление колонок
                                        if let Some(data) = excel_data.get() {
                                            view! {
                                                <div style="flex: 1; overflow: auto; padding: 15px 20px; display: flex; flex-direction: column;">
                                                    // Кнопка Импортировать в центре
                                                    <div style="display: flex; justify-content: center; margin-bottom: 20px;">
                                                        <button
                                                            class="btn"
                                                            style="background: #4CAF50; color: white; padding: 12px 32px; font-size: 15px; font-weight: 600;"
                                                            prop:disabled=move || excel_data.get().is_none()
                                                            on:click=handle_import
                                                        >
                                                            {icon("upload")}
                                                            {move || {
                                                                if let Some(data) = excel_data.get() {
                                                                    format!("Импортировать {} строк", data.metadata.row_count)
                                                                } else {
                                                                    "Импортировать".to_string()
                                                                }
                                                            }}
                                                        </button>
                                                    </div>

                                                    // Прокручиваемая таблица сопоставления
                                                    <div style="flex: 1; overflow-y: auto; overflow-x: hidden; display: flex; justify-content: center;">
                                                        <div style="max-width: fit-content; min-width: 600px;">
                                                            <table style="width: 100%; border-collapse: collapse; border: 1px solid #ddd; border-radius: 4px; background: white; font-size: 13px;">
                                                                <thead style="position: sticky; top: 0; background: #f5f5f5; z-index: 10;">
                                                                    <tr>
                                                                        <th style="padding: 10px 12px; text-align: center; font-weight: 600; color: #333; border-bottom: 2px solid #ddd; width: 50px;"></th>
                                                                        <th style="padding: 10px 12px; text-align: left; font-weight: 600; color: #333; border-bottom: 2px solid #ddd;">{"Формат"}</th>
                                                                        <th style="padding: 10px 12px; text-align: left; font-weight: 600; color: #333; border-bottom: 2px solid #ddd;">{"Колонка"}</th>
                                                                    </tr>
                                                                </thead>
                                                                <tbody>
                                                                    {data.column_mapping.iter().map(|mapping| {
                                                                        let is_exact = mapping.found.as_ref()
                                                                            .map(|f| f.to_lowercase() == mapping.expected.to_lowercase())
                                                                            .unwrap_or(false);
                                                                        let is_mapped = mapping.found.is_some();

                                                                        let (bg_color, icon, icon_color) = if is_exact {
                                                                            ("#f0f9ff", "✓", "#22c55e")
                                                                        } else if is_mapped {
                                                                            ("#fffbeb", "!", "#d97706")
                                                                        } else {
                                                                            ("#fef2f2", "✗", "#ef4444")
                                                                        };

                                                                        let excel_col = if let Some(file_idx) = mapping.file_index {
                                                                            index_to_excel_column(file_idx)
                                                                        } else {
                                                                            String::new()
                                                                        };

                                                                        view! {
                                                                            <tr style=format!("background: {};", bg_color)>
                                                                                <td style=format!("padding: 8px 12px; text-align: center; border-bottom: 1px solid #eee; font-weight: 700; font-size: 16px; color: {};", icon_color)>
                                                                                    {icon}
                                                                                </td>
                                                                                <td style="padding: 8px 12px; border-bottom: 1px solid #eee;">
                                                                                    <strong>{mapping.expected.clone()}</strong>
                                                                                </td>
                                                                                <td style="padding: 8px 12px; border-bottom: 1px solid #eee;">
                                                                                    {if let Some(found) = &mapping.found {
                                                                                        view! {
                                                                                            <span>
                                                                                                <strong>{found.clone()}</strong>
                                                                                                {if !excel_col.is_empty() {
                                                                                                    view! {
                                                                                                        <span style="margin-left: 8px; color: #999; font-size: 12px;">
                                                                                                            {"("}
                                                                                                            {excel_col.clone()}
                                                                                                            {")"}
                                                                                                        </span>
                                                                                                    }.into_any()
                                                                                                } else {
                                                                                                    view! { <></> }.into_any()
                                                                                                }}
                                                                                            </span>
                                                                                        }.into_any()
                                                                                    } else {
                                                                                        view! {
                                                                                            <span style="color: #999; font-style: italic;">
                                                                                                {"не найдено"}
                                                                                            </span>
                                                                                        }.into_any()
                                                                                    }}
                                                                                </td>
                                                                            </tr>
                                                                        }
                                                                    }).collect_view()}
                                                                </tbody>
                                                            </table>
                                                        </div>
                                                    </div>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <></> }.into_any()
                                        }
                                    },
                                    1 => {
                                        // Закладка: Просмотр данных
                                        if let Some(data) = excel_data.get() {
                                            let preview_rows = data.rows.iter().take(50).collect::<Vec<_>>();

                                            view! {
                                                <div style="flex: 1; overflow: auto; padding: 15px 20px;">
                                                    <div style="margin-bottom: 12px; display: flex; justify-content: space-between; align-items: center;">
                                                        <h3 style="margin: 0; font-size: 16px; color: #333;">
                                                            {"Просмотр данных"}
                                                        </h3>
                                                        <div style="color: #666; font-size: 13px;">
                                                            {"Показано: "}
                                                            <strong>{preview_rows.len()}</strong>
                                                            {" из "}
                                                            <strong>{data.metadata.row_count}</strong>
                                                        </div>
                                                    </div>

                                                    <div style="overflow: auto; border: 1px solid #ddd; border-radius: 4px;">
                                                        <table style="width: 100%; border-collapse: collapse; font-size: 13px; background: white;">
                                                            <thead style="position: sticky; top: 0; background: #f5f5f5; z-index: 10; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                                                                <tr>
                                                                    <th style="padding: 8px 10px; text-align: left; width: 40px; color: #666; border-bottom: 2px solid #ddd;">"#"</th>
                                                                    {columns_signal.0.get().iter().map(|col| {
                                                                        view! {
                                                                            <th style="padding: 8px 10px; text-align: left; font-weight: 600; color: #333; border-bottom: 2px solid #ddd;">
                                                                                {col.title.clone()}
                                                                            </th>
                                                                        }
                                                                    }).collect_view()}
                                                                </tr>
                                                            </thead>
                                                            <tbody>
                                                                {preview_rows.into_iter().enumerate().map(|(idx, row)| {
                                                                    let bg = if idx % 2 == 0 { "#fff" } else { "#fafafa" };
                                                                    let cols = columns_signal.0.get();
                                                                    view! {
                                                                        <tr style=format!("background: {};", bg)>
                                                                            <td style="padding: 6px 10px; color: #999; border-bottom: 1px solid #eee;">{idx + 1}</td>
                                                                            {cols.iter().map(|col| {
                                                                                let value = row.get(&col.field_name).cloned().unwrap_or_default();
                                                                                let value_for_title = value.clone();
                                                                                view! {
                                                                                    <td style="padding: 6px 10px; border-bottom: 1px solid #eee; max-width: 300px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;" title=value_for_title>
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
                                    },
                                    2 => {
                                        // Закладка: JSON
                                        if let Some(data) = excel_data.get() {
                                            if let Ok(json) = data.to_json_pretty() {
                                                let json_for_display = json.clone();
                                                let json_for_stats = json.clone();

                                                view! {
                                                    <div style="flex: 1; overflow: auto; padding: 15px 20px; display: flex; flex-direction: column;">
                                                        <div style="margin-bottom: 12px;">
                                                            <h3 style="margin: 0 0 8px 0; font-size: 16px; color: #333;">
                                                                {"Результат JSON"}
                                                            </h3>
                                                            <div style="color: #666; font-size: 13px;">
                                                                {"Размер: "}
                                                                <strong>{format!("{} символов", json_for_stats.len())}</strong>
                                                                {" | Строк: "}
                                                                <strong>{json_for_stats.lines().count()}</strong>
                                                            </div>
                                                        </div>

                                                        <div style="flex: 1; overflow: auto; border: 1px solid #ddd; border-radius: 4px; background: #f5f5f5;">
                                                            <pre style="margin: 0; padding: 15px; font-family: 'Courier New', monospace; font-size: 12px; line-height: 1.5; color: #333; white-space: pre; overflow-x: auto;">
                                                                {json_for_display}
                                                            </pre>
                                                        </div>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <div style="padding: 40px; text-align: center; color: #999;">
                                                        {"Ошибка формирования JSON"}
                                                    </div>
                                                }.into_any()
                                            }
                                        } else {
                                            view! { <></> }.into_any()
                                        }
                                    },
                                    _ => view! { <></> }.into_any()
                                }}
                            </div>
                        </>
                    }.into_any()
                } else {
                    view! {
                        <div style="padding: 40px; text-align: center; color: #999;">
                            <div style="font-size: 48px; margin-bottom: 15px;">{"📁"}</div>
                            <div style="font-size: 16px;">{"Выберите файл Excel для импорта"}</div>
                        </div>
                    }.into_any()
                }}
            </div>
        </div>
    }
}
