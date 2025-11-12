use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use crate::domain::a004_nomenclature::ui::details::NomenclatureDetails;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NomenclatureBarcodeDto {
    pub barcode: String,
    pub source: String,
    pub nomenclature_ref: Option<String>,
    pub nomenclature_name: Option<String>,
    pub article: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarcodeListResponse {
    pub barcodes: Vec<NomenclatureBarcodeDto>,
    pub total_count: i32,
    pub limit: i32,
    pub offset: i32,
}

#[derive(Debug, Clone, PartialEq)]
enum SortColumn {
    Barcode,
    NomenclatureRef,
    NomenclatureName,
    Article,
    Source,
    UpdatedAt,
}

#[derive(Debug, Clone, PartialEq)]
enum SortDirection {
    Asc,
    Desc,
}

/// Форматирует ISO8601 дату в русский формат DD.MM.YYYY HH:MM:SS
fn format_datetime(iso_string: &str) -> String {
    // Пытаемся распарсить ISO8601 дату
    if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(iso_string) {
        parsed.format("%d.%m.%Y %H:%M:%S").to_string()
    } else {
        // Если не удалось, возвращаем исходную строку
        iso_string.to_string()
    }
}

#[component]
pub fn BarcodesList() -> impl IntoView {
    let (barcodes, set_barcodes) = signal(Vec::<NomenclatureBarcodeDto>::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(None::<String>);

    // Состояние модального окна для номенклатуры
    let (selected_nomenclature_id, set_selected_nomenclature_id) = signal::<Option<String>>(None);

    // Состояние сортировки
    let (sort_column, set_sort_column) = signal::<Option<SortColumn>>(Some(SortColumn::Barcode));
    let (sort_direction, set_sort_direction) = signal(SortDirection::Asc);

    // Пагинация
    let (limit, _set_limit) = signal(100);
    let (offset, set_offset) = signal(0);
    let (total_count, set_total_count) = signal(0);

    // Фильтры
    let (search_barcode, set_search_barcode) = signal(String::new());
    let (search_article, set_search_article) = signal(String::new());
    let (filter_source, set_filter_source) = signal(String::new());
    let (include_inactive, set_include_inactive) = signal(false);

    // Функция загрузки данных
    let load_barcodes = move || {
        set_loading.set(true);
        set_error.set(None);

        spawn_local(async move {
            // Формируем query параметры
            let mut query_params = vec![
                format!("limit={}", limit.get()),
                format!("offset={}", offset.get()),
                format!("include_inactive={}", include_inactive.get()),
            ];

            let article_filter = search_article.get();
            if !article_filter.is_empty() {
                query_params.push(format!("article={}", article_filter));
            }

            let source_filter = filter_source.get();
            if !source_filter.is_empty() && source_filter != "all" {
                query_params.push(format!("source={}", source_filter));
            }

            let query = query_params.join("&");
            let url = format!("/api/p901/barcodes?{}", query);

            match gloo_net::http::Request::get(&url).send().await {
                Ok(response) => {
                    if response.ok() {
                        match response.json::<BarcodeListResponse>().await {
                            Ok(data) => {
                                set_total_count.set(data.total_count);
                                set_barcodes.set(data.barcodes);
                            }
                            Err(e) => {
                                set_error.set(Some(format!("Ошибка парсинга: {}", e)));
                            }
                        }
                    } else {
                        set_error.set(Some(format!("HTTP ошибка: {}", response.status())));
                    }
                }
                Err(e) => {
                    set_error.set(Some(format!("Ошибка запроса: {}", e)));
                }
            }

            set_loading.set(false);
        });
    };

    // Загрузить данные при монтировании
    Effect::new(move || {
        load_barcodes();
    });

    // Функция для обработки клика по заголовку колонки
    let handle_column_click = move |column: SortColumn| {
        if sort_column.get() == Some(column.clone()) {
            // Переключаем направление
            set_sort_direction.set(match sort_direction.get() {
                SortDirection::Asc => SortDirection::Desc,
                SortDirection::Desc => SortDirection::Asc,
            });
        } else {
            // Новая колонка - сортируем по возрастанию
            set_sort_column.set(Some(column));
            set_sort_direction.set(SortDirection::Asc);
        }
    };

    // Отсортированные данные
    let sorted_barcodes = move || {
        let mut data = barcodes.get();

        // Локальный поиск по штрихкоду
        let search = search_barcode.get();
        if !search.is_empty() {
            data.retain(|b| b.barcode.to_lowercase().contains(&search.to_lowercase()));
        }

        if let Some(col) = sort_column.get() {
            let direction = sort_direction.get();
            data.sort_by(|a, b| {
                let cmp = match col {
                    SortColumn::Barcode => a.barcode.cmp(&b.barcode),
                    SortColumn::NomenclatureRef => a.nomenclature_ref.cmp(&b.nomenclature_ref),
                    SortColumn::NomenclatureName => {
                        let a_name = a.nomenclature_name.as_deref().unwrap_or("");
                        let b_name = b.nomenclature_name.as_deref().unwrap_or("");
                        a_name.cmp(b_name)
                    }
                    SortColumn::Article => {
                        let a_art = a.article.as_deref().unwrap_or("");
                        let b_art = b.article.as_deref().unwrap_or("");
                        a_art.cmp(b_art)
                    }
                    SortColumn::Source => a.source.cmp(&b.source),
                    SortColumn::UpdatedAt => a.updated_at.cmp(&b.updated_at),
                };
                match direction {
                    SortDirection::Asc => cmp,
                    SortDirection::Desc => cmp.reverse(),
                }
            });
        }
        data
    };

    // Индикатор сортировки
    let sort_indicator = move |column: SortColumn| {
        if sort_column.get() == Some(column) {
            match sort_direction.get() {
                SortDirection::Asc => " ▲",
                SortDirection::Desc => " ▼",
            }
        } else {
            ""
        }
    };

    // Экспорт в CSV (Excel-compatible)
    let export_to_csv = move |_| {
        let data = sorted_barcodes();

        let mut csv = String::from("Штрихкод;ID Номенклатуры;Артикул;Источник;Создано;Обновлено;Активен\n");

        for item in data {
            let line = format!(
                "{};{};{};{};{};{};{}\n",
                item.barcode,
                item.nomenclature_ref.unwrap_or_default(),
                item.article.unwrap_or_default(),
                item.source,
                item.created_at,
                item.updated_at,
                if item.is_active { "Да" } else { "Нет" }
            );
            csv.push_str(&line);
        }

        // Создать blob и скачать
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                // Создаем Blob с BOM для корректного отображения кириллицы в Excel
                let bom = "\u{FEFF}";
                let csv_with_bom = format!("{}{}", bom, csv);

                let array = js_sys::Array::new();
                array.push(&wasm_bindgen::JsValue::from_str(&csv_with_bom));

                let options = web_sys::BlobPropertyBag::new();
                options.set_type("text/csv;charset=utf-8");

                if let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&array, &options) {
                    let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();

                    if let Ok(Some(anchor)) = document.create_element("a").map(|e| e.dyn_into::<web_sys::HtmlAnchorElement>().ok()) {
                        anchor.set_href(&url);
                        anchor.set_download(&format!("barcodes_{}.csv", chrono::Utc::now().format("%Y%m%d_%H%M%S")));
                        let _ = anchor.click();
                        web_sys::Url::revoke_object_url(&url).ok();
                    }
                }
            }
        }
    };

    // Пагинация
    let go_to_prev_page = move |_| {
        let current_offset: i32 = offset.get();
        let current_limit: i32 = limit.get();
        let new_offset = current_offset.saturating_sub(current_limit);
        set_offset.set(new_offset);
        load_barcodes();
    };

    let go_to_next_page = move |_| {
        let new_offset = offset.get() + limit.get();
        if new_offset < total_count.get() {
            set_offset.set(new_offset);
            load_barcodes();
        }
    };

    view! {
        <div style="padding: 20px;">
            <h2>"p901: Штрихкоды номенклатуры"</h2>

            // Модальное окно для деталей номенклатуры
            {move || {
                if let Some(nomenclature_id) = selected_nomenclature_id.get() {
                    view! {
                        <div class="modal-overlay" style="position: fixed; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.5); display: flex; align-items: flex-start; justify-content: center; z-index: 1000; padding-top: 40px;">
                            <div class="modal-content" style="background: white; border-radius: 8px; box-shadow: 0 4px 6px rgba(0,0,0,0.1); max-width: 800px; width: 90%; max-height: calc(100vh - 80px); overflow-y: auto; margin: 0;">
                                <NomenclatureDetails
                                    id=Some(nomenclature_id.clone())
                                    on_saved=move || {
                                        set_selected_nomenclature_id.set(None);
                                        load_barcodes();
                                    }
                                    on_cancel=move || {
                                        set_selected_nomenclature_id.set(None);
                                    }
                                />
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}

            // Фильтры и управление
            <div style="margin: 20px 0; padding: 15px; background: #f5f5f5; border-radius: 8px;">
                <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 10px;">
                    // Поиск по штрихкоду
                    <div>
                        <label style="display: block; font-size: 12px; margin-bottom: 4px; font-weight: bold;">
                            "Поиск по штрихкоду:"
                        </label>
                        <input
                            type="text"
                            placeholder="Введите штрихкод..."
                            style="width: 100%; padding: 6px; border: 1px solid #ddd; border-radius: 4px;"
                            prop:value=move || search_barcode.get()
                            on:input=move |ev| {
                                set_search_barcode.set(event_target_value(&ev));
                            }
                        />
                    </div>

                    // Поиск по артикулу
                    <div>
                        <label style="display: block; font-size: 12px; margin-bottom: 4px; font-weight: bold;">
                            "Поиск по артикулу:"
                        </label>
                        <input
                            type="text"
                            placeholder="Введите артикул..."
                            style="width: 100%; padding: 6px; border: 1px solid #ddd; border-radius: 4px;"
                            prop:value=move || search_article.get()
                            on:input=move |ev| {
                                set_search_article.set(event_target_value(&ev));
                            }
                        />
                    </div>

                    // Фильтр по источнику
                    <div>
                        <label style="display: block; font-size: 12px; margin-bottom: 4px; font-weight: bold;">
                            "Источник:"
                        </label>
                        <select
                            style="width: 100%; padding: 6px; border: 1px solid #ddd; border-radius: 4px;"
                            on:change=move |ev| {
                                set_filter_source.set(event_target_value(&ev));
                            }
                        >
                            <option value="all">"Все"</option>
                            <option value="1C">"1C"</option>
                            <option value="OZON">"OZON"</option>
                            <option value="WB">"WB"</option>
                            <option value="YM">"YM"</option>
                        </select>
                    </div>

                    // Чекбокс неактивных
                    <div style="display: flex; align-items: flex-end;">
                        <label style="display: flex; align-items: center; gap: 5px;">
                            <input
                                type="checkbox"
                                prop:checked=move || include_inactive.get()
                                on:change=move |ev| {
                                    set_include_inactive.set(event_target_checked(&ev));
                                }
                            />
                            <span>"Показать неактивные"</span>
                        </label>
                    </div>
                </div>

                <div style="margin-top: 10px; display: flex; justify-content: space-between; align-items: center;">
                    <div style="display: flex; gap: 10px;">
                        <button
                            style="padding: 8px 16px; background: #007bff; color: white; border: none; border-radius: 4px; cursor: pointer;"
                            on:click=move |_| load_barcodes()
                        >
                            "Применить фильтры"
                        </button>
                        <button
                            style="padding: 8px 16px; background: #28a745; color: white; border: none; border-radius: 4px; cursor: pointer;"
                            on:click=export_to_csv
                        >
                            "Экспорт в Excel (CSV)"
                        </button>
                    </div>

                    // Пагинация
                    <div style="display: flex; align-items: center; gap: 15px;">
                        <div style="font-size: 14px; color: #666;">
                            {move || {
                                let current_offset = offset.get();
                                let current_limit = limit.get();
                                let total = total_count.get();
                                let start = if total > 0 { current_offset + 1 } else { 0 };
                                let end = std::cmp::min(current_offset + current_limit, total);
                                format!("Показано: {}-{} из {}", start, end, total)
                            }}
                        </div>
                        <div style="display: flex; gap: 5px;">
                            <button
                                style="padding: 6px 12px; background: #007bff; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 13px;"
                                on:click=go_to_prev_page
                                prop:disabled=move || offset.get() == 0
                            >
                                "← Назад"
                            </button>
                            <button
                                style="padding: 6px 12px; background: #007bff; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 13px;"
                                on:click=go_to_next_page
                                prop:disabled=move || {
                                    let current_offset = offset.get();
                                    let current_limit = limit.get();
                                    let total = total_count.get();
                                    current_offset + current_limit >= total
                                }
                            >
                                "Вперёд →"
                            </button>
                        </div>
                    </div>
                </div>
            </div>

            // Ошибки
            {move || {
                if let Some(err) = error.get() {
                    view! {
                        <div style="padding: 10px; background: #fee; border: 1px solid #fcc; border-radius: 4px; color: #c00; margin: 10px 0;">
                            {err}
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}

            // Индикатор загрузки
            {move || {
                if loading.get() {
                    view! {
                        <div style="padding: 20px; text-align: center;">
                            "Загрузка..."
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}

            // Таблица
            <div style="overflow-x: auto;">
                <table style="width: 100%; border-collapse: collapse; background: white; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                    <thead>
                        <tr style="background: #f8f9fa; border-bottom: 2px solid #dee2e6;">
                            <th
                                style="padding: 12px; text-align: left; cursor: pointer; user-select: none; font-weight: 600; color: #495057;"
                                on:click=move |_| handle_column_click(SortColumn::Barcode)
                            >
                                "Штрихкод" {sort_indicator(SortColumn::Barcode)}
                            </th>
                            //<th
                            //    style="padding: 12px; text-align: left; cursor: pointer; user-select: none; font-weight: 600; color: #495057;"
                            //    on:click=move |_| handle_column_click(SortColumn::NomenclatureRef)
                            //>
                            //    "ID Номенклатуры" {sort_indicator(SortColumn::NomenclatureRef)}
                            //</th>
                            <th
                                style="padding: 12px; text-align: left; cursor: pointer; user-select: none; font-weight: 600; color: #495057;"
                                on:click=move |_| handle_column_click(SortColumn::NomenclatureName)
                            >
                                "Наименование" {sort_indicator(SortColumn::NomenclatureName)}
                            </th>
                            <th
                                style="padding: 12px; text-align: left; cursor: pointer; user-select: none; font-weight: 600; color: #495057;"
                                on:click=move |_| handle_column_click(SortColumn::Article)
                            >
                                "Артикул" {sort_indicator(SortColumn::Article)}
                            </th>
                            <th
                                style="padding: 12px; text-align: left; cursor: pointer; user-select: none; font-weight: 600; color: #495057;"
                                on:click=move |_| handle_column_click(SortColumn::Source)
                            >
                                "Источник" {sort_indicator(SortColumn::Source)}
                            </th>
                            <th
                                style="padding: 12px; text-align: left; cursor: pointer; user-select: none; font-weight: 600; color: #495057;"
                                on:click=move |_| handle_column_click(SortColumn::UpdatedAt)
                            >
                                "Обновлено" {sort_indicator(SortColumn::UpdatedAt)}
                            </th>
                            <th style="padding: 12px; text-align: center; font-weight: 600; color: #495057;">"Активен"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            sorted_barcodes().into_iter().enumerate().map(|(idx, item)| {
                                // Базовый цвет фона
                                let base_bg_color = if idx % 2 == 0 { "#fff" } else { "#f9f9f9" };

                                // Если nomenclature_ref отсутствует - подсвечиваем строку желтым
                                let bg_color = if item.nomenclature_ref.is_none() {
                                    "#fff3cd"  // Светло-желтый фон для строк без номенклатуры
                                } else {
                                    base_bg_color
                                };

                                let has_no_nomenclature = item.nomenclature_ref.is_none();
                                let nomenclature_ref_for_link = item.nomenclature_ref.clone();

                                view! {
                                    <tr style={format!("background: {}; border-bottom: 1px solid #eee;", bg_color)}>
                                        <td style="padding: 10px; font-family: monospace;">
                                            {item.barcode.clone()}
                                            {if has_no_nomenclature {
                                                view! {
                                                    <span
                                                        style="margin-left: 6px; padding: 2px 5px; background: #f0ad4e; color: white; font-size: 10px; border-radius: 3px;"
                                                        title="Не привязан к номенклатуре"
                                                    >
                                                        "!"
                                                    </span>
                                                }.into_any()
                                            } else {
                                                view! { <span></span> }.into_any()
                                            }}
                                        </td>
                                        //<td style="padding: 10px; font-family: monospace; font-size: 11px;">{item.nomenclature_ref.clone()}</td>
                                        <td style="padding: 10px;">
                                            {if let Some(nom_ref) = nomenclature_ref_for_link {
                                                if let Some(name) = item.nomenclature_name.clone() {
                                                    view! {
                                                        <a
                                                            href="#"
                                                            style="color: #007bff; text-decoration: none; cursor: pointer;"
                                                            on:click=move |ev| {
                                                                ev.prevent_default();
                                                                set_selected_nomenclature_id.set(Some(nom_ref.clone()));
                                                            }
                                                        >
                                                            {name}
                                                        </a>
                                                    }.into_any()
                                                } else {
                                                    view! { <span style="color: #999;">"-"</span> }.into_any()
                                                }
                                            } else {
                                                view! {
                                                    <span style="color: #f0ad4e; font-weight: 500;">
                                                        "Не привязан"
                                                    </span>
                                                }.into_any()
                                            }}
                                        </td>
                                        <td style="padding: 10px;">{item.article.clone().unwrap_or_else(|| "-".to_string())}</td>
                                        <td style="padding: 10px;">
                                            <span style={format!("padding: 2px 8px; border-radius: 3px; background: {}; color: white; font-size: 11px;",
                                                match item.source.as_str() {
                                                    "1C" => "#6c757d",
                                                    "OZON" => "#0088cc",
                                                    "WB" => "#8b00ff",
                                                    "YM" => "#fc0",
                                                    _ => "#333",
                                                }
                                            )}>
                                                {item.source.clone()}
                                            </span>
                                        </td>
                                        <td style="padding: 10px; font-size: 12px;">{format_datetime(&item.updated_at)}</td>
                                        <td style="padding: 10px; text-align: center;">
                                            {if item.is_active {
                                                view! { <span style="color: #28a745; font-weight: bold;">"✓"</span> }.into_any()
                                            } else {
                                                view! { <span style="color: #dc3545; font-weight: bold;">"✗"</span> }.into_any()
                                            }}
                                        </td>
                                    </tr>
                                }
                            }).collect_view()
                        }}
                    </tbody>
                </table>
            </div>
        </div>
    }
}
