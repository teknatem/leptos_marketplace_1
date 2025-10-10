use crate::domain::a007_marketplace_product::ui::details::MarketplaceProductDetails;
use crate::shared::icons::icon;
use contracts::domain::a005_marketplace::aggregate::Marketplace;
use contracts::domain::a007_marketplace_product::aggregate::MarketplaceProduct;
use leptos::prelude::*;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct MarketplaceProductRow {
    pub id: String,
    pub code: String,
    pub product_name: String,
    pub art: String,
    pub marketplace_sku: String,
    pub barcode: Option<String>,
    pub price: String,
    pub stock: String,
    pub marketplace_id: String,
    pub marketplace_name: String,
}

impl MarketplaceProductRow {
    fn from_product(m: MarketplaceProduct, marketplace_map: &HashMap<String, String>) -> Self {
        use contracts::domain::common::AggregateId;

        let marketplace_name = marketplace_map
            .get(&m.marketplace_id)
            .cloned()
            .unwrap_or_else(|| "Неизвестно".to_string());

        Self {
            id: m.base.id.as_string(),
            code: m.base.code,
            product_name: m.product_name,
            art: m.art,
            marketplace_sku: m.marketplace_sku,
            barcode: m.barcode,
            price: m
                .price
                .map(|p| format!("{:.2}", p))
                .unwrap_or_else(|| "-".to_string()),
            stock: m
                .stock
                .map(|s| s.to_string())
                .unwrap_or_else(|| "-".to_string()),
            marketplace_id: m.marketplace_id,
            marketplace_name,
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
    let (selected_marketplace, set_selected_marketplace) = signal::<Option<String>>(None);

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

    let fetch = move || {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_marketplace_products().await {
                Ok(v) => {
                    let mp_map = marketplace_map();
                    let rows: Vec<MarketplaceProductRow> = v
                        .into_iter()
                        .map(|p| MarketplaceProductRow::from_product(p, &mp_map))
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

    fetch_mp();
    fetch();

    view! {
        <div class="content">
            <div class="header">
                <h2>{"Товары маркетплейсов"}</h2>
                <div class="header-actions">
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
                            <th>{"Код"}</th>
                            <th>{"Маркетплейс"}</th>
                            <th>{"Наименование"}</th>
                            <th>{"Артикул"}</th>
                            <th>{"SKU"}</th>
                            <th>{"Штрихкод"}</th>
                            <th>{"Цена"}</th>
                            <th>{"Остаток"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || items.get().into_iter()
                            .filter(|row| {
                                if let Some(ref mp_id) = selected_marketplace.get() {
                                    &row.marketplace_id == mp_id
                                } else {
                                    true
                                }
                            })
                            .map(|row| {
                            let id = row.id.clone();
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
                                    <td>{row.code}</td>
                                    <td>{row.marketplace_name}</td>
                                    <td>{row.product_name}</td>
                                    <td>{row.art}</td>
                                    <td>{row.marketplace_sku}</td>
                                    <td>{row.barcode.unwrap_or_else(|| "-".to_string())}</td>
                                    <td>{row.price}</td>
                                    <td>{row.stock}</td>
                                </tr>
                            }
                        }).collect_view()}
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
