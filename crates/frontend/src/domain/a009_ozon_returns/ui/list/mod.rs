use crate::shared::icons::icon;
use crate::shared::list_utils::{
    get_sort_indicator, highlight_matches, SearchInput, Searchable, Sortable,
};
use contracts::domain::a002_organization::aggregate::Organization;
use contracts::domain::a005_marketplace::aggregate::Marketplace;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::a009_ozon_returns::aggregate::OzonReturns;
use leptos::prelude::*;
use std::cmp::Ordering;

#[derive(Clone, Debug)]
pub struct OzonReturnsRow {
    pub id: String,
    pub connection_name: String,
    pub organization_name: String,
    pub marketplace_name: String,
    pub return_id: String,
    pub return_date: String,
    pub return_reason_name: String,
    pub return_type: String,
    pub order_number: String,
    pub sku: String,
    pub product_name: String,
    pub quantity: i32,
    pub price: String,
}

impl OzonReturnsRow {
    fn from_return(
        r: OzonReturns,
        conn_map: &std::collections::HashMap<String, String>,
        org_map: &std::collections::HashMap<String, String>,
        mp_map: &std::collections::HashMap<String, String>,
    ) -> Self {
        use contracts::domain::common::AggregateId;
        let connection_name = conn_map
            .get(&r.connection_id)
            .cloned()
            .unwrap_or_else(|| "?".to_string());
        let organization_name = org_map
            .get(&r.organization_id)
            .cloned()
            .unwrap_or_else(|| "?".to_string());
        let marketplace_name = mp_map
            .get(&r.marketplace_id)
            .cloned()
            .unwrap_or_else(|| "?".to_string());
        Self {
            id: r.base.id.as_string(),
            connection_name,
            organization_name,
            marketplace_name,
            return_id: r.return_id,
            return_date: r.return_date.format("%Y-%m-%d").to_string(),
            return_reason_name: r.return_reason_name,
            return_type: r.return_type,
            order_number: r.order_number,
            sku: r.sku,
            product_name: r.product_name,
            quantity: r.quantity,
            price: format!("{:.2}", r.price),
        }
    }
}

impl Searchable for OzonReturnsRow {
    fn matches_filter(&self, filter: &str) -> bool {
        let f = filter.to_lowercase();
        self.connection_name.to_lowercase().contains(&f)
            || self.organization_name.to_lowercase().contains(&f)
            || self.marketplace_name.to_lowercase().contains(&f)
            || self.return_id.to_lowercase().contains(&f)
            || self.return_reason_name.to_lowercase().contains(&f)
            || self.order_number.to_lowercase().contains(&f)
            || self.sku.to_lowercase().contains(&f)
            || self.product_name.to_lowercase().contains(&f)
    }

    fn get_field_value(&self, field: &str) -> Option<String> {
        match field {
            "connection" => Some(self.connection_name.clone()),
            "organization" => Some(self.organization_name.clone()),
            "marketplace" => Some(self.marketplace_name.clone()),
            "return_id" => Some(self.return_id.clone()),
            "return_date" => Some(self.return_date.clone()),
            "return_reason" => Some(self.return_reason_name.clone()),
            "return_type" => Some(self.return_type.clone()),
            "order_number" => Some(self.order_number.clone()),
            "sku" => Some(self.sku.clone()),
            "product_name" => Some(self.product_name.clone()),
            "quantity" => Some(self.quantity.to_string()),
            "price" => Some(self.price.clone()),
            _ => None,
        }
    }
}

impl Sortable for OzonReturnsRow {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "connection" => self
                .connection_name
                .to_lowercase()
                .cmp(&other.connection_name.to_lowercase()),
            "organization" => self
                .organization_name
                .to_lowercase()
                .cmp(&other.organization_name.to_lowercase()),
            "marketplace" => self
                .marketplace_name
                .to_lowercase()
                .cmp(&other.marketplace_name.to_lowercase()),
            "return_id" => self.return_id.cmp(&other.return_id),
            "return_date" => self.return_date.cmp(&other.return_date),
            "return_reason" => self
                .return_reason_name
                .to_lowercase()
                .cmp(&other.return_reason_name.to_lowercase()),
            "return_type" => self
                .return_type
                .to_lowercase()
                .cmp(&other.return_type.to_lowercase()),
            "order_number" => self.order_number.cmp(&other.order_number),
            "sku" => self.sku.cmp(&other.sku),
            "product_name" => self
                .product_name
                .to_lowercase()
                .cmp(&other.product_name.to_lowercase()),
            "quantity" => self.quantity.cmp(&other.quantity),
            "price" => {
                let a = self.price.parse::<f64>().unwrap_or(0.0);
                let b = other.price.parse::<f64>().unwrap_or(0.0);
                a.partial_cmp(&b).unwrap_or(Ordering::Equal)
            }
            _ => Ordering::Equal,
        }
    }
}

#[component]
#[allow(non_snake_case)]
pub fn OzonReturnsList() -> impl IntoView {
    use std::collections::HashMap;
    let (items, set_items) = signal::<Vec<OzonReturnsRow>>(Vec::new());
    let (returns, set_returns) = signal::<Vec<OzonReturns>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);
    let (filter_text, set_filter_text) = signal(String::new());
    let (sort_field, set_sort_field) = signal::<String>("return_date".to_string());
    let (sort_ascending, set_sort_ascending) = signal(false);

    let (connections, set_connections) = signal::<Vec<ConnectionMP>>(Vec::new());
    let (organizations, set_organizations) = signal::<Vec<Organization>>(Vec::new());
    let (marketplaces, set_marketplaces) = signal::<Vec<Marketplace>>(Vec::new());

    let conn_map = move || -> HashMap<String, String> {
        connections
            .get()
            .into_iter()
            .map(|x| {
                use contracts::domain::common::AggregateId;
                (x.base.id.as_string(), x.base.description)
            })
            .collect()
    };
    let org_map = move || -> HashMap<String, String> {
        organizations
            .get()
            .into_iter()
            .map(|x| {
                use contracts::domain::common::AggregateId;
                (x.base.id.as_string(), x.base.description)
            })
            .collect()
    };
    let mp_map = move || -> HashMap<String, String> {
        marketplaces
            .get()
            .into_iter()
            .map(|x| {
                use contracts::domain::common::AggregateId;
                (x.base.id.as_string(), x.base.description)
            })
            .collect()
    };

    let compose_rows = move |source: &Vec<OzonReturns>| -> Vec<OzonReturnsRow> {
        source
            .iter()
            .cloned()
            .map(|r| OzonReturnsRow::from_return(r, &conn_map(), &org_map(), &mp_map()))
            .collect()
    };

    let fetch = move || {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_returns().await {
                Ok(v) => {
                    let rows = compose_rows(&v);
                    set_returns.set(v);
                    set_items.set(rows);
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    let fetch_refs = move || {
        wasm_bindgen_futures::spawn_local(async move {
            let c = fetch_connections().await;
            let o = fetch_organizations().await;
            let m = fetch_marketplaces().await;
            if let Ok(v) = c {
                set_connections.set(v);
            }
            if let Ok(v) = o {
                set_organizations.set(v);
            }
            if let Ok(v) = m {
                set_marketplaces.set(v);
            }
            // Пересобрать строки с учетом загруженных справочников
            let current = returns.get();
            if !current.is_empty() {
                set_items.set(compose_rows(&current));
            }
        });
    };

    let get_filtered_sorted = move || -> Vec<OzonReturnsRow> {
        let mut result: Vec<OzonReturnsRow> = items
            .get()
            .into_iter()
            .filter(|row| {
                let filter = filter_text.get();
                if filter.trim().is_empty() || filter.trim().len() < 3 {
                    true
                } else {
                    row.matches_filter(&filter)
                }
            })
            .collect();
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

    fetch_refs();
    fetch();

    view! {
        <div class="content">
            <div class="header">
                <h2>{"Возвраты OZON"}</h2>
                <div class="header-actions">
                    <SearchInput
                        value=filter_text
                        on_change=Callback::new(move |val: String| set_filter_text.set(val))
                        placeholder="Поиск по возвратам...".to_string()
                    />
                    <button class="btn btn-secondary" on:click=move |_| fetch()>
                        {icon("refresh")}
                        {"Обновить"}
                    </button>
                </div>
            </div>

            {move || error.get().map(|e| view! { <div class="error">{e}</div> })}

            <div class="table-container">
                <table>
                    <thead>
                        <tr>
                            <th class="cursor-pointer user-select-none" on:click=toggle_sort("return_date") title="Сортировать">
                                {move || format!("Дата{}", get_sort_indicator(&sort_field.get(), "return_date", sort_ascending.get()))}
                            </th>
                            <th class="cursor-pointer user-select-none" on:click=toggle_sort("connection") title="Сортировать">
                                {move || format!("Подключение{}", get_sort_indicator(&sort_field.get(), "connection", sort_ascending.get()))}
                            </th>
                            <th class="cursor-pointer user-select-none" on:click=toggle_sort("return_id") title="Сортировать">
                                {move || format!("ID возврата{}", get_sort_indicator(&sort_field.get(), "return_id", sort_ascending.get()))}
                            </th>
                            <th class="cursor-pointer user-select-none" on:click=toggle_sort("return_reason") title="Сортировать">
                                {move || format!("Причина{}", get_sort_indicator(&sort_field.get(), "return_reason", sort_ascending.get()))}
                            </th>
                            <th class="cursor-pointer user-select-none" on:click=toggle_sort("order_number") title="Сортировать">
                                {move || format!("Заказ{}", get_sort_indicator(&sort_field.get(), "order_number", sort_ascending.get()))}
                            </th>
                            <th class="cursor-pointer user-select-none" on:click=toggle_sort("sku") title="Сортировать">
                                {move || format!("SKU{}", get_sort_indicator(&sort_field.get(), "sku", sort_ascending.get()))}
                            </th>
                            <th class="cursor-pointer user-select-none" on:click=toggle_sort("product_name") title="Сортировать">
                                {move || format!("Товар{}", get_sort_indicator(&sort_field.get(), "product_name", sort_ascending.get()))}
                            </th>
                            <th class="cursor-pointer user-select-none" on:click=toggle_sort("quantity") title="Сортировать">
                                {move || format!("Кол-во{}", get_sort_indicator(&sort_field.get(), "quantity", sort_ascending.get()))}
                            </th>
                            <th class="cursor-pointer user-select-none" on:click=toggle_sort("price") title="Сортировать">
                                {move || format!("Цена{}", get_sort_indicator(&sort_field.get(), "price", sort_ascending.get()))}
                            </th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            let filtered = get_filtered_sorted();
                            let current_filter = filter_text.get();
                            filtered.into_iter().map(|row| {
                                let conn_view = if current_filter.len() >= 3 { highlight_matches(&row.connection_name, &current_filter) } else { view!{ <span>{row.connection_name.clone()}</span> }.into_any() };
                                let return_id_view = if current_filter.len() >= 3 { highlight_matches(&row.return_id, &current_filter) } else { view!{ <span>{row.return_id.clone()}</span> }.into_any() };
                                let reason_view = if current_filter.len() >= 3 { highlight_matches(&row.return_reason_name, &current_filter) } else { view!{ <span>{row.return_reason_name.clone()}</span> }.into_any() };
                                let order_view = if current_filter.len() >= 3 { highlight_matches(&row.order_number, &current_filter) } else { view!{ <span>{row.order_number.clone()}</span> }.into_any() };
                                let sku_view = if current_filter.len() >= 3 { highlight_matches(&row.sku, &current_filter) } else { view!{ <span>{row.sku.clone()}</span> }.into_any() };
                                let product_view = if current_filter.len() >= 3 { highlight_matches(&row.product_name, &current_filter) } else { view!{ <span>{row.product_name.clone()}</span> }.into_any() };
                                view! {
                                    <tr>
                                        <td>{row.return_date.clone()}</td>
                                        <td>{conn_view}</td>
                                        <td>{return_id_view}</td>
                                        <td>{reason_view}</td>
                                        <td>{order_view}</td>
                                        <td>{sku_view}</td>
                                        <td>{product_view}</td>
                                        <td>{row.quantity}</td>
                                        <td>{row.price.clone()}</td>
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

async fn fetch_returns() -> Result<Vec<OzonReturns>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};
    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);
    let url = format!("{}/api/ozon_returns", api_base());
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
    let data: Vec<OzonReturns> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}

async fn fetch_connections() -> Result<Vec<ConnectionMP>, String> {
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

async fn fetch_organizations() -> Result<Vec<Organization>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};
    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);
    let url = format!("{}/api/organization", api_base());
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
    let data: Vec<Organization> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
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
