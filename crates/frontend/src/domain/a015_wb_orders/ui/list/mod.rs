use super::details::WbOrdersDetail;
use crate::shared::list_utils::{format_number, format_number_int, get_sort_indicator, Sortable};
use chrono::{Datelike, Utc};
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures;
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub code: String,
    pub description: String,
}

/// Форматирует ISO 8601 дату в dd.mm.yyyy
fn format_date(iso_date: &str) -> String {
    // Парсим ISO 8601: "2025-11-05T16:52:58.585775200Z"
    if let Some(date_part) = iso_date.split('T').next() {
        if let Some((year, rest)) = date_part.split_once('-') {
            if let Some((month, day)) = rest.split_once('-') {
                return format!("{}.{}.{}", day, month, year);
            }
        }
    }
    iso_date.to_string() // fallback
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WbOrdersDto {
    pub id: String,
    pub document_no: String,
    pub order_date: String,
    pub supplier_article: String,
    pub brand: Option<String>,
    pub qty: f64,
    pub finished_price: Option<f64>,
    pub total_price: Option<f64>,
    pub is_cancel: bool,
    pub organization_name: Option<String>,
    pub marketplace_article: Option<String>,
    pub nomenclature_code: Option<String>,
    pub nomenclature_article: Option<String>,
}

impl Sortable for WbOrdersDto {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "document_no" => self
                .document_no
                .to_lowercase()
                .cmp(&other.document_no.to_lowercase()),
            "order_date" => self.order_date.cmp(&other.order_date),
            "supplier_article" => self
                .supplier_article
                .to_lowercase()
                .cmp(&other.supplier_article.to_lowercase()),
            "brand" => match (&self.brand, &other.brand) {
                (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            "qty" => self.qty.partial_cmp(&other.qty).unwrap_or(Ordering::Equal),
            "finished_price" => match (&self.finished_price, &other.finished_price) {
                (Some(a), Some(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            "total_price" => match (&self.total_price, &other.total_price) {
                (Some(a), Some(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            "organization_name" => match (&self.organization_name, &other.organization_name) {
                (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            _ => Ordering::Equal,
        }
    }
}

#[component]
pub fn WbOrdersList() -> impl IntoView {
    let (orders, set_orders) = signal::<Vec<WbOrdersDto>>(Vec::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (selected_id, set_selected_id) = signal::<Option<String>>(None);
    let (show_detail, set_show_detail) = signal(false);

    // Фильтры
    let (date_from, set_date_from) = signal(String::new());
    let (date_to, set_date_to) = signal(String::new());
    let (selected_organization_id, set_selected_organization_id) = signal::<Option<String>>(None);
    let (organizations, set_organizations) = signal::<Vec<Organization>>(Vec::new());

    // Сортировка
    let (sort_field, set_sort_field) = signal::<String>("order_date".to_string());
    let (sort_ascending, set_sort_ascending) = signal(false); // По умолчанию - новые сначала

    // Поиск (пагинация убрана - показываем весь список)
    let (search_query, set_search_query) = signal(String::new());

    // Загрузка организаций
    let load_organizations = move || {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_organizations().await {
                Ok(orgs) => {
                    set_organizations.set(orgs);
                }
                Err(e) => {
                    log!("Failed to load organizations: {}", e);
                }
            }
        });
    };

    // Загрузка данных
    let load_data = move || {
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let mut url = "/api/a015/wb-orders?limit=20000".to_string();

            if !date_from.get().is_empty() {
                url.push_str(&format!("&date_from={}", date_from.get()));
            }
            if !date_to.get().is_empty() {
                url.push_str(&format!("&date_to={}", date_to.get()));
            }
            if let Some(org_id) = selected_organization_id.get() {
                url.push_str(&format!("&organization_id={}", org_id));
            }

            match Request::get(&url).send().await {
                Ok(response) => {
                    if response.ok() {
                        match response.json::<Vec<serde_json::Value>>().await {
                            Ok(data) => {
                                log!("Received {} items from backend", data.len());

                                let parsed_orders: Vec<WbOrdersDto> = data
                                    .into_iter()
                                    .filter_map(|item| {
                                        // Backend использует #[serde(flatten)], поэтому поля на верхнем уровне
                                        // WbOrders также имеет flatten на base, поэтому id напрямую, а не в base.id
                                        let state = item.get("state")?;
                                        let line = item.get("line")?;
                                        let header = item.get("header")?;

                                        // id находится на верхнем уровне из-за flatten на base
                                        let id = item.get("id")?.as_str()?.to_string();

                                        let document_no =
                                            header.get("document_no")?.as_str()?.to_string();

                                        let order_date =
                                            state.get("order_dt")?.as_str()?.to_string();

                                        let supplier_article =
                                            line.get("supplier_article")?.as_str()?.to_string();

                                        let brand = line
                                            .get("brand")
                                            .and_then(|v| v.as_str())
                                            .map(String::from);

                                        let qty = line.get("qty")?.as_f64()?;

                                        let finished_price =
                                            line.get("finished_price").and_then(|v| v.as_f64());

                                        let total_price =
                                            line.get("total_price").and_then(|v| v.as_f64());

                                        let is_cancel =
                                            state.get("is_cancel")?.as_bool().unwrap_or(false);

                                        let organization_name = item
                                            .get("organization_name")
                                            .and_then(|v| v.as_str())
                                            .map(String::from);

                                        let marketplace_article = item
                                            .get("marketplace_article")
                                            .and_then(|v| v.as_str())
                                            .map(String::from);

                                        let nomenclature_code = item
                                            .get("nomenclature_code")
                                            .and_then(|v| v.as_str())
                                            .map(String::from);

                                        let nomenclature_article = item
                                            .get("nomenclature_article")
                                            .and_then(|v| v.as_str())
                                            .map(String::from);

                                        Some(WbOrdersDto {
                                            id,
                                            document_no,
                                            order_date,
                                            supplier_article,
                                            brand,
                                            qty,
                                            finished_price,
                                            total_price,
                                            is_cancel,
                                            organization_name,
                                            marketplace_article,
                                            nomenclature_code,
                                            nomenclature_article,
                                        })
                                    })
                                    .collect();

                                log!("Successfully parsed {} orders", parsed_orders.len());
                                set_orders.set(parsed_orders);
                            }
                            Err(e) => {
                                log!("Failed to parse response: {}", e);
                                set_error.set(Some(format!("Failed to parse response: {}", e)));
                            }
                        }
                    } else {
                        log!("Server error: {}", response.status());
                        set_error.set(Some(format!("Server error: {}", response.status())));
                    }
                }
                Err(e) => {
                    log!("Network error: {}", e);
                    set_error.set(Some(format!("Network error: {}", e)));
                }
            }

            set_loading.set(false);
        });
    };

    // Инициализация: устанавливаем даты по умолчанию
    Effect::new(move |_| {
        let today = Utc::now();
        let first_day_of_month = format!("{}-{:02}-01", today.year(), today.month());
        let last_day = format!("{}-{:02}-{:02}", today.year(), today.month(), today.day());

        set_date_from.set(first_day_of_month);
        set_date_to.set(last_day);

        load_organizations();
        load_data();
    });

    // Обработчик сортировки
    let handle_sort = move |field: &'static str| {
        if field.to_string() == sort_field.get() {
            set_sort_ascending.update(|asc| *asc = !*asc);
        } else {
            set_sort_field.set(field.to_string());
            set_sort_ascending.set(true);
        }
    };

    // Фильтрация, сортировка и пагинация
    let filtered_and_sorted_orders = Memo::new(move |_| {
        let mut result = orders.get();
        let query = search_query.get().to_lowercase();

        // Фильтрация по поиску
        if !query.is_empty() {
            result.retain(|order| {
                order.supplier_article.to_lowercase().contains(&query)
                    || order.document_no.to_lowercase().contains(&query)
                    || order
                        .brand
                        .as_ref()
                        .map_or(false, |b| b.to_lowercase().contains(&query))
            });
        }

        // Сортировка
        let field = sort_field.get();
        let asc = sort_ascending.get();
        result.sort_by(|a, b| {
            let ord = a.compare_by_field(b, &field);
            if asc {
                ord
            } else {
                ord.reverse()
            }
        });

        result
    });

    // Пагинация убрана - показываем весь отфильтрованный список

    // Функция для вычисления итогов
    let totals = move || {
        let data = filtered_and_sorted_orders.get();
        let total_qty: f64 = data.iter().map(|o| o.qty).sum();
        let total_finished: f64 = data.iter().filter_map(|o| o.finished_price).sum();
        let total_price: f64 = data.iter().filter_map(|o| o.total_price).sum();
        (data.len(), total_qty, total_finished, total_price)
    };

    view! {
        <div class="container-fluid">

            {move || {
                if show_detail.get() {
                    if let Some(id) = selected_id.get() {
                        view! {
                            <WbOrdersDetail id=id on_close=move || {
                                set_show_detail.set(false);
                                set_selected_id.set(None);
                            } />
                        }
                            .into_any()
                    } else {
                        view! { <div /> }.into_any()
                    }
                } else {
                    view! {
                        <div>
                            <div style="display: flex; align-items: center; gap: 12px; margin-bottom: 8px; flex-wrap: wrap;">
                                <h2 style="margin: 0; font-size: var(--font-size-h3); line-height: 1.2;">"Wildberries Заказы (A015)"</h2>

                                <label style="margin: 0; font-size: var(--font-size-sm); white-space: nowrap;">"От:"</label>
                                <input
                                    type="date"
                                    prop:value=date_from
                                    on:input=move |ev| {
                                        set_date_from.set(event_target_value(&ev));
                                    }
                                    style="padding: 4px 8px; border: 1px solid #ddd; border-radius: 4px; font-size: var(--font-size-sm);"
                                />

                                <label style="margin: 0; font-size: var(--font-size-sm); white-space: nowrap;">"До:"</label>
                                <input
                                    type="date"
                                    prop:value=date_to
                                    on:input=move |ev| {
                                        set_date_to.set(event_target_value(&ev));
                                    }
                                    style="padding: 4px 8px; border: 1px solid #ddd; border-radius: 4px; font-size: var(--font-size-sm);"
                                />

                                <label style="margin: 0; font-size: var(--font-size-sm); white-space: nowrap;">"Организация:"</label>
                                <select
                                    on:change=move |ev| {
                                        let value = event_target_value(&ev);
                                        if value.is_empty() {
                                            set_selected_organization_id.set(None);
                                        } else {
                                            set_selected_organization_id.set(Some(value));
                                        }
                                    }
                                    style="padding: 4px 8px; border: 1px solid #ddd; border-radius: 4px; font-size: var(--font-size-sm); min-width: 200px;"
                                >
                                    <option value="">"Все организации"</option>
                                    {move || organizations.get().into_iter().map(|org| {
                                        let org_id = org.id.clone();
                                        let org_desc = org.description.clone();
                                        view! {
                                            <option value=org_id.clone() selected=move || {
                                                selected_organization_id.get().as_ref() == Some(&org_id)
                                            }>
                                                {org_desc}
                                            </option>
                                        }
                                    }).collect_view()}
                                </select>

                                <button
                                    on:click=move |_| {
                                        load_data();
                                    }
                                    style="padding: 4px 12px; background: #4CAF50; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: var(--font-size-sm); transition: all 0.2s;"
                                    onmouseenter="this.style.opacity='0.85'; this.style.transform='translateY(-2px)'"
                                    onmouseleave="this.style.opacity='1'; this.style.transform='translateY(0)'"
                                    disabled=move || loading.get()
                                >
                                    {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                                </button>

                                <button
                                    on:click=move |_| {
                                        let data = filtered_and_sorted_orders.get();
                                        if let Err(e) = export_to_csv(&data) {
                                            log!("Failed to export: {}", e);
                                        }
                                    }
                                    style="padding: 4px 12px; background: #2196F3; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: var(--font-size-sm); transition: all 0.2s;"
                                    onmouseenter="this.style.opacity='0.85'; this.style.transform='translateY(-2px)'"
                                    onmouseleave="this.style.opacity='1'; this.style.transform='translateY(0)'"
                                    disabled=move || loading.get() || filtered_and_sorted_orders.get().is_empty()
                                >
                                    "Экспорт в Excel"
                                </button>

                                <div style="margin-bottom: 8px;">
                                    {move || {
                                        let (count, total_qty, total_finished, total_price) = totals();
                                        let limit_warning = if !loading.get() && count >= 20000 {
                                            view! {
                                                <span style="margin-left: 8px; padding: 6px 12px; background: #fff3cd; color: #856404; border-radius: 4px; font-size: var(--font-size-sm);">
                                                    "⚠️ Показаны первые 20000 записей. Уточните период для полной загрузки."
                                                </span>
                                            }.into_any()
                                        } else {
                                            view! { <></> }.into_any()
                                        };
                                        view! {
                                            <>
                                                <span style="font-size: var(--font-size-base); font-weight: 600; color: var(--color-text); background: var(--color-background-alt, #f5f5f5); padding: 6px 12px; border-radius: 4px;">
                                                "Total: " {format_number_int(count as f64)} " records | "
                                                "Кол-во: " {format_number_int(total_qty)} " | "
                                                "Итоговая цена: " {format_number(total_finished)} " | "
                                                "Полная цена: " {format_number(total_price)}
                                                </span>
                                                {limit_warning}
                                            </>
                                        }
                                    }}
                                </div>
                            </div>

                            <div class="row mb-3">
                                <div class="col-md-6">
                                    <input
                                        type="text"
                                        class="form-control"
                                        placeholder="Поиск по артикулу, номеру заказа, бренду..."
                                        value=move || search_query.get()
                                        on:input=move |ev| {
                                            set_search_query.set(event_target_value(&ev));
                                        }
                                    />
                                </div>
                            </div>

                            {move || {
                                error
                                    .get()
                                    .map(|err| {
                                        view! {
                                            <div class="alert alert-danger" role="alert">
                                                {err}
                                            </div>
                                        }
                                    })
                            }}

                            <div class="table-container">
                                <table class="table__data" style="width: 100%; border-collapse: collapse;">
                                    <thead>
                                        <tr style="background: #f5f5f5;">
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=move |_| handle_sort("document_no")
                                                title="Сортировать"
                                            >
                                                {move || format!("Номер заказа{}", get_sort_indicator(&sort_field.get(), "document_no", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=move |_| handle_sort("order_date")
                                                title="Сортировать"
                                            >
                                                {move || format!("Дата заказа{}", get_sort_indicator(&sort_field.get(), "order_date", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=move |_| handle_sort("organization_name")
                                                title="Сортировать"
                                            >
                                                {move || format!("Организация{}", get_sort_indicator(&sort_field.get(), "organization_name", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=move |_| handle_sort("supplier_article")
                                                title="Сортировать"
                                            >
                                                {move || format!("Артикул{}", get_sort_indicator(&sort_field.get(), "supplier_article", sort_ascending.get()))}
                                            </th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Артикул МП"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Артикул 1С"</th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Код 1С"</th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=move |_| handle_sort("brand")
                                                title="Сортировать"
                                            >
                                                {move || format!("Бренд{}", get_sort_indicator(&sort_field.get(), "brand", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; text-align: right; cursor: pointer; user-select: none;"
                                                on:click=move |_| handle_sort("qty")
                                                title="Сортировать"
                                            >
                                                {move || format!("Кол-во{}", get_sort_indicator(&sort_field.get(), "qty", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; text-align: right; cursor: pointer; user-select: none;"
                                                on:click=move |_| handle_sort("finished_price")
                                                title="Сортировать"
                                            >
                                                {move || format!("Итоговая цена{}", get_sort_indicator(&sort_field.get(), "finished_price", sort_ascending.get()))}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; text-align: right; cursor: pointer; user-select: none;"
                                                on:click=move |_| handle_sort("total_price")
                                                title="Сортировать"
                                            >
                                                {move || format!("Полная цена{}", get_sort_indicator(&sort_field.get(), "total_price", sort_ascending.get()))}
                                            </th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Отменён"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {move || {
                                            filtered_and_sorted_orders
                                                .get()
                                                .into_iter()
                                                .map(|order| {
                                                    let order_id = order.id.clone();
                                                    let is_cancelled = order.is_cancel;
                                                    let formatted_date = format_date(&order.order_date);
                                                    let formatted_qty = format!("{:.0}", order.qty);
                                                    let formatted_finished = order.finished_price
                                                        .map(|p| format!("{:.2}", p))
                                                        .unwrap_or_else(|| "-".to_string());
                                                    let formatted_total = order.total_price
                                                        .map(|p| format!("{:.2}", p))
                                                        .unwrap_or_else(|| "-".to_string());

                                                    view! {
                                                        <tr
                                                            on:click=move |_| {
                                                                set_selected_id.set(Some(order_id.clone()));
                                                                set_show_detail.set(true);
                                                            }
                                                            style=move || {
                                                                if is_cancelled {
                                                                    "cursor: pointer; transition: background 0.2s; background-color: #ffdddd;"
                                                                } else {
                                                                    "cursor: pointer; transition: background 0.2s;"
                                                                }
                                                            }
                                                        >
                                                            <td style="border: 1px solid #ddd; padding: 8px;">{order.document_no}</td>
                                                            <td style="border: 1px solid #ddd; padding: 8px;">{formatted_date}</td>
                                                            <td style="border: 1px solid #ddd; padding: 8px;">{order.organization_name.clone().unwrap_or_else(|| "—".to_string())}</td>
                                                            <td style="border: 1px solid #ddd; padding: 8px;">{order.supplier_article}</td>
                                                            <td style="border: 1px solid #ddd; padding: 8px;"><span style="color: #1976d2; font-weight: 600;">{order.marketplace_article.clone().unwrap_or_else(|| "—".to_string())}</span></td>
                                                            <td style="border: 1px solid #ddd; padding: 8px;"><span style="color: #2e7d32; font-weight: 600;">{order.nomenclature_article.clone().unwrap_or_else(|| "—".to_string())}</span></td>
                                                            <td style="border: 1px solid #ddd; padding: 8px;"><span style="color: #2e7d32; font-weight: 600;">{order.nomenclature_code.clone().unwrap_or_else(|| "—".to_string())}</span></td>
                                                            <td style="border: 1px solid #ddd; padding: 8px;">{order.brand.clone().unwrap_or_else(|| "—".to_string())}</td>
                                                            <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{formatted_qty}</td>
                                                            <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{formatted_finished}</td>
                                                            <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{formatted_total}</td>
                                                            <td style="border: 1px solid #ddd; padding: 8px;">
                                                                {if order.is_cancel {
                                                                    "Да"
                                                                } else {
                                                                    "Нет"
                                                                }}
                                                            </td>
                                                        </tr>
                                                    }
                                                })
                                                .collect_view()
                                        }}
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    }
                        .into_any()
                }
            }}

        </div>
    }
}

/// Загрузка списка организаций
async fn fetch_organizations() -> Result<Vec<Organization>, String> {
    use web_sys::{Request as WebRequest, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = "http://localhost:3000/api/organization";
    let request = WebRequest::new_with_str_and_init(url, &opts).map_err(|e| format!("{e:?}"))?;
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

/// Экспорт WB Orders в CSV для Excel
fn export_to_csv(data: &[WbOrdersDto]) -> Result<(), String> {
    // UTF-8 BOM для правильного отображения кириллицы в Excel
    let mut csv = String::from("\u{FEFF}");

    // Заголовок с точкой с запятой как разделитель
    csv.push_str("Номер заказа;Дата заказа;Организация;Артикул продавца;Артикул МП;Артикул 1С;Код 1С;Бренд;Количество;Итоговая цена;Цена без скидки;Отменён\n");

    for order in data {
        let order_date = format_date(&order.order_date);
        let org_name = order
            .organization_name
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("—");
        let mp_article = order
            .marketplace_article
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("—");
        let nom_code = order
            .nomenclature_code
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("—");
        let nom_article = order
            .nomenclature_article
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("—");
        let brand = order.brand.as_ref().map(|s| s.as_str()).unwrap_or("—");

        // Форматируем суммы с запятой как десятичный разделитель
        let qty_str = format!("{:.0}", order.qty);
        let finished_price_str = order
            .finished_price
            .map(|a| format!("{:.2}", a).replace(".", ","))
            .unwrap_or_else(|| "—".to_string());
        let total_price_str = order
            .total_price
            .map(|a| format!("{:.2}", a).replace(".", ","))
            .unwrap_or_else(|| "—".to_string());
        let is_cancel_str = if order.is_cancel { "Да" } else { "Нет" };

        csv.push_str(&format!(
            "\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";{};{};{};\"{}\"\n",
            order.document_no.replace('\"', "\"\""),
            order_date,
            org_name.replace('\"', "\"\""),
            order.supplier_article.replace('\"', "\"\""),
            mp_article.replace('\"', "\"\""),
            nom_article.replace('\"', "\"\""),
            nom_code.replace('\"', "\"\""),
            brand.replace('\"', "\"\""),
            qty_str,
            finished_price_str,
            total_price_str,
            is_cancel_str
        ));
    }

    // Создаем Blob с CSV данными
    let blob_parts = js_sys::Array::new();
    blob_parts.push(&wasm_bindgen::JsValue::from_str(&csv));

    let blob_props = BlobPropertyBag::new();
    blob_props.set_type("text/csv;charset=utf-8;");

    let blob = Blob::new_with_str_sequence_and_options(&blob_parts, &blob_props)
        .map_err(|e| format!("Failed to create blob: {:?}", e))?;

    // Создаем URL для blob
    let url = Url::create_object_url_with_blob(&blob)
        .map_err(|e| format!("Failed to create URL: {:?}", e))?;

    // Создаем временную ссылку для скачивания
    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let document = window.document().ok_or_else(|| "no document".to_string())?;

    let a = document
        .create_element("a")
        .map_err(|e| format!("Failed to create element: {:?}", e))?
        .dyn_into::<HtmlAnchorElement>()
        .map_err(|e| format!("Failed to cast to anchor: {:?}", e))?;

    a.set_href(&url);
    let filename = format!(
        "wb_orders_{}.csv",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    a.set_download(&filename);
    a.click();

    // Освобождаем URL
    Url::revoke_object_url(&url).map_err(|e| format!("Failed to revoke URL: {:?}", e))?;

    Ok(())
}
