use crate::domain::a007_marketplace_product::ui::details::MarketplaceProductDetails;
use crate::shared::export::{export_to_excel, ExcelExportable};
use crate::shared::icons::icon;
use crate::shared::list_utils::{
    get_sort_indicator, highlight_matches, SearchInput, Searchable, Sortable,
};
use contracts::domain::a004_nomenclature::aggregate::Nomenclature;
use contracts::domain::a005_marketplace::aggregate::Marketplace;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::a007_marketplace_product::aggregate::MarketplaceProduct;
use leptos::prelude::*;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct MarketplaceProductRow {
    pub id: String,
    pub code: String,
    pub description: String,
    pub article: String,
    pub marketplace_sku: String,
    pub barcode: Option<String>,
    pub marketplace_ref: String,
    pub marketplace_name: String,
    pub connection_mp_ref: String,
    pub connection_mp_name: String,
    pub nomenclature_ref: Option<String>,
    pub nomenclature_name: Option<String>,
}

impl MarketplaceProductRow {
    fn from_product(
        m: MarketplaceProduct,
        marketplace_map: &HashMap<String, String>,
        connection_mp_map: &HashMap<String, String>,
        nomenclature_map: &HashMap<String, String>,
    ) -> Self {
        use contracts::domain::common::AggregateId;

        let marketplace_name = marketplace_map
            .get(&m.marketplace_ref)
            .cloned()
            .unwrap_or_else(|| "Неизвестно".to_string());

        let connection_mp_name = connection_mp_map
            .get(&m.connection_mp_ref)
            .cloned()
            .unwrap_or_else(|| "Неизвестно".to_string());

        let nomenclature_name = m
            .nomenclature_ref
            .as_ref()
            .and_then(|id| nomenclature_map.get(id).cloned());

        Self {
            id: m.base.id.as_string(),
            code: m.base.code,
            description: m.base.description,
            article: m.article,
            marketplace_sku: m.marketplace_sku,
            barcode: m.barcode,
            marketplace_ref: m.marketplace_ref,
            marketplace_name,
            connection_mp_ref: m.connection_mp_ref,
            connection_mp_name,
            nomenclature_ref: m.nomenclature_ref.clone(),
            nomenclature_name,
        }
    }
}

impl ExcelExportable for MarketplaceProductRow {
    fn headers() -> Vec<&'static str> {
        vec![
            "Код",
            "Маркетплейс",
            "Кабинет",
            "Описание",
            "Артикул",
            "SKU",
            "Штрихкод",
            "Связь 1С",
            "Номенклатура 1С",
        ]
    }

    fn to_csv_row(&self) -> Vec<String> {
        vec![
            self.code.clone(),
            self.marketplace_name.clone(),
            self.connection_mp_name.clone(),
            self.description.clone(),
            self.article.clone(),
            self.marketplace_sku.clone(),
            self.barcode.clone().unwrap_or_else(|| "-".to_string()),
            if self.nomenclature_ref.is_some() {
                "Да"
            } else {
                "Нет"
            }
            .to_string(),
            self.nomenclature_name
                .clone()
                .unwrap_or_else(|| "-".to_string()),
        ]
    }
}

impl Searchable for MarketplaceProductRow {
    fn matches_filter(&self, filter: &str) -> bool {
        let filter_lower = filter.to_lowercase();

        self.code.to_lowercase().contains(&filter_lower)
            || self.description.to_lowercase().contains(&filter_lower)
            || self.article.to_lowercase().contains(&filter_lower)
            || self.marketplace_sku.to_lowercase().contains(&filter_lower)
            || self.marketplace_name.to_lowercase().contains(&filter_lower)
            || self
                .connection_mp_name
                .to_lowercase()
                .contains(&filter_lower)
            || self
                .barcode
                .as_ref()
                .map_or(false, |b| b.to_lowercase().contains(&filter_lower))
            || self
                .nomenclature_name
                .as_ref()
                .map_or(false, |n| n.to_lowercase().contains(&filter_lower))
    }

    fn get_field_value(&self, field: &str) -> Option<String> {
        match field {
            "code" => Some(self.code.clone()),
            "description" => Some(self.description.clone()),
            "article" => Some(self.article.clone()),
            "marketplace_sku" => Some(self.marketplace_sku.clone()),
            "marketplace_name" => Some(self.marketplace_name.clone()),
            "connection_mp_name" => Some(self.connection_mp_name.clone()),
            "barcode" => self.barcode.clone(),
            "nomenclature_name" => self.nomenclature_name.clone(),
            _ => None,
        }
    }
}

impl Sortable for MarketplaceProductRow {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "code" => self.code.to_lowercase().cmp(&other.code.to_lowercase()),
            "marketplace_name" => self
                .marketplace_name
                .to_lowercase()
                .cmp(&other.marketplace_name.to_lowercase()),
            "connection_mp_name" => self
                .connection_mp_name
                .to_lowercase()
                .cmp(&other.connection_mp_name.to_lowercase()),
            "description" => self
                .description
                .to_lowercase()
                .cmp(&other.description.to_lowercase()),
            "article" => self
                .article
                .to_lowercase()
                .cmp(&other.article.to_lowercase()),
            "marketplace_sku" => self
                .marketplace_sku
                .to_lowercase()
                .cmp(&other.marketplace_sku.to_lowercase()),
            "barcode" => {
                let a = self
                    .barcode
                    .as_ref()
                    .map(|s| s.to_lowercase())
                    .unwrap_or_default();
                let b = other
                    .barcode
                    .as_ref()
                    .map(|s| s.to_lowercase())
                    .unwrap_or_default();
                a.cmp(&b)
            }
            "nomenclature_name" => {
                let a = self
                    .nomenclature_name
                    .as_ref()
                    .map(|s| s.to_lowercase())
                    .unwrap_or_default();
                let b = other
                    .nomenclature_name
                    .as_ref()
                    .map(|s| s.to_lowercase())
                    .unwrap_or_default();
                a.cmp(&b)
            }
            _ => Ordering::Equal,
        }
    }
}

#[component]
#[allow(non_snake_case)]
pub fn MarketplaceProductList() -> impl IntoView {
    let (items, set_items) = signal::<Vec<MarketplaceProductRow>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);
    let (show_modal, set_show_modal) = signal(false);
    let (editing_id, set_editing_id) = signal::<Option<String>>(None);
    let (selected, set_selected) = signal::<HashSet<String>>(HashSet::new());
    let (marketplaces, set_marketplaces) = signal::<Vec<Marketplace>>(Vec::new());
    let (connections_mp, set_connections_mp) = signal::<Vec<ConnectionMP>>(Vec::new());
    let (nomenclatures, set_nomenclatures) = signal::<Vec<Nomenclature>>(Vec::new());
    let (selected_marketplace, set_selected_marketplace) = signal::<Option<String>>(None);

    // Поиск и сортировка
    let (filter_text, set_filter_text) = signal(String::new());
    let (sort_field, set_sort_field) = signal::<String>("code".to_string());
    let (sort_ascending, set_sort_ascending) = signal(true);

    // Создаем HashMap для маппинга marketplace_id -> description
    let marketplace_map = move || -> HashMap<String, String> {
        marketplaces
            .get()
            .into_iter()
            .map(|mp| {
                use contracts::domain::common::AggregateId;
                (mp.base.id.as_string(), mp.base.description.clone())
            })
            .collect()
    };

    // Создаем HashMap для маппинга connection_mp_ref -> description
    let connection_mp_map = move || -> HashMap<String, String> {
        connections_mp
            .get()
            .into_iter()
            .map(|conn| {
                use contracts::domain::common::AggregateId;
                (conn.base.id.as_string(), conn.base.description.clone())
            })
            .collect()
    };

    // Создаем HashMap для маппинга nomenclature_id -> description
    let nomenclature_map = move || -> HashMap<String, String> {
        nomenclatures
            .get()
            .into_iter()
            .map(|nom| {
                use contracts::domain::common::AggregateId;
                (nom.base.id.as_string(), nom.base.description.clone())
            })
            .collect()
    };

    // Обновляем отображаемые названия после загрузки справочников
    Effect::new(move |_| {
        let mp_map = marketplace_map();
        set_items.update(|rows| {
            for row in rows.iter_mut() {
                let name = mp_map
                    .get(&row.marketplace_ref)
                    .cloned()
                    .unwrap_or_else(|| "Неизвестно".to_string());
                row.marketplace_name = name;
            }
        });
    });

    Effect::new(move |_| {
        let conn_map = connection_mp_map();
        set_items.update(|rows| {
            for row in rows.iter_mut() {
                let name = conn_map
                    .get(&row.connection_mp_ref)
                    .cloned()
                    .unwrap_or_else(|| "Неизвестно".to_string());
                row.connection_mp_name = name;
            }
        });
    });

    Effect::new(move |_| {
        let nom_map = nomenclature_map();
        set_items.update(|rows| {
            for row in rows.iter_mut() {
                row.nomenclature_name = row
                    .nomenclature_ref
                    .as_ref()
                    .and_then(|id| nom_map.get(id).cloned());
            }
        });
    });

    let fetch = move || {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_marketplace_products().await {
                Ok(v) => {
                    let mp_map = marketplace_map();
                    let conn_map = connection_mp_map();
                    let nom_map = nomenclature_map();
                    let rows: Vec<MarketplaceProductRow> = v
                        .into_iter()
                        .map(|p| {
                            MarketplaceProductRow::from_product(p, &mp_map, &conn_map, &nom_map)
                        })
                        .collect();
                    set_items.set(rows);
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    let fetch_mp = move || {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_marketplaces().await {
                Ok(v) => {
                    set_marketplaces.set(v);
                }
                Err(e) => set_error.set(Some(format!("Ошибка загрузки маркетплейсов: {}", e))),
            }
        });
    };

    let fetch_connection_mp = move || {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_connections_mp().await {
                Ok(v) => {
                    set_connections_mp.set(v);
                }
                Err(e) => set_error.set(Some(format!("Ошибка загрузки кабинетов: {}", e))),
            }
        });
    };

    let fetch_nomenclature = move || {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_nomenclatures().await {
                Ok(v) => {
                    set_nomenclatures.set(v);
                }
                Err(e) => set_error.set(Some(format!("Ошибка загрузки номенклатуры: {}", e))),
            }
        });
    };

    let handle_create_new = move || {
        set_editing_id.set(None);
        set_show_modal.set(true);
    };

    let handle_edit = move |id: String| {
        let items_clone = items.get();
        if items_clone.iter().any(|item| item.id == id) {
            set_editing_id.set(Some(id));
            set_show_modal.set(true);
        }
    };

    let handle_cancel = move |_| {
        set_show_modal.set(false);
        set_editing_id.set(None);
    };

    let toggle_select = move |id: String, checked: bool| {
        set_selected.update(|s| {
            if checked {
                s.insert(id.clone());
            } else {
                s.remove(&id);
            }
        });
    };

    let clear_selection = move || set_selected.set(HashSet::new());

    let delete_selected = move || {
        let ids: Vec<String> = selected.get().into_iter().collect();
        if ids.is_empty() {
            return;
        }

        let count = ids.len();
        let confirmed = {
            if let Some(win) = web_sys::window() {
                win.confirm_with_message(&format!(
                    "Удалить выбранные элементы? Количество: {}",
                    count
                ))
                .unwrap_or(false)
            } else {
                false
            }
        };
        if !confirmed {
            return;
        }

        wasm_bindgen_futures::spawn_local(async move {
            let mut all_ok = true;
            for id in ids {
                if let Err(_) = delete_marketplace_product(&id).await {
                    all_ok = false;
                }
            }
            if all_ok {
                let _ = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(
                    &wasm_bindgen::JsValue::UNDEFINED,
                ))
                .await;
            }
        });
        fetch();
        clear_selection();
    };

    // Функция для получения отфильтрованных и отсортированных данных
    let get_filtered_sorted_items = move || -> Vec<MarketplaceProductRow> {
        let mut result: Vec<MarketplaceProductRow> = items
            .get()
            .into_iter()
            // Фильтр по маркетплейсу
            .filter(|row| {
                if let Some(ref mp_id) = selected_marketplace.get() {
                    &row.marketplace_ref == mp_id
                } else {
                    true
                }
            })
            // Поиск
            .filter(|row| {
                let filter = filter_text.get();
                if filter.trim().is_empty() || filter.trim().len() < 3 {
                    true
                } else {
                    row.matches_filter(&filter)
                }
            })
            .collect();

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

    let handle_export = move || {
        // Получаем текущие отфильтрованные элементы
        let filtered_items = get_filtered_sorted_items();

        if filtered_items.is_empty() {
            if let Some(win) = web_sys::window() {
                let _ = win.alert_with_message("Нет данных для экспорта");
            }
            return;
        }

        // Формируем имя файла с учетом фильтра
        let filename = if let Some(ref mp_id) = selected_marketplace.get() {
            // Находим имя маркетплейса
            let mp_name = marketplaces
                .get()
                .into_iter()
                .find(|mp| {
                    use contracts::domain::common::AggregateId;
                    mp.base.id.as_string() == *mp_id
                })
                .map(|mp| mp.base.description.clone())
                .unwrap_or_else(|| "маркетплейс".to_string());
            format!("товары_{}.csv", mp_name)
        } else {
            "товары_маркетплейсов.csv".to_string()
        };

        // Экспортируем данные
        if let Err(e) = export_to_excel(&filtered_items, &filename) {
            if let Some(win) = web_sys::window() {
                let _ = win.alert_with_message(&format!("Ошибка экспорта: {}", e));
            }
        }
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

    fetch_mp();
    fetch_connection_mp();
    fetch_nomenclature();
    fetch();

    view! {
        <div class="content">
            <div class="header">
                <h2>{"Товары маркетплейсов"}</h2>
                <div class="header-actions">
                    <SearchInput
                        value=filter_text
                        on_change=Callback::new(move |val: String| set_filter_text.set(val))
                        placeholder="Поиск по товарам...".to_string()
                    />
                    <select
                        class="form-control"
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            if value.is_empty() {
                                set_selected_marketplace.set(None);
                            } else {
                                set_selected_marketplace.set(Some(value));
                            }
                        }
                    >
                        <option value="">{"Все маркетплейсы"}</option>
                        {move || marketplaces.get().into_iter().map(|mp| {
                            use contracts::domain::common::AggregateId;
                            let id = mp.base.id.as_string();
                            let name = mp.base.description.clone();
                            view! {
                                <option value={id.clone()}>{name}</option>
                            }
                        }).collect_view()}
                    </select>
                    <button class="btn btn-primary" on:click=move |_| handle_create_new()>
                        {icon("plus")}
                        {"Новый товар"}
                    </button>
                    <button class="btn btn-success" on:click=move |_| {
                        wasm_bindgen_futures::spawn_local(async move {
                            match fill_test_data().await {
                                Ok(_) => fetch(),
                                Err(e) => set_error.set(Some(format!("Ошибка заполнения: {}", e))),
                            }
                        });
                    }>
                        {icon("download")}
                        {"Заполнить"}
                    </button>
                    <button class="btn btn-secondary" on:click=move |_| fetch()>
                        {icon("refresh")}
                        {"Обновить"}
                    </button>
                    <button class="btn btn-success" on:click=move |_| handle_export()>
                        {icon("excel")}
                        {"Excel"}
                    </button>
                    <button class="btn btn-danger" on:click=move |_| delete_selected() disabled={move || selected.get().is_empty()}>
                        {icon("delete")}
                        {move || format!("Удалить ({})", selected.get().len())}
                    </button>
                </div>
            </div>

            {move || error.get().map(|e| view! { <div class="error">{e}</div> })}

            <div class="table-container">
                <table>
                    <thead>
                        <tr>
                            <th></th>
                            <th
                                class="cursor-pointer user-select-none"
                                on:click=toggle_sort("code")
                                title="Сортировать"
                            >
                                {move || format!("Код{}", get_sort_indicator(&sort_field.get(), "code", sort_ascending.get()))}
                            </th>
                            <th
                                class="cursor-pointer user-select-none"
                                on:click=toggle_sort("marketplace_name")
                                title="Сортировать"
                            >
                                {move || format!("Маркетплейс{}", get_sort_indicator(&sort_field.get(), "marketplace_name", sort_ascending.get()))}
                            </th>
                            <th
                                class="cursor-pointer user-select-none"
                                on:click=toggle_sort("connection_mp_name")
                                title="Сортировать"
                            >
                                {move || format!("Кабинет{}", get_sort_indicator(&sort_field.get(), "connection_mp_name", sort_ascending.get()))}
                            </th>
                            <th
                                class="cursor-pointer user-select-none"
                                on:click=toggle_sort("description")
                                title="Сортировать"
                            >
                                {move || format!("Описание{}", get_sort_indicator(&sort_field.get(), "description", sort_ascending.get()))}
                            </th>
                            <th
                                class="cursor-pointer user-select-none"
                                on:click=toggle_sort("article")
                                title="Сортировать"
                            >
                                {move || format!("Артикул{}", get_sort_indicator(&sort_field.get(), "article", sort_ascending.get()))}
                            </th>
                            <th
                                class="cursor-pointer user-select-none"
                                on:click=toggle_sort("marketplace_sku")
                                title="Сортировать"
                            >
                                {move || format!("SKU{}", get_sort_indicator(&sort_field.get(), "marketplace_sku", sort_ascending.get()))}
                            </th>
                            <th
                                class="cursor-pointer user-select-none"
                                on:click=toggle_sort("barcode")
                                title="Сортировать"
                            >
                                {move || format!("Штрихкод{}", get_sort_indicator(&sort_field.get(), "barcode", sort_ascending.get()))}
                            </th>
                            <th>{"Связь 1С"}</th>
                            <th
                                class="cursor-pointer user-select-none"
                                on:click=toggle_sort("nomenclature_name")
                                title="Сортировать"
                            >
                                {move || format!("Номенклатура 1С{}", get_sort_indicator(&sort_field.get(), "nomenclature_name", sort_ascending.get()))}
                            </th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            let filtered = get_filtered_sorted_items();
                            let current_filter = filter_text.get();

                            filtered.into_iter().map(|row| {
                                let id = row.id.clone();

                                // Подсветка совпадений в ячейках
                                let code_view = if current_filter.len() >= 3 {
                                    highlight_matches(&row.code, &current_filter)
                                } else {
                                    view! { <span>{row.code.clone()}</span> }.into_any()
                                };

                                let marketplace_name_view = if current_filter.len() >= 3 {
                                    highlight_matches(&row.marketplace_name, &current_filter)
                                } else {
                                    view! { <span>{row.marketplace_name.clone()}</span> }.into_any()
                                };

                                let description_view = if current_filter.len() >= 3 {
                                    highlight_matches(&row.description, &current_filter)
                                } else {
                                    view! { <span>{row.description.clone()}</span> }.into_any()
                                };

                                let article_view = if current_filter.len() >= 3 {
                                    highlight_matches(&row.article, &current_filter)
                                } else {
                                    view! { <span>{row.article.clone()}</span> }.into_any()
                                };

                                let sku_view = if current_filter.len() >= 3 {
                                    highlight_matches(&row.marketplace_sku, &current_filter)
                                } else {
                                    view! { <span>{row.marketplace_sku.clone()}</span> }.into_any()
                                };

                                let barcode_str = row.barcode.clone().unwrap_or_else(|| "-".to_string());
                                let barcode_view = if current_filter.len() >= 3 {
                                    highlight_matches(&barcode_str, &current_filter)
                                } else {
                                    view! { <span>{barcode_str}</span> }.into_any()
                                };

                                let nomenclature_str = row.nomenclature_name.clone().unwrap_or_else(|| "-".to_string());
                                let nomenclature_view = if current_filter.len() >= 3 {
                                    highlight_matches(&nomenclature_str, &current_filter)
                                } else {
                                    view! { <span>{nomenclature_str}</span> }.into_any()
                                };

                                view! {
                                    <tr on:click=move |_| handle_edit(id.clone())>
                                        <td>
                                            <input type="checkbox"
                                                prop:checked={
                                                    let selected = selected.get();
                                                    selected.contains(&id)
                                                }
                                                on:click=move |ev| ev.stop_propagation()
                                                on:change={
                                                    let id2 = id.clone();
                                                    move |ev| {
                                                        let checked = event_target_checked(&ev);
                                                        toggle_select(id2.clone(), checked);
                                                    }
                                                }
                                            />
                                        </td>
                                        <td>{code_view}</td>
                                        <td>{marketplace_name_view}</td>
                                        <td>{
                                            if current_filter.len() >= 3 {
                                                highlight_matches(&row.connection_mp_name, &current_filter)
                                            } else {
                                                view! { <span>{row.connection_mp_name.clone()}</span> }.into_any()
                                            }
                                        }</td>
                                        <td>{description_view}</td>
                                        <td>{article_view}</td>
                                        <td>{sku_view}</td>
                                        <td>{barcode_view}</td>
                                        <td style="text-align: center;">
                                            {if row.nomenclature_ref.is_some() {
                                                view! { <span style="color: green; font-weight: bold;">{"✓"}</span> }.into_any()
                                            } else {
                                                view! { <span style="color: red;">{"✗"}</span> }.into_any()
                                            }}
                                        </td>
                                        <td>{nomenclature_view}</td>
                                    </tr>
                                }
                            }).collect_view()
                        }}
                    </tbody>
                </table>
            </div>

            {move || if show_modal.get() {
                view! {
                    <div class="modal-overlay">
                        <div class="modal-content">
                            <MarketplaceProductDetails
                                id=editing_id.get()
                                on_saved=Rc::new(move |_| { set_show_modal.set(false); set_editing_id.set(None); fetch(); })
                                on_cancel=Rc::new(move |_| handle_cancel(()))
                            />
                        </div>
                    </div>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }}
        </div>
    }
}

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

async fn fetch_marketplace_products() -> Result<Vec<MarketplaceProduct>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/marketplace_product", api_base());
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
    let data: Vec<MarketplaceProduct> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}

async fn delete_marketplace_product(id: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("DELETE");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/marketplace_product/{}", api_base(), id);
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
    Ok(())
}

async fn fill_test_data() -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/marketplace_product/testdata", api_base());
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
    Ok(())
}

async fn fetch_marketplaces() -> Result<Vec<Marketplace>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/marketplace", api_base());
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
    let data: Vec<Marketplace> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}

async fn fetch_connections_mp() -> Result<Vec<ConnectionMP>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/connection_mp", api_base());
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
    let data: Vec<ConnectionMP> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}

async fn fetch_nomenclatures() -> Result<Vec<Nomenclature>, String> {
    use wasm_bindgen::JsCast;
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
