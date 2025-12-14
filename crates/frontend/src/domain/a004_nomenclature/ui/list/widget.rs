use super::super::details::NomenclatureDetails;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::excel_importer::{ColumnDef, DataType, ExcelData, ExcelImporter};
use crate::shared::export::{export_to_excel, ExcelExportable};
use crate::shared::icons::icon;
use contracts::domain::a004_nomenclature::aggregate::Nomenclature;
use contracts::domain::a004_nomenclature::ImportResult;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

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

async fn fetch_nomenclature() -> Result<Vec<Nomenclature>, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/nomenclature", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    let data: Vec<Nomenclature> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}

// Отправить данные на сервер для импорта
async fn send_import_data(excel_data: ExcelData) -> Result<ImportResult, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let body =
        serde_json::to_string(&excel_data).map_err(|e| format!("Ошибка сериализации: {e}"))?;
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body));

    let url = format!("{}/api/nomenclature/import-excel", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Fetch failed: {e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;

    if !resp.ok() {
        let status = resp.status();
        let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
            .await
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "Unknown error".to_string());
        return Err(format!("HTTP {}: {}", status, text));
    }

    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    let result: ImportResult =
        serde_json::from_str(&text).map_err(|e| format!("Parse error: {e}"))?;
    Ok(result)
}

// Реализация ExcelExportable для экспорта в Excel
impl ExcelExportable for Nomenclature {
    fn headers() -> Vec<&'static str> {
        vec![
            "Артикул",
            "Наименование",
            "Категория",
            "Линейка",
            "Модель",
            "Формат",
            "Раковина",
            "Размер",
        ]
    }

    fn to_csv_row(&self) -> Vec<String> {
        vec![
            self.article.clone(),
            self.base.description.clone(),
            self.dim1_category.clone(),
            self.dim2_line.clone(),
            self.dim3_model.clone(),
            self.dim4_format.clone(),
            self.dim5_sink.clone(),
            self.dim6_size.clone(),
        ]
    }
}

#[derive(Clone, Copy, PartialEq)]
enum SortColumn {
    Article,
    Description,
    Dim1Category,
    Dim2Line,
    Dim3Model,
    Dim4Format,
    Dim5Sink,
    Dim6Size,
}

#[derive(Clone, Copy, PartialEq)]
enum SortDirection {
    Asc,
    Desc,
}

#[component]
pub fn NomenclatureList() -> impl IntoView {
    let (all_items, set_all_items) = signal(Vec::<Nomenclature>::new());
    let (is_loading, set_is_loading) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);

    // Фильтры
    let (filter_text, set_filter_text) = signal(String::new());
    let (filter_input, set_filter_input) = signal(String::new());
    let (show_only_mp, set_show_only_mp) = signal(true); // По умолчанию включен

    // Сортировка
    let (sort_column, set_sort_column) = signal(SortColumn::Article);
    let (sort_direction, set_sort_direction) = signal(SortDirection::Asc);

    // Выбранные элементы (по ID)
    let (selected_ids, set_selected_ids) = signal(std::collections::HashSet::<String>::new());

    // Excel Import
    let (show_excel_importer, set_show_excel_importer) = signal(false);

    // Modal for details
    let (show_modal, set_show_modal) = signal(false);
    let (editing_id, set_editing_id) = signal(Option::<String>::None);
    let _tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");

    // Определение колонок для Excel импорта
    let excel_columns = vec![
        ColumnDef {
            field_name: "article".to_string(),
            title: "Артикул".to_string(),
            data_type: DataType::String,
        },
        ColumnDef {
            field_name: "category".to_string(),
            title: "Категория".to_string(),
            data_type: DataType::String,
        },
        ColumnDef {
            field_name: "line".to_string(),
            title: "Линейка".to_string(),
            data_type: DataType::String,
        },
        ColumnDef {
            field_name: "model".to_string(),
            title: "Модель".to_string(),
            data_type: DataType::String,
        },
        ColumnDef {
            field_name: "format".to_string(),
            title: "Формат".to_string(),
            data_type: DataType::String,
        },
        ColumnDef {
            field_name: "sink".to_string(),
            title: "Раковина".to_string(),
            data_type: DataType::String,
        },
        ColumnDef {
            field_name: "size".to_string(),
            title: "Размер".to_string(),
            data_type: DataType::String,
        },
    ];

    // Функция загрузки данных
    let load = move || {
        set_is_loading.set(true);
        set_error.set(None);
        leptos::task::spawn_local(async move {
            match fetch_nomenclature().await {
                Ok(data) => {
                    set_all_items.set(data);
                    set_is_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_is_loading.set(false);
                }
            }
        });
    };

    // Загрузка при монтировании
    leptos::task::spawn_local(async move {
        load();
    });

    // Debounce для поиска
    let handle_input_change = move |val: String| {
        set_filter_input.set(val.clone());
        if val.len() >= 3 || val.is_empty() {
            set_filter_text.set(val);
        }
    };

    // Функция переключения сортировки
    let toggle_sort = move |column: SortColumn| {
        if sort_column.get() == column {
            // Переключаем направление
            set_sort_direction.set(match sort_direction.get() {
                SortDirection::Asc => SortDirection::Desc,
                SortDirection::Desc => SortDirection::Asc,
            });
        } else {
            // Новая колонка - сортировка по возрастанию
            set_sort_column.set(column);
            set_sort_direction.set(SortDirection::Asc);
        }
    };

    // Фильтрация и сортировка данных
    let filtered_items = move || {
        let mut items: Vec<Nomenclature> = all_items
            .get()
            .into_iter()
            .filter(|item| {
                // Исключаем папки
                if item.is_folder {
                    return false;
                }

                // Фильтр "только из маркетплейсов"
                if show_only_mp.get() && item.mp_ref_count == 0 {
                    return false;
                }

                // Текстовый поиск
                let filter = filter_text.get().to_lowercase();
                if filter.is_empty() {
                    return true;
                }

                item.article.to_lowercase().contains(&filter)
                    || item.base.description.to_lowercase().contains(&filter)
                    || item.dim1_category.to_lowercase().contains(&filter)
                    || item.dim2_line.to_lowercase().contains(&filter)
                    || item.dim3_model.to_lowercase().contains(&filter)
                    || item.dim4_format.to_lowercase().contains(&filter)
                    || item.dim5_sink.to_lowercase().contains(&filter)
                    || item.dim6_size.to_lowercase().contains(&filter)
            })
            .collect();

        // Сортировка
        let col = sort_column.get();
        let dir = sort_direction.get();

        items.sort_by(|a, b| {
            let cmp = match col {
                SortColumn::Article => a.article.cmp(&b.article),
                SortColumn::Description => a.base.description.cmp(&b.base.description),
                SortColumn::Dim1Category => a.dim1_category.cmp(&b.dim1_category),
                SortColumn::Dim2Line => a.dim2_line.cmp(&b.dim2_line),
                SortColumn::Dim3Model => a.dim3_model.cmp(&b.dim3_model),
                SortColumn::Dim4Format => a.dim4_format.cmp(&b.dim4_format),
                SortColumn::Dim5Sink => a.dim5_sink.cmp(&b.dim5_sink),
                SortColumn::Dim6Size => a.dim6_size.cmp(&b.dim6_size),
            };

            match dir {
                SortDirection::Asc => cmp,
                SortDirection::Desc => cmp.reverse(),
            }
        });

        items
    };

    let is_filter_active = move || !filter_text.get().is_empty();

    // Функция для переключения чекбокса
    let toggle_item = move |id: String| {
        set_selected_ids.update(|ids| {
            if ids.contains(&id) {
                ids.remove(&id);
            } else {
                ids.insert(id);
            }
        });
    };

    // Функция для переключения всех видимых
    let toggle_all_visible = move || {
        let visible_items = filtered_items();
        let visible_ids: std::collections::HashSet<String> = visible_items
            .iter()
            .map(|item| item.base.id.0.to_string())
            .collect();

        set_selected_ids.update(|ids| {
            // Проверяем, все ли видимые элементы выбраны
            let all_selected = visible_ids.iter().all(|id| ids.contains(id));

            if all_selected {
                // Снимаем выбор со всех видимых
                for id in visible_ids {
                    ids.remove(&id);
                }
            } else {
                // Выбираем все видимые
                for id in visible_ids {
                    ids.insert(id);
                }
            }
        });
    };

    // Проверка, все ли видимые выбраны
    let are_all_visible_selected = move || {
        let visible_items = filtered_items();
        if visible_items.is_empty() {
            return false;
        }
        let selected = selected_ids.get();
        visible_items
            .iter()
            .all(|item| selected.contains(&item.base.id.0.to_string()))
    };

    // Обработка импорта из Excel
    let handle_excel_import = Callback::new(move |excel_data: ExcelData| {
        set_show_excel_importer.set(false);

        // Отправляем весь ExcelData на сервер
        // Backend сам сделает маппинг полей и обработку
        let load_clone = load.clone();
        leptos::task::spawn_local(async move {
            match send_import_data(excel_data).await {
                Ok(result) => {
                    let msg = if result.not_found_articles.is_empty() {
                        format!("✓ Успешно обновлено записей: {}", result.updated_count)
                    } else {
                        format!(
                            "✓ Обновлено: {}\n❌ Не найдено артикулов: {}\n\nАртикулы: {}",
                            result.updated_count,
                            result.not_found_articles.len(),
                            result.not_found_articles.join(", ")
                        )
                    };
                    web_sys::window().and_then(|w| Some(w.alert_with_message(&msg).ok()));
                    load_clone();
                }
                Err(e) => {
                    let msg = format!("Ошибка импорта: {}", e);
                    web_sys::window().and_then(|w| Some(w.alert_with_message(&msg).ok()));
                }
            }
        });
    });

    // Обработка отмены импорта
    let handle_excel_cancel = Callback::new(move |_| {
        set_show_excel_importer.set(false);
    });

    // Обработка просмотра JSON
    let _handle_json_view = Callback::new(move |json: String| {
        // TODO: Добавить JSON viewer в роутинг
        // Пока просто показываем alert с началом JSON
        web_sys::window()
            .and_then(|w| Some(w.alert_with_message(&json[..json.len().min(500)]).ok()));
    });

    // Обработка экспорта в Excel
    let handle_excel_export = move || {
        let items = filtered_items();
        if items.is_empty() {
            web_sys::window()
                .and_then(|w| Some(w.alert_with_message("Нет данных для экспорта").ok()));
            return;
        }

        let filename = format!(
            "nomenclature_{}.csv",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        );

        if let Err(e) = export_to_excel(&items, &filename) {
            web_sys::window().and_then(|w| {
                Some(
                    w.alert_with_message(&format!("Ошибка экспорта: {}", e))
                        .ok(),
                )
            });
        }
    };

    view! {
        <div style="display: flex; flex-direction: column; height: calc(100vh - 120px); overflow: hidden;">
            // Toolbar
            <div style="display: flex; gap: 10px; padding: 10px; background: #f5f5f5; border-bottom: 1px solid #ddd; flex-shrink: 0; align-items: center; flex-wrap: wrap;">
                <div style="position: relative; display: inline-flex; align-items: center;">
                    <input
                        type="text"
                        placeholder="Поиск по артикулу, наименованию, измерениям..."
                        style=move || format!(
                            "width: 350px; padding: 6px 32px 6px 10px; border: 1px solid #ddd; border-radius: 4px; font-size: 15px; background: {};",
                            if is_filter_active() { "#fffbea" } else { "white" }
                        )
                        prop:value=move || filter_input.get()
                        on:input=move |ev| {
                            let val = event_target_value(&ev);
                            handle_input_change(val);
                        }
                    />
                    {move || if !filter_input.get().is_empty() {
                        view! {
                            <button
                                style="position: absolute; right: 6px; background: none; border: none; cursor: pointer; padding: 4px; display: inline-flex; align-items: center; color: #666; line-height: 1;"
                                on:click=move |_| {
                                    set_filter_input.set(String::new());
                                    set_filter_text.set(String::new());
                                }
                                title="Очистить"
                            >
                                {icon("x")}
                            </button>
                        }.into_any()
                    } else {
                        view! { <></> }.into_any()
                    }}
                </div>
                <label style="display: inline-flex; align-items: center; gap: 6px; cursor: pointer; user-select: none; font-size: 15px;">
                    <input
                        type="checkbox"
                        prop:checked=move || show_only_mp.get()
                        on:change=move |ev| {
                            set_show_only_mp.set(event_target_checked(&ev));
                        }
                        style="cursor: pointer;"
                    />
                    <span>{"Только из маркетплейсов"}</span>
                </label>
                <button class="button button--secondary" on:click=move |_| load()>
                    {icon("refresh")}
                    {"Обновить"}
                </button>
                <button
                    class="button button--primary"
                    on:click=move |_| set_show_excel_importer.set(true)
                >
                    {icon("upload")}
                    {"Импорт из Excel"}
                </button>
                <button
                    class="button button--primary"
                    on:click=move |_| handle_excel_export()
                >
                    {icon("download")}
                    {"Экспорт в Excel"}
                </button>

                // Счетчики
                <div style="margin-left: auto; display: flex; gap: 15px; font-size: 14px; color: #666;">
                    <span>
                        {"Всего: "}
                        <strong style="color: #333;">{move || filtered_items().len()}</strong>
                    </span>
                    <span>
                        {"Выбрано: "}
                        <strong style="color: #2196F3;">{move || selected_ids.get().len()}</strong>
                    </span>
                </div>
            </div>

            {move || error.get().map(|e| view! { <div class="error" style="background: #fee; color: #c33; padding: 8px; border-radius: 4px; margin: 8px; font-size: 15px; flex-shrink: 0;">{e}</div> })}

            {move || if is_loading.get() {
                view! { <div style="text-align: center; padding: 20px; color: #666;">{"⏳ Загрузка..."}</div> }.into_any()
            } else {
                let items = filtered_items();
                view! {
                    <div style="flex: 1; overflow-y: auto; overflow-x: hidden;">
                        <table style="width: 100%; border-collapse: collapse; font-size: 14px;">
                            <thead style="position: sticky; top: 0; background: #f9f9f9; z-index: 10;">
                                <tr style="border-bottom: 2px solid #ddd;">
                                    <th style="padding: 10px 8px; text-align: center; width: 40px;">
                                        <input
                                            type="checkbox"
                                            prop:checked=move || are_all_visible_selected()
                                            on:change=move |_| toggle_all_visible()
                                            style="cursor: pointer;"
                                            title="Выбрать/снять все видимые"
                                        />
                                    </th>
                                    <th
                                        style="padding: 10px 8px; text-align: left; cursor: pointer; user-select: none; min-width: 120px;"
                                        on:click=move |_| toggle_sort(SortColumn::Article)
                                    >
                                        {"Артикул "}
                                        {move || if sort_column.get() == SortColumn::Article {
                                            match sort_direction.get() {
                                                SortDirection::Asc => "↑",
                                                SortDirection::Desc => "↓",
                                            }
                                        } else {
                                            ""
                                        }}
                                    </th>
                                    <th
                                        style="padding: 10px 8px; text-align: left; cursor: pointer; user-select: none; min-width: 200px;"
                                        on:click=move |_| toggle_sort(SortColumn::Description)
                                    >
                                        {"Наименование "}
                                        {move || if sort_column.get() == SortColumn::Description {
                                            match sort_direction.get() {
                                                SortDirection::Asc => "↑",
                                                SortDirection::Desc => "↓",
                                            }
                                        } else {
                                            ""
                                        }}
                                    </th>
                                    <th
                                        style="padding: 10px 8px; text-align: left; cursor: pointer; user-select: none; min-width: 120px;"
                                        on:click=move |_| toggle_sort(SortColumn::Dim1Category)
                                    >
                                        {"Категория "}
                                        {move || if sort_column.get() == SortColumn::Dim1Category {
                                            match sort_direction.get() {
                                                SortDirection::Asc => "↑",
                                                SortDirection::Desc => "↓",
                                            }
                                        } else {
                                            ""
                                        }}
                                    </th>
                                    <th
                                        style="padding: 10px 8px; text-align: left; cursor: pointer; user-select: none; min-width: 120px;"
                                        on:click=move |_| toggle_sort(SortColumn::Dim2Line)
                                    >
                                        {"Линейка "}
                                        {move || if sort_column.get() == SortColumn::Dim2Line {
                                            match sort_direction.get() {
                                                SortDirection::Asc => "↑",
                                                SortDirection::Desc => "↓",
                                            }
                                        } else {
                                            ""
                                        }}
                                    </th>
                                    <th
                                        style="padding: 10px 8px; text-align: left; cursor: pointer; user-select: none; min-width: 150px;"
                                        on:click=move |_| toggle_sort(SortColumn::Dim3Model)
                                    >
                                        {"Модель "}
                                        {move || if sort_column.get() == SortColumn::Dim3Model {
                                            match sort_direction.get() {
                                                SortDirection::Asc => "↑",
                                                SortDirection::Desc => "↓",
                                            }
                                        } else {
                                            ""
                                        }}
                                    </th>
                                    <th
                                        style="padding: 10px 8px; text-align: left; cursor: pointer; user-select: none; min-width: 100px;"
                                        on:click=move |_| toggle_sort(SortColumn::Dim4Format)
                                    >
                                        {"Формат "}
                                        {move || if sort_column.get() == SortColumn::Dim4Format {
                                            match sort_direction.get() {
                                                SortDirection::Asc => "↑",
                                                SortDirection::Desc => "↓",
                                            }
                                        } else {
                                            ""
                                        }}
                                    </th>
                                    <th
                                        style="padding: 10px 8px; text-align: left; cursor: pointer; user-select: none; min-width: 120px;"
                                        on:click=move |_| toggle_sort(SortColumn::Dim5Sink)
                                    >
                                        {"Раковина "}
                                        {move || if sort_column.get() == SortColumn::Dim5Sink {
                                            match sort_direction.get() {
                                                SortDirection::Asc => "↑",
                                                SortDirection::Desc => "↓",
                                            }
                                        } else {
                                            ""
                                        }}
                                    </th>
                                    <th
                                        style="padding: 10px 8px; text-align: left; cursor: pointer; user-select: none; min-width: 100px;"
                                        on:click=move |_| toggle_sort(SortColumn::Dim6Size)
                                    >
                                        {"Размер "}
                                        {move || if sort_column.get() == SortColumn::Dim6Size {
                                            match sort_direction.get() {
                                                SortDirection::Asc => "↑",
                                                SortDirection::Desc => "↓",
                                            }
                                        } else {
                                            ""
                                        }}
                                    </th>
                                </tr>
                            </thead>
                            <tbody>
                                {
                                    if items.is_empty() {
                                        view! {
                                            <tr>
                                                <td colspan="10" style="text-align: center; padding: 20px; color: #888;">
                                                    {if all_items.get().is_empty() {
                                                        "Нет данных. Нажмите 'Обновить' или загрузите данные через импорт."
                                                    } else {
                                                        "По фильтру ничего не найдено"
                                                    }}
                                                </td>
                                            </tr>
                                        }.into_any()
                                    } else {
                                        items.into_iter().enumerate().map(|(idx, item)| {
                                            let bg_color = if idx % 2 == 0 { "#fff" } else { "#f9f9f9" };
                                            let item_id = item.base.id.0.to_string();
                                            let item_id_for_check = item_id.clone();
                                            let item_id_for_click = item_id.clone();
                                            view! {
                                                <tr style=format!("background: {}; border-bottom: 1px solid #eee; cursor: pointer;", bg_color)
                                                    class="hover:bg-gray-100"
                                                    on:click=move |e| {
                                                        // Проверяем что клик не по чекбоксу
                                                        if let Some(target) = e.target() {
                                                            if let Ok(el) = target.dyn_into::<web_sys::HtmlElement>() {
                                                                if el.tag_name() != "INPUT" {
                                                                    set_editing_id.set(Some(item_id_for_click.clone()));
                                                                    set_show_modal.set(true);
                                                                }
                                                            }
                                                        }
                                                    }
                                                    on:mouseenter=move |e| {
                                                        if let Some(target) = e.target() {
                                                            if let Ok(el) = target.dyn_into::<web_sys::HtmlElement>() {
                                                                let _ = el.style().set_property("background", "#f0f0f0");
                                                            }
                                                        }
                                                    }
                                                    on:mouseleave=move |e| {
                                                        if let Some(target) = e.target() {
                                                            if let Ok(el) = target.dyn_into::<web_sys::HtmlElement>() {
                                                                let _ = el.style().set_property("background", bg_color);
                                                            }
                                                        }
                                                    }
                                                >
                                                    <td style="padding: 8px; text-align: center;">
                                                        <input
                                                            type="checkbox"
                                                            prop:checked=move || selected_ids.get().contains(&item_id_for_check)
                                                            on:change=move |_| toggle_item(item_id.clone())
                                                            style="cursor: pointer;"
                                                        />
                                                    </td>
                                                    <td style="padding: 8px;" title=item.article.clone()>{item.article.clone()}</td>
                                                    <td style="padding: 8px;" title=item.base.description.clone()>{item.base.description.clone()}</td>
                                                    <td style="padding: 8px;" title=item.dim1_category.clone()>{item.dim1_category.clone()}</td>
                                                    <td style="padding: 8px;" title=item.dim2_line.clone()>{item.dim2_line.clone()}</td>
                                                    <td style="padding: 8px;" title=item.dim3_model.clone()>{item.dim3_model.clone()}</td>
                                                    <td style="padding: 8px;" title=item.dim4_format.clone()>{item.dim4_format.clone()}</td>
                                                    <td style="padding: 8px;" title=item.dim5_sink.clone()>{item.dim5_sink.clone()}</td>
                                                    <td style="padding: 8px;" title=item.dim6_size.clone()>{item.dim6_size.clone()}</td>
                                                </tr>
                                            }
                                        }).collect_view().into_any()
                                    }
                                }
                            </tbody>
                        </table>
                    </div>
                }.into_any()
            }}

            // Excel Importer Modal
            {move || if show_excel_importer.get() {
                view! {
                    <ExcelImporter
                        columns=excel_columns.clone()
                        on_import=handle_excel_import
                        on_cancel=handle_excel_cancel
                    />
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }}

            // Details Modal
            {move || if show_modal.get() {
                view! {
                    <div class="modal-overlay">
                        <div class="modal-content-wide">
                            <NomenclatureDetails
                                id=editing_id.get()
                                on_saved=move || { set_show_modal.set(false); set_editing_id.set(None); load(); }
                                on_cancel=move || { set_show_modal.set(false); set_editing_id.set(None); }
                            />
                        </div>
                    </div>
                }.into_any()
            } else { view! { <></> }.into_any() }}
        </div>
    }
}
