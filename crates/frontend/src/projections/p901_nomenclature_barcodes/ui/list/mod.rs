use crate::domain::a004_nomenclature::ui::details::NomenclatureDetails;
use crate::shared::date_utils::format_datetime;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;

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

        let mut csv =
            String::from("Штрихкод;ID Номенклатуры;Артикул;Источник;Создано;Обновлено;Активен\n");

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

                if let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&array, &options)
                {
                    let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();

                    if let Ok(Some(anchor)) = document
                        .create_element("a")
                        .map(|e| e.dyn_into::<web_sys::HtmlAnchorElement>().ok())
                    {
                        anchor.set_href(&url);
                        anchor.set_download(&format!(
                            "barcodes_{}.csv",
                            chrono::Utc::now().format("%Y%m%d_%H%M%S")
                        ));
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
        <div class="document-container">
            <div class="document-content">
                <div class="document-inner">
                    // Заголовок страницы
                    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--spacing-lg); padding-bottom: var(--spacing-md); border-bottom: 1px solid var(--color-border);">
                        <h2 style="margin: 0; font-size: var(--font-size-xl); color: var(--color-text-primary);">"Штрихкоды номенклатуры"</h2>
                        <div class="button-group">
                            <button
                                class="button button--primary"
                                on:click=move |_| load_barcodes()
                            >
                                "Обновить"
                            </button>
                            <button
                                class="button button--secondary"
                                on:click=export_to_csv
                            >
                                "Экспорт в Excel"
                            </button>
                        </div>
                    </div>

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

            // Фильтры
            <div class="form-section" style="background: var(--color-background-secondary); padding: var(--spacing-md); border-radius: var(--radius-md); margin-bottom: var(--spacing-lg);">
                <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: var(--spacing-md);">
                    // Поиск по штрихкоду
                    <div class="form__group">
                        <label class="form__label">
                            "Поиск по штрихкоду:"
                        </label>
                        <input
                            class="form__input"
                            type="text"
                            placeholder="Введите штрихкод..."
                            prop:value=move || search_barcode.get()
                            on:input=move |ev| {
                                set_search_barcode.set(event_target_value(&ev));
                            }
                        />
                    </div>

                    // Поиск по артикулу
                    <div class="form__group">
                        <label class="form__label">
                            "Поиск по артикулу:"
                        </label>
                        <input
                            class="form__input"
                            type="text"
                            placeholder="Введите артикул..."
                            prop:value=move || search_article.get()
                            on:input=move |ev| {
                                set_search_article.set(event_target_value(&ev));
                            }
                        />
                    </div>

                    // Фильтр по источнику
                    <div class="form__group">
                        <label class="form__label">
                            "Источник:"
                        </label>
                        <select
                            class="form__select"
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
                        <label class="form__checkbox-wrapper">
                            <input
                                class="form__checkbox"
                                type="checkbox"
                                prop:checked=move || include_inactive.get()
                                on:change=move |ev| {
                                    set_include_inactive.set(event_target_checked(&ev));
                                }
                            />
                            <span class="form__checkbox-label">"Показать неактивные"</span>
                        </label>
                    </div>
                </div>

                <div style="margin-top: var(--spacing-md); display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: var(--spacing-md);">
                    <button
                        class="button button--primary"
                        on:click=move |_| load_barcodes()
                    >
                        "Применить фильтры"
                    </button>

                    // Пагинация
                    <div style="display: flex; align-items: center; gap: var(--spacing-md); flex-wrap: wrap;">
                        <div style="font-size: var(--font-size-sm); color: var(--color-text-secondary);">
                            {move || {
                                let current_offset = offset.get();
                                let current_limit = limit.get();
                                let total = total_count.get();
                                let start = if total > 0 { current_offset + 1 } else { 0 };
                                let end = std::cmp::min(current_offset + current_limit, total);
                                format!("Показано: {}-{} из {}", start, end, total)
                            }}
                        </div>
                        <div class="button-group">
                            <button
                                class="button button--secondary"
                                on:click=go_to_prev_page
                                prop:disabled=move || offset.get() == 0
                            >
                                "← Назад"
                            </button>
                            <button
                                class="button button--secondary"
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
                        <div class="warning-box" style="background: var(--color-error-50); border-color: var(--color-error-100); margin-bottom: var(--spacing-md);">
                            <span class="warning-box__icon" style="color: var(--color-error);">"⚠"</span>
                            <span class="warning-box__text" style="color: var(--color-error);">{err}</span>
                        </div>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}

            // Индикатор загрузки
            {move || {
                if loading.get() {
                    view! {
                        <div class="info-box" style="text-align: center; margin-bottom: var(--spacing-md);">
                            <span class="info-box__text">"Загрузка..."</span>
                        </div>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}

            // Таблица
            <div class="table">
                <table class="table__data table--striped">
                    <thead class="table__head">
                        <tr>
                            <th
                                class="table__header-cell table__header-cell--sortable"
                                on:click=move |_| handle_column_click(SortColumn::Barcode)
                            >
                                "Штрихкод" {sort_indicator(SortColumn::Barcode)}
                            </th>
                            <th
                                class="table__header-cell table__header-cell--sortable"
                                on:click=move |_| handle_column_click(SortColumn::NomenclatureName)
                            >
                                "Наименование" {sort_indicator(SortColumn::NomenclatureName)}
                            </th>
                            <th
                                class="table__header-cell table__header-cell--sortable"
                                on:click=move |_| handle_column_click(SortColumn::Article)
                            >
                                "Артикул" {sort_indicator(SortColumn::Article)}
                            </th>
                            <th
                                class="table__header-cell table__header-cell--sortable"
                                on:click=move |_| handle_column_click(SortColumn::Source)
                            >
                                "Источник" {sort_indicator(SortColumn::Source)}
                            </th>
                            <th
                                class="table__header-cell table__header-cell--sortable"
                                on:click=move |_| handle_column_click(SortColumn::UpdatedAt)
                            >
                                "Обновлено" {sort_indicator(SortColumn::UpdatedAt)}
                            </th>
                            <th class="table__header-cell table__header-cell--center">"Активен"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            sorted_barcodes().into_iter().map(|item| {
                                let has_no_nomenclature = item.nomenclature_ref.is_none();
                                let nomenclature_ref_for_link = item.nomenclature_ref.clone();

                                view! {
                                    <tr
                                        class="table__row"
                                        class:table__row--warning=has_no_nomenclature
                                    >
                                        <td class="table__cell" style="font-family: monospace;">
                                            {item.barcode.clone()}
                                            {if has_no_nomenclature {
                                                view! {
                                                    <span
                                                        style="margin-left: var(--spacing-xs); padding: 2px 5px; background: var(--color-warning); color: white; font-size: var(--font-size-xs); border-radius: var(--radius-sm);"
                                                        title="Не привязан к номенклатуре"
                                                    >
                                                        "!"
                                                    </span>
                                                }.into_any()
                                            } else {
                                                view! { <></> }.into_any()
                                            }}
                                        </td>
                                        <td class="table__cell">
                                            {if let Some(nom_ref) = nomenclature_ref_for_link {
                                                if let Some(name) = item.nomenclature_name.clone() {
                                                    view! {
                                                        <a
                                                            href="#"
                                                            style="color: var(--color-primary); text-decoration: none; cursor: pointer;"
                                                            on:click=move |ev| {
                                                                ev.prevent_default();
                                                                set_selected_nomenclature_id.set(Some(nom_ref.clone()));
                                                            }
                                                        >
                                                            {name}
                                                        </a>
                                                    }.into_any()
                                                } else {
                                                    view! { <span style="color: var(--color-text-tertiary);">"-"</span> }.into_any()
                                                }
                                            } else {
                                                view! {
                                                    <span style="color: var(--color-warning); font-weight: 500;">
                                                        "Не привязан"
                                                    </span>
                                                }.into_any()
                                            }}
                                        </td>
                                        <td class="table__cell">{item.article.clone().unwrap_or_else(|| "-".to_string())}</td>
                                        <td class="table__cell">
                                            <span style={format!("padding: 2px 8px; border-radius: var(--radius-sm); background: {}; color: white; font-size: var(--font-size-xs);",
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
                                        <td class="table__cell" style="font-size: var(--font-size-sm);">{format_datetime(&item.updated_at)}</td>
                                        <td class="table__cell table__cell--center">
                                            {if item.is_active {
                                                view! { <span style="color: var(--color-success); font-weight: bold;">"✓"</span> }.into_any()
                                            } else {
                                                view! { <span style="color: var(--color-error); font-weight: bold;">"✗"</span> }.into_any()
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
            </div>
        </div>
    }
}
