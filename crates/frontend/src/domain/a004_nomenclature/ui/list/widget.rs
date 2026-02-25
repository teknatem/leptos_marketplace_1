use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::excel_importer::{ColumnDef, DataType, ExcelImporter};
use crate::shared::export::{export_to_excel, ExcelExportable};
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator};
use crate::shared::modal_stack::ModalStackService;
use contracts::domain::a004_nomenclature::aggregate::Nomenclature;
use contracts::domain::common::AggregateId;
use leptos::prelude::*;
use serde::Deserialize;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use thaw::*;
use wasm_bindgen::JsCast;

use super::state::create_state;

#[derive(Debug, Clone, Deserialize)]
struct PaginatedResponse {
    pub items: Vec<Nomenclature>,
    pub total: u64,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

async fn fetch_nomenclature_paginated(
    limit: usize,
    offset: usize,
    sort_by: &str,
    sort_desc: bool,
    q: &str,
    only_mp: bool,
) -> Result<PaginatedResponse, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let mut url = format!(
        "{}/api/a004/nomenclature?limit={}&offset={}&sort_by={}&sort_desc={}&only_mp={}",
        api_base(),
        limit,
        offset,
        sort_by,
        sort_desc,
        only_mp
    );

    let q_trimmed = q.trim();
    if q_trimmed.len() >= 3 {
        // NOTE: simplest encoding to avoid pulling extra deps
        let encoded = js_sys::encode_uri_component(q_trimmed)
            .as_string()
            .unwrap_or_default();
        url.push_str(&format!("&q={}", encoded));
    }

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
    serde_json::from_str(&text).map_err(|e| format!("{e}"))
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

#[component]
pub fn NomenclatureList() -> impl IntoView {
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");
    let modal_stack =
        use_context::<ModalStackService>().expect("ModalStackService not found in context");
    let state = create_state();

    let (items, set_items) = signal(Vec::<Nomenclature>::new());
    let (is_loading, set_is_loading) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);

    // Фильтры
    let q = RwSignal::new(state.get_untracked().q.clone());
    let only_mp = RwSignal::new(state.get_untracked().only_mp);

    // Сортировка
    let sort_field = RwSignal::new(state.get_untracked().sort_field.clone());
    let sort_ascending = RwSignal::new(state.get_untracked().sort_ascending);

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

        let s = state.get_untracked();
        let limit = s.page_size;
        let offset = s.page * s.page_size;
        let sort_desc = !s.sort_ascending;
        let q_val = s.q.clone();
        let only_mp_val = s.only_mp;
        let sort_by = s.sort_field.clone();

        leptos::task::spawn_local(async move {
            match fetch_nomenclature_paginated(
                limit,
                offset,
                &sort_by,
                sort_desc,
                &q_val,
                only_mp_val,
            )
            .await
            {
                Ok(data) => {
                    set_items.set(data.items);
                    state.update(|st| {
                        st.total_count = data.total as usize;
                        st.total_pages = data.total_pages;
                        st.page = data.page;
                        st.page_size = data.page_size;
                        st.is_loaded = true;
                    });
                    set_is_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_is_loading.set(false);
                }
            }
        });
    };

    // Initial load - only once
    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            load();
        }
    });

    // sync q (with debounce) -> state.q and reload
    let debounce_timeout = StoredValue::new(None::<i32>);
    let q_first_run = StoredValue::new(true);
    Effect::new(move |_| {
        let q_now = q.get();

        if q_first_run.get_value() {
            q_first_run.set_value(false);
            return;
        }

        // Cancel previous timer
        if let Some(timeout_id) = debounce_timeout.get_value() {
            web_sys::window().and_then(|w| Some(w.clear_timeout_with_handle(timeout_id)));
        }

        // Only apply filter for len>=3, or clear if empty
        if !(q_now.trim().is_empty() || q_now.trim().len() >= 3) {
            return;
        }

        let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
            state.update(|s| {
                s.q = q_now.clone();
                s.page = 0;
            });
            load();
        }) as Box<dyn Fn()>);

        let window = web_sys::window().expect("no window");
        let timeout_id = window
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref::<js_sys::Function>(),
                300,
            )
            .expect("setTimeout failed");

        closure.forget();
        debounce_timeout.set_value(Some(timeout_id));
    });

    // sync only_mp -> state.only_mp and reload
    let only_mp_first_run = StoredValue::new(true);
    Effect::new(move |_| {
        let v = only_mp.get();
        if only_mp_first_run.get_value() {
            only_mp_first_run.set_value(false);
            return;
        }
        state.update(|s| {
            s.only_mp = v;
            s.page = 0;
        });
        load();
    });

    let go_to_page = move |page: usize| {
        state.update(|s| s.page = page);
        load();
    };

    let change_page_size = move |size: usize| {
        state.update(|s| {
            s.page_size = size;
            s.page = 0;
        });
        load();
    };

    // Selection helpers
    let toggle_select = move |id: String, checked: bool| {
        state.update(|s| {
            if checked {
                if !s.selected_ids.contains(&id) {
                    s.selected_ids.push(id);
                }
            } else {
                s.selected_ids.retain(|x| x != &id);
            }
        });
    };

    let toggle_select_all_on_page = move |checked: bool| {
        let page_ids: Vec<String> = items.get().iter().map(|i| i.base.id.as_string()).collect();

        state.update(|s| {
            if checked {
                for id in page_ids {
                    if !s.selected_ids.contains(&id) {
                        s.selected_ids.push(id);
                    }
                }
            } else {
                s.selected_ids.retain(|id| !page_ids.contains(id));
            }
        });
    };

    let all_on_page_selected = Signal::derive(move || {
        let page_ids: Vec<String> = items.get().iter().map(|i| i.base.id.as_string()).collect();
        if page_ids.is_empty() {
            return false;
        }
        let selected = state.get().selected_ids;
        page_ids.iter().all(|id| selected.contains(id))
    });

    // Open Excel Importer via centralized modal stack
    let open_excel_importer = {
        let load_on_success = load.clone();
        Callback::new(move |_| {
            let columns = excel_columns.clone();
            let close_lock = Arc::new(AtomicBool::new(false));
            let close_guard = {
                let close_lock = close_lock.clone();
                Arc::new(move || !close_lock.load(Ordering::Relaxed))
            };

            modal_stack.push_with_frame_guard(
                Some("max-width: min(1400px, 95vw); width: min(1400px, 95vw);".to_string()),
                Some("excel-importer-modal".to_string()),
                Some(close_guard),
                move |handle| {
                    view! {
                        <ExcelImporter
                            columns=columns.clone()
                            import_endpoint="/api/nomenclature/import-excel".to_string()
                            on_success=Callback::new(move |_| load_on_success())
                            close_lock=close_lock.clone()
                            on_cancel=Callback::new({
                                let handle = handle.clone();
                                move |_| handle.close()
                            })
                        />
                    }
                    .into_any()
                },
            );
        })
    };

    // Open details in tab
    let open_details_tab = {
        let tabs_store = tabs_store;
        move |id: String, article: String, description: String| {
            use crate::layout::tabs::{detail_tab_label, pick_identifier};
            use contracts::domain::a004_nomenclature::ENTITY_METADATA as A004;
            let identifier = pick_identifier(None, Some(&article), Some(&description), &id);
            let title = detail_tab_label(A004.ui.element_name, identifier);
            tabs_store.open_tab(&format!("a004_nomenclature_detail_{}", id), &title);
        }
    };

    // Обработка просмотра JSON
    let _handle_json_view = Callback::new(move |json: String| {
        // TODO: Добавить JSON viewer в роутинг
        // Пока просто показываем alert с началом JSON
        web_sys::window()
            .and_then(|w| Some(w.alert_with_message(&json[..json.len().min(500)]).ok()));
    });

    // Обработка экспорта в Excel
    let handle_excel_export = move || {
        let page_items = items.get();
        if page_items.is_empty() {
            web_sys::window()
                .and_then(|w| Some(w.alert_with_message("Нет данных для экспорта").ok()));
            return;
        }

        let filename = format!(
            "nomenclature_{}.csv",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        );

        if let Err(e) = export_to_excel(&page_items, &filename) {
            web_sys::window().and_then(|w| {
                Some(
                    w.alert_with_message(&format!("Ошибка экспорта: {}", e))
                        .ok(),
                )
            });
        }
    };

    view! {
        <div class="page page--wide">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Номенклатура"</h1>
                    <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                        {move || state.get().total_count.to_string()}
                    </Badge>
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| load()
                        disabled=move || is_loading.get()
                    >
                        {icon("refresh")}
                        {move || if is_loading.get() { " Загрузка..." } else { " Обновить" }}
                    </Button>
                    <Button appearance=ButtonAppearance::Primary on_click=move |_| open_excel_importer.run(())>
                        {icon("upload")}
                        " Импорт"
                    </Button>
                    <Button appearance=ButtonAppearance::Secondary on_click=move |_| handle_excel_export()>
                        {icon("download")}
                        " Excel (страница)"
                    </Button>
                </div>
            </div>

            {move || error.get().map(|e| view! {
                <div class="warning-box warning-box--error">
                    <span class="warning-box__icon">"⚠"</span>
                    <span class="warning-box__text">{e}</span>
                </div>
            })}

            <div class="filter-panel">
                <div class="filter-panel-header">
                    <div class="filter-panel-header__left">
                        {icon("filter")}
                        <span class="filter-panel__title">"Фильтры"</span>
                    </div>

                    <div class="filter-panel-header__center">
                        <PaginationControls
                            current_page=Signal::derive(move || state.get().page)
                            total_pages=Signal::derive(move || state.get().total_pages)
                            total_count=Signal::derive(move || state.get().total_count)
                            page_size=Signal::derive(move || state.get().page_size)
                            on_page_change=Callback::new(go_to_page)
                            on_page_size_change=Callback::new(change_page_size)
                        />
                    </div>

                    <div class="filter-panel-header__right">
                        <span class="filter-panel__count">
                            {"Выбрано: "}{move || state.get().selected_ids.len()}
                    </span>
                </div>
            </div>

                <div class="filter-panel__collapsible filter-panel__collapsible--expanded">
                    <div class="filter-panel-content">
                        <Flex gap=FlexGap::Small align=FlexAlign::End>
                            <div style="min-width: 360px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Поиск:"</Label>
                                    <Input
                                        value=q
                                        placeholder="Артикул, наименование, измерения… (мин. 3 символа)"
                                    />
                                </Flex>
                            </div>

                            <div style="min-width: 220px; padding-bottom: 2px;">
                                <Checkbox checked=only_mp label="Только из маркетплейсов" />
                            </div>
                        </Flex>
                    </div>
                </div>
            </div>

            <div class="page__content">
                <div style="width: 100%; overflow-x: auto;">
                <Table attr:style="width: 100%;">
                    <TableHeader>
                        <TableRow>
                            <TableHeaderCell resizable=false class="fixed-checkbox-column">
                                        <input
                                            type="checkbox"
                                            class="table__checkbox"
                                    prop:checked=move || all_on_page_selected.get()
                                    on:change=move |ev| {
                                        toggle_select_all_on_page(event_target_checked(&ev));
                                    }
                                />
                            </TableHeaderCell>

                            <TableHeaderCell resizable=true min_width=120.0>
                                "Артикул"
                                <span
                                    class={move || format!("table__header-sort-indicator {}", get_sort_class("article", &sort_field.get()))}
                                    on:click=move |e| {
                                        e.stop_propagation();
                                        state.update(|s| {
                                            if s.sort_field == "article" {
                                                s.sort_ascending = !s.sort_ascending;
                                        } else {
                                                s.sort_field = "article".to_string();
                                                s.sort_ascending = true;
                                            }
                                            s.page = 0;
                                        });
                                        sort_field.set(state.get_untracked().sort_field.clone());
                                        sort_ascending.set(state.get_untracked().sort_ascending);
                                        load();
                                    }
                                >
                                    {move || get_sort_indicator("article", &sort_field.get(), sort_ascending.get())}
                                </span>
                            </TableHeaderCell>

                            <TableHeaderCell resizable=true min_width=220.0>
                                "Наименование"
                                <span
                                    class={move || format!("table__header-sort-indicator {}", get_sort_class("description", &sort_field.get()))}
                                    on:click=move |e| {
                                        e.stop_propagation();
                                        state.update(|s| {
                                            if s.sort_field == "description" {
                                                s.sort_ascending = !s.sort_ascending;
                                        } else {
                                                s.sort_field = "description".to_string();
                                                s.sort_ascending = true;
                                            }
                                            s.page = 0;
                                        });
                                        sort_field.set(state.get_untracked().sort_field.clone());
                                        sort_ascending.set(state.get_untracked().sort_ascending);
                                        load();
                                    }
                                >
                                    {move || get_sort_indicator("description", &sort_field.get(), sort_ascending.get())}
                                </span>
                            </TableHeaderCell>

                            <TableHeaderCell resizable=true min_width=140.0>
                                "Категория"
                                <span
                                    class={move || format!("table__header-sort-indicator {}", get_sort_class("dim1_category", &sort_field.get()))}
                                    on:click=move |e| {
                                        e.stop_propagation();
                                        state.update(|s| {
                                            if s.sort_field == "dim1_category" {
                                                s.sort_ascending = !s.sort_ascending;
                                        } else {
                                                s.sort_field = "dim1_category".to_string();
                                                s.sort_ascending = true;
                                            }
                                            s.page = 0;
                                        });
                                        sort_field.set(state.get_untracked().sort_field.clone());
                                        sort_ascending.set(state.get_untracked().sort_ascending);
                                        load();
                                    }
                                >
                                    {move || get_sort_indicator("dim1_category", &sort_field.get(), sort_ascending.get())}
                                </span>
                            </TableHeaderCell>

                            <TableHeaderCell resizable=true min_width=140.0>
                                "Линейка"
                                <span
                                    class={move || format!("table__header-sort-indicator {}", get_sort_class("dim2_line", &sort_field.get()))}
                                                    on:click=move |e| {
                                        e.stop_propagation();
                                        state.update(|s| {
                                            if s.sort_field == "dim2_line" {
                                                s.sort_ascending = !s.sort_ascending;
                                            } else {
                                                s.sort_field = "dim2_line".to_string();
                                                s.sort_ascending = true;
                                            }
                                            s.page = 0;
                                        });
                                        sort_field.set(state.get_untracked().sort_field.clone());
                                        sort_ascending.set(state.get_untracked().sort_ascending);
                                        load();
                                    }
                                >
                                    {move || get_sort_indicator("dim2_line", &sort_field.get(), sort_ascending.get())}
                                </span>
                            </TableHeaderCell>

                            <TableHeaderCell resizable=true min_width=160.0>
                                "Модель"
                                <span
                                    class={move || format!("table__header-sort-indicator {}", get_sort_class("dim3_model", &sort_field.get()))}
                                    on:click=move |e| {
                                        e.stop_propagation();
                                        state.update(|s| {
                                            if s.sort_field == "dim3_model" {
                                                s.sort_ascending = !s.sort_ascending;
                                            } else {
                                                s.sort_field = "dim3_model".to_string();
                                                s.sort_ascending = true;
                                            }
                                            s.page = 0;
                                        });
                                        sort_field.set(state.get_untracked().sort_field.clone());
                                        sort_ascending.set(state.get_untracked().sort_ascending);
                                        load();
                                    }
                                >
                                    {move || get_sort_indicator("dim3_model", &sort_field.get(), sort_ascending.get())}
                                </span>
                            </TableHeaderCell>

                            <TableHeaderCell resizable=true min_width=120.0>
                                "Формат"
                                <span
                                    class={move || format!("table__header-sort-indicator {}", get_sort_class("dim4_format", &sort_field.get()))}
                                    on:click=move |e| {
                                        e.stop_propagation();
                                        state.update(|s| {
                                            if s.sort_field == "dim4_format" {
                                                s.sort_ascending = !s.sort_ascending;
                                            } else {
                                                s.sort_field = "dim4_format".to_string();
                                                s.sort_ascending = true;
                                            }
                                            s.page = 0;
                                        });
                                        sort_field.set(state.get_untracked().sort_field.clone());
                                        sort_ascending.set(state.get_untracked().sort_ascending);
                                        load();
                                    }
                                >
                                    {move || get_sort_indicator("dim4_format", &sort_field.get(), sort_ascending.get())}
                                </span>
                            </TableHeaderCell>

                            <TableHeaderCell resizable=true min_width=140.0>
                                "Раковина"
                                <span
                                    class={move || format!("table__header-sort-indicator {}", get_sort_class("dim5_sink", &sort_field.get()))}
                                    on:click=move |e| {
                                        e.stop_propagation();
                                        state.update(|s| {
                                            if s.sort_field == "dim5_sink" {
                                                s.sort_ascending = !s.sort_ascending;
                                            } else {
                                                s.sort_field = "dim5_sink".to_string();
                                                s.sort_ascending = true;
                                            }
                                            s.page = 0;
                                        });
                                        sort_field.set(state.get_untracked().sort_field.clone());
                                        sort_ascending.set(state.get_untracked().sort_ascending);
                                        load();
                                    }
                                >
                                    {move || get_sort_indicator("dim5_sink", &sort_field.get(), sort_ascending.get())}
                                </span>
                            </TableHeaderCell>

                            <TableHeaderCell resizable=true min_width=120.0>
                                "Размер"
                                <span
                                    class={move || format!("table__header-sort-indicator {}", get_sort_class("dim6_size", &sort_field.get()))}
                                    on:click=move |e| {
                                        e.stop_propagation();
                                        state.update(|s| {
                                            if s.sort_field == "dim6_size" {
                                                s.sort_ascending = !s.sort_ascending;
                                            } else {
                                                s.sort_field = "dim6_size".to_string();
                                                s.sort_ascending = true;
                                            }
                                            s.page = 0;
                                        });
                                        sort_field.set(state.get_untracked().sort_field.clone());
                                        sort_ascending.set(state.get_untracked().sort_ascending);
                                        load();
                                    }
                                >
                                    {move || get_sort_indicator("dim6_size", &sort_field.get(), sort_ascending.get())}
                                </span>
                            </TableHeaderCell>
                        </TableRow>
                    </TableHeader>

                    <TableBody>
                        {move || {
                            let data = items.get();
                            if data.is_empty() && !is_loading.get() {
                                return view! {
                                    <TableRow>
                                        <TableCell attr:colspan="9">
                                            <TableCellLayout>
                                                <span class="table__cell--muted">"Нет данных"</span>
                                            </TableCellLayout>
                                        </TableCell>
                                    </TableRow>
                                }.into_any();
                            }

                            data.into_iter().map(|row| {
                                let id = row.base.id.as_string();
                                let id_for_checkbox = id.clone();
                                let id_for_checkbox2 = id.clone();
                                let article_for_tab = row.article.clone();
                                let description_for_tab = row.base.description.clone();
                                let id_for_open_article = id.clone();
                                let article_for_open_article = article_for_tab.clone();
                                let description_for_open_article = description_for_tab.clone();
                                let id_for_open_desc = id.clone();

                                view! {
                                    <TableRow>
                                        <TableCell class="fixed-checkbox-column" on:click=|e| e.stop_propagation()>
                                                        <input
                                                            type="checkbox"
                                                            class="table__checkbox"
                                               prop:checked=move || state.get().selected_ids.contains(&id_for_checkbox)
                                               on:change=move |ev| {
                                                    toggle_select(id_for_checkbox2.clone(), event_target_checked(&ev));
                                                }
                                            />
                                        </TableCell>

                                        <TableCell>
                                            <TableCellLayout>
                                                <a
                                                    href="#"
                                                    style="color: var(--colorBrandForeground1); text-decoration: none; cursor: pointer;"
                                                    on:click=move |e| {
                                                        e.prevent_default();
                                                        open_details_tab(id_for_open_article.clone(), article_for_open_article.clone(), description_for_open_article.clone());
                                                    }
                                                >
                                                    {row.article}
                                                </a>
                                            </TableCellLayout>
                                        </TableCell>

                                        <TableCell>
                                            <TableCellLayout truncate=true>
                                                <a
                                                    href="#"
                                                    style="color: var(--colorBrandForeground1); text-decoration: none; cursor: pointer;"
                                                    on:click=move |e| {
                                                        e.prevent_default();
                                                        open_details_tab(id_for_open_desc.clone(), article_for_tab.clone(), description_for_tab.clone());
                                                    }
                                                >
                                                    {row.base.description}
                                                </a>
                                            </TableCellLayout>
                                        </TableCell>
                                        <TableCell><TableCellLayout truncate=true>{row.dim1_category}</TableCellLayout></TableCell>
                                        <TableCell><TableCellLayout truncate=true>{row.dim2_line}</TableCellLayout></TableCell>
                                        <TableCell><TableCellLayout truncate=true>{row.dim3_model}</TableCellLayout></TableCell>
                                        <TableCell><TableCellLayout truncate=true>{row.dim4_format}</TableCellLayout></TableCell>
                                        <TableCell><TableCellLayout truncate=true>{row.dim5_sink}</TableCellLayout></TableCell>
                                        <TableCell><TableCellLayout truncate=true>{row.dim6_size}</TableCellLayout></TableCell>
                                    </TableRow>
                                }
                            }).collect_view().into_any()
                        }}
                    </TableBody>
                </Table>
                </div>
            </div>
        </div>
    }
}
