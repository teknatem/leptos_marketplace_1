use crate::shared::icons::icon;
use crate::shared::list_utils::{
    get_sort_indicator, highlight_matches, SearchInput, Searchable, Sortable,
};
use contracts::domain::a002_organization::aggregate::Organization;
use contracts::domain::a005_marketplace::aggregate::Marketplace;
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::a007_marketplace_product::aggregate::MarketplaceProduct;
use contracts::domain::a008_marketplace_sales::aggregate::MarketplaceSales;
use leptos::prelude::*;
use std::cmp::Ordering;

#[derive(Clone, Debug)]
pub struct MarketplaceSalesRow {
    pub id: String,
    pub connection_name: String,
    pub organization_name: String,
    pub marketplace_name: String,
    pub accrual_date: String,
    pub product_name: String,
    pub quantity: i32,
    pub revenue: String,
    pub operation_type: String,
}

impl MarketplaceSalesRow {
    fn from_sale(
        s: MarketplaceSales,
        conn_map: &std::collections::HashMap<String, String>,
        org_map: &std::collections::HashMap<String, String>,
        mp_map: &std::collections::HashMap<String, String>,
        product_map: &std::collections::HashMap<String, String>,
    ) -> Self {
        use contracts::domain::common::AggregateId;
        let connection_name = conn_map
            .get(&s.connection_id)
            .cloned()
            .unwrap_or_else(|| "?".to_string());
        let organization_name = org_map
            .get(&s.organization_id)
            .cloned()
            .unwrap_or_else(|| "?".to_string());
        let marketplace_name = mp_map
            .get(&s.marketplace_id)
            .cloned()
            .unwrap_or_else(|| "?".to_string());
        let product_name = product_map
            .get(&s.product_id)
            .cloned()
            .unwrap_or_else(|| "?".to_string());
        Self {
            id: s.base.id.as_string(),
            connection_name,
            organization_name,
            marketplace_name,
            accrual_date: s.accrual_date.format("%Y-%m-%d").to_string(),
            product_name,
            quantity: s.quantity,
            revenue: format!("{:.2}", s.revenue),
            operation_type: s.operation_type,
        }
    }
}

impl Searchable for MarketplaceSalesRow {
    fn matches_filter(&self, filter: &str) -> bool {
        let f = filter.to_lowercase();
        self.connection_name.to_lowercase().contains(&f)
            || self.organization_name.to_lowercase().contains(&f)
            || self.marketplace_name.to_lowercase().contains(&f)
            || self.product_name.to_lowercase().contains(&f)
    }

    fn get_field_value(&self, field: &str) -> Option<String> {
        match field {
            "connection" => Some(self.connection_name.clone()),
            "organization" => Some(self.organization_name.clone()),
            "marketplace" => Some(self.marketplace_name.clone()),
            "accrual_date" => Some(self.accrual_date.clone()),
            "product" => Some(self.product_name.clone()),
            "quantity" => Some(self.quantity.to_string()),
            "revenue" => Some(self.revenue.clone()),
            "operation_type" => Some(self.operation_type.clone()),
            _ => None,
        }
    }
}

impl Sortable for MarketplaceSalesRow {
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
            "accrual_date" => self.accrual_date.cmp(&other.accrual_date),
            "product" => self
                .product_name
                .to_lowercase()
                .cmp(&other.product_name.to_lowercase()),
            "quantity" => self.quantity.cmp(&other.quantity),
            "revenue" => {
                let a = self.revenue.parse::<f64>().unwrap_or(0.0);
                let b = other.revenue.parse::<f64>().unwrap_or(0.0);
                a.partial_cmp(&b).unwrap_or(Ordering::Equal)
            }
            "operation_type" => self
                .operation_type
                .to_lowercase()
                .cmp(&other.operation_type.to_lowercase()),
            _ => Ordering::Equal,
        }
    }
}

#[component]
#[allow(non_snake_case)]
pub fn MarketplaceSalesList() -> impl IntoView {
    use std::collections::HashMap;
    let (items, set_items) = signal::<Vec<MarketplaceSalesRow>>(Vec::new());
    let (sales, set_sales) = signal::<Vec<MarketplaceSales>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);
    let (filter_text, set_filter_text) = signal(String::new());
    let (sort_field, set_sort_field) = signal::<String>("accrual_date".to_string());
    let (sort_ascending, set_sort_ascending) = signal(true);

    let (connections, set_connections) = signal::<Vec<ConnectionMP>>(Vec::new());
    let (organizations, set_organizations) = signal::<Vec<Organization>>(Vec::new());
    let (marketplaces, set_marketplaces) = signal::<Vec<Marketplace>>(Vec::new());
    let (products, set_products) = signal::<Vec<MarketplaceProduct>>(Vec::new());

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
    let product_map = move || -> HashMap<String, String> {
        products
            .get()
            .into_iter()
            .map(|x| {
                use contracts::domain::common::AggregateId;
                (x.base.id.as_string(), x.product_name)
            })
            .collect()
    };

    let compose_rows = move |source: &Vec<MarketplaceSales>| -> Vec<MarketplaceSalesRow> {
        source
            .iter()
            .cloned()
            .map(|s| {
                MarketplaceSalesRow::from_sale(
                    s,
                    &conn_map(),
                    &org_map(),
                    &mp_map(),
                    &product_map(),
                )
            })
            .collect()
    };

    let fetch = move || {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_sales().await {
                Ok(v) => {
                    let rows = compose_rows(&v);
                    set_sales.set(v);
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
            let p = fetch_products().await;
            if let Ok(v) = c {
                set_connections.set(v);
            }
            if let Ok(v) = o {
                set_organizations.set(v);
            }
            if let Ok(v) = m {
                set_marketplaces.set(v);
            }
            if let Ok(v) = p {
                set_products.set(v);
            }
            // Пересобрать строки с учетом загруженных справочников
            let current = sales.get();
            if !current.is_empty() {
                set_items.set(compose_rows(&current));
            }
        });
    };

    let get_filtered_sorted = move || -> Vec<MarketplaceSalesRow> {
        let mut result: Vec<MarketplaceSalesRow> = items
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
                <h2>{"Продажи маркетплейсов"}</h2>
                <div class="header-actions">
                    <SearchInput
                        value=filter_text
                        on_change=Callback::new(move |val: String| set_filter_text.set(val))
                        placeholder="Поиск по продажам...".to_string()
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
                            <th class="cursor-pointer user-select-none" on:click=toggle_sort("connection") title="Сортировать">
                                {move || format!("Подключение{}", get_sort_indicator(&sort_field.get(), "connection", sort_ascending.get()))}
                            </th>
                            <th class="cursor-pointer user-select-none" on:click=toggle_sort("organization") title="Сортировать">
                                {move || format!("Организация{}", get_sort_indicator(&sort_field.get(), "organization", sort_ascending.get()))}
                            </th>
                            <th class="cursor-pointer user-select-none" on:click=toggle_sort("marketplace") title="Сортировать">
                                {move || format!("Маркетплейс{}", get_sort_indicator(&sort_field.get(), "marketplace", sort_ascending.get()))}
                            </th>
                            <th class="cursor-pointer user-select-none" on:click=toggle_sort("accrual_date") title="Сортировать">
                                {move || format!("Дата начисления{}", get_sort_indicator(&sort_field.get(), "accrual_date", sort_ascending.get()))}
                            </th>
                            <th class="cursor-pointer user-select-none" on:click=toggle_sort("product") title="Сортировать">
                                {move || format!("Позиция{}", get_sort_indicator(&sort_field.get(), "product", sort_ascending.get()))}
                            </th>
                            <th class="cursor-pointer user-select-none" on:click=toggle_sort("quantity") title="Сортировать">
                                {move || format!("Количество{}", get_sort_indicator(&sort_field.get(), "quantity", sort_ascending.get()))}
                            </th>
                            <th class="cursor-pointer user-select-none" on:click=toggle_sort("revenue") title="Сортировать">
                                {move || format!("Выручка{}", get_sort_indicator(&sort_field.get(), "revenue", sort_ascending.get()))}
                            </th>
                            <th class="cursor-pointer user-select-none" on:click=toggle_sort("operation_type") title="Сортировать">
                                {move || format!("Тип операции{}", get_sort_indicator(&sort_field.get(), "operation_type", sort_ascending.get()))}
                            </th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            let filtered = get_filtered_sorted();
                            let current_filter = filter_text.get();
                            filtered.into_iter().map(|row| {
                                let conn_view = if current_filter.len() >= 3 { highlight_matches(&row.connection_name, &current_filter) } else { view!{ <span>{row.connection_name.clone()}</span> }.into_any() };
                                let org_view = if current_filter.len() >= 3 { highlight_matches(&row.organization_name, &current_filter) } else { view!{ <span>{row.organization_name.clone()}</span> }.into_any() };
                                let mp_view = if current_filter.len() >= 3 { highlight_matches(&row.marketplace_name, &current_filter) } else { view!{ <span>{row.marketplace_name.clone()}</span> }.into_any() };
                                let product_view = if current_filter.len() >= 3 { highlight_matches(&row.product_name, &current_filter) } else { view!{ <span>{row.product_name.clone()}</span> }.into_any() };
                                view! {
                                    <tr>
                                        <td>{conn_view}</td>
                                        <td>{org_view}</td>
                                        <td>{mp_view}</td>
                                        <td>{row.accrual_date.clone()}</td>
                                        <td>{product_view}</td>
                                        <td>{row.quantity}</td>
                                        <td>{row.revenue.clone()}</td>
                                <td>{row.operation_type.clone()}</td>
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

async fn fetch_sales() -> Result<Vec<MarketplaceSales>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};
    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);
    let url = format!("{}/api/marketplace_sales", api_base());
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
    let data: Vec<MarketplaceSales> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
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

async fn fetch_products() -> Result<Vec<MarketplaceProduct>, String> {
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
