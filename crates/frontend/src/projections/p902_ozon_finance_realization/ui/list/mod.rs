use chrono::Utc;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use crate::domain::a010_ozon_fbs_posting::ui::details::OzonFbsPostingDetail;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFinanceRealizationDto {
    pub posting_number: String,
    pub sku: String,
    pub document_type: String,
    pub registrator_ref: String,
    pub connection_mp_ref: String,
    pub organization_ref: String,
    pub posting_ref: Option<String>,
    pub accrual_date: String,
    pub sale_date: Option<String>,
    pub operation_date: Option<String>,
    pub delivery_date: Option<String>,
    pub delivery_schema: Option<String>,
    pub delivery_region: Option<String>,
    pub delivery_city: Option<String>,
    pub quantity: f64,
    pub price: Option<f64>,
    pub amount: f64,
    pub commission_amount: Option<f64>,
    pub commission_percent: Option<f64>,
    pub services_amount: Option<f64>,
    pub payout_amount: Option<f64>,
    pub operation_type: String,
    pub operation_type_name: Option<String>,
    pub is_return: bool,
    pub currency_code: Option<String>,
    pub loaded_at_utc: String,
    pub payload_version: i32,
    pub extra: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFinanceRealizationListResponse {
    pub items: Vec<OzonFinanceRealizationDto>,
    pub total_count: i32,
    pub has_more: bool,
}

#[derive(Debug, Clone)]
struct SelectedPosting {
    posting_id: String,
}

#[component]
pub fn OzonFinanceRealizationList() -> impl IntoView {
    let (data, set_data) = signal(Vec::<OzonFinanceRealizationDto>::new());
    let (total_count, set_total_count) = signal(0);
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(None::<String>);
    let (selected_posting, set_selected_posting) = signal::<Option<SelectedPosting>>(None);

    // Фильтры - период по умолчанию: широкий диапазон для загрузки всех данных
    let now = Utc::now().date_naive();

    // Начальная дата: год назад от текущей даты
    let default_start = now - chrono::Duration::days(365);
    // Конечная дата: текущая дата
    let default_end = now;

    let (date_from, set_date_from) = signal(default_start.format("%Y-%m-%d").to_string());
    let (date_to, set_date_to) = signal(default_end.format("%Y-%m-%d").to_string());
    let (posting_number_filter, set_posting_number_filter) = signal("".to_string());
    let (sku_filter, set_sku_filter) = signal("".to_string());
    let (operation_type_filter, set_operation_type_filter) = signal("".to_string());
    let (sort_by, set_sort_by) = signal("accrual_date".to_string());
    let (sort_desc, set_sort_desc) = signal(true);

    let load_data = move || {
        set_loading.set(true);
        set_error.set(None);

        let date_from_val = date_from.get();
        let date_to_val = date_to.get();
        let posting_number_val = posting_number_filter.get();
        let sku_val = sku_filter.get();
        let operation_type_val = operation_type_filter.get();
        let sort_by_val = sort_by.get();
        let sort_desc_val = sort_desc.get();

        let mut query_params = format!(
            "?date_from={}&date_to={}&sort_by={}&sort_desc={}&limit=10000&offset=0",
            date_from_val, date_to_val, sort_by_val, sort_desc_val
        );

        if !posting_number_val.is_empty() {
            query_params.push_str(&format!("&posting_number={}", posting_number_val));
        }
        if !sku_val.is_empty() {
            query_params.push_str(&format!("&sku={}", sku_val));
        }
        if !operation_type_val.is_empty() {
            query_params.push_str(&format!("&operation_type={}", operation_type_val));
        }

        spawn_local(async move {
            match fetch_finance_realization(&query_params).await {
                Ok(response) => {
                    set_total_count.set(response.total_count);
                    set_data.set(response.items);
                    set_loading.set(false);
                }
                Err(e) => {
                    log!("Failed to fetch finance realization: {:?}", e);
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    };

    let handle_posting_click = move |posting_id: String| {
        set_selected_posting.set(Some(SelectedPosting { posting_id }));
    };

    let close_details = move || {
        set_selected_posting.set(None);
    };

    // Функция для сортировки по колонке
    let handle_column_sort = move |column: &str| {
        let current_sort = sort_by.get();
        if current_sort == column {
            // Переключаем направление сортировки
            set_sort_desc.set(!sort_desc.get());
        } else {
            // Новая колонка - сортируем по убыванию
            set_sort_by.set(column.to_string());
            set_sort_desc.set(true);
        }
        load_data();
    };

    // Экспорт в Excel (по аналогии с a014)
    let export_to_excel = move || {
        let items = data.get();
        if items.is_empty() {
            log!("No data to export");
            return;
        }

        // UTF-8 BOM для правильного отображения кириллицы в Excel
        let mut csv = String::from("\u{FEFF}");

        // Заголовок с точкой с запятой как разделитель
        csv.push_str(
            "Date;Sale Date;Posting;SKU;Qty;Amount;Commission;Payout;Price;Type;Loaded At\n",
        );

        for item in items {
            let sale_date = item.sale_date.as_ref().map(|s| s.as_str()).unwrap_or("-");

            // Форматируем числа с запятой как десятичный разделитель
            let qty_str = format!("{:.2}", item.quantity).replace(".", ",");
            let amount_str = format!("{:.2}", item.amount).replace(".", ",");
            let commission_str = item
                .commission_amount
                .map(|c| format!("{:.2}", c).replace(".", ","))
                .unwrap_or_else(|| "-".to_string());
            let payout_str = item
                .payout_amount
                .map(|p| format!("{:.2}", p).replace(".", ","))
                .unwrap_or_else(|| "-".to_string());
            let price_str = item
                .price
                .map(|p| format!("{:.2}", p).replace(".", ","))
                .unwrap_or_else(|| "-".to_string());

            csv.push_str(&format!(
                "\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";{};{};{};{};\"{}\";\"{}\"\n",
                item.accrual_date,
                sale_date,
                item.posting_number.replace('\"', "\"\""),
                item.sku.replace('\"', "\"\""),
                qty_str,
                amount_str,
                commission_str,
                payout_str,
                price_str,
                item.operation_type.replace('\"', "\"\""),
                item.loaded_at_utc.replace('\"', "\"\"")
            ));
        }

        // Создаем Blob с CSV данными
        use js_sys::Array;
        use wasm_bindgen::JsValue;

        let array = Array::new();
        array.push(&JsValue::from_str(&csv));

        let blob_props = web_sys::BlobPropertyBag::new();
        blob_props.set_type("text/csv;charset=utf-8;");

        if let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&array, &blob_props) {
            if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                if let Some(window) = web_sys::window() {
                    if let Some(document) = window.document() {
                        if let Ok(a) = document.create_element("a") {
                            let a: web_sys::HtmlAnchorElement = a.unchecked_into();
                            a.set_href(&url);
                            let filename = format!(
                                "ozon_finance_realization_{}.csv",
                                chrono::Utc::now().format("%Y%m%d_%H%M%S")
                            );
                            a.set_download(&filename);
                            a.click();
                            let _ = web_sys::Url::revoke_object_url(&url);
                        }
                    }
                }
            }
        }
    };

    view! {
        <div id="p902_ozon_finance_realization--list" data-page-category="legacy" class="finance-realization-list">
            <div style="display: flex; align-items: center; gap: 12px; margin-bottom: 12px;">
                <h2 style="margin: 0; font-size: var(--font-size-h3);">
                    "OZON Finance Realization (P902)"
                    {move || {
                        let count = total_count.get();
                        if count > 0 {
                            format!(" - {} records", count)
                        } else {
                            String::new()
                        }
                    }}
                </h2>

                <label style="margin: 0; font-size: var(--font-size-sm);">"From:"</label>
                <input
                    type="date"
                    prop:value=move || date_from.get()
                    on:input=move |ev| {
                        set_date_from.set(event_target_value(&ev));
                    }
                    style="padding: 4px 6px; border-radius: 4px; border: 1px solid var(--border-color);"
                />

                <label style="margin: 0; font-size: var(--font-size-sm);">"To:"</label>
                <input
                    type="date"
                    prop:value=move || date_to.get()
                    on:input=move |ev| {
                        set_date_to.set(event_target_value(&ev));
                    }
                    style="padding: 4px 6px; border-radius: 4px; border: 1px solid var(--border-color);"
                />

                <button
                    on:click=move |_| load_data()
                    style="padding: 4px 12px; background: #28a745; color: white; border: none; border-radius: 4px; cursor: pointer;"
                    title="Обновить данные"
                >
                    "Обновить"
                </button>

                <button
                    on:click=move |_| export_to_excel()
                    style="padding: 4px 12px; background: #28a745; color: white; border: none; border-radius: 4px; cursor: pointer;"
                    title="Export to Excel (CSV)"
                >
                    "Export Excel"
                </button>
            </div>

            <div style="display: flex; gap: 12px; margin-bottom: 12px;">
                <input
                    type="text"
                    placeholder="Posting Number..."
                    prop:value=move || posting_number_filter.get()
                    on:input=move |ev| {
                        set_posting_number_filter.set(event_target_value(&ev));
                    }
                    style="padding: 4px 8px; border: 1px solid var(--border-color); border-radius: 4px; flex: 1;"
                />
                <input
                    type="text"
                    placeholder="SKU..."
                    prop:value=move || sku_filter.get()
                    on:input=move |ev| {
                        set_sku_filter.set(event_target_value(&ev));
                    }
                    style="padding: 4px 8px; border: 1px solid var(--border-color); border-radius: 4px; flex: 1;"
                />
                <input
                    type="text"
                    placeholder="Operation Type..."
                    prop:value=move || operation_type_filter.get()
                    on:input=move |ev| {
                        set_operation_type_filter.set(event_target_value(&ev));
                    }
                    style="padding: 4px 8px; border: 1px solid var(--border-color); border-radius: 4px; flex: 1;"
                />

                <select
                    on:change=move |ev| {
                        set_sort_by.set(event_target_value(&ev));
                    }
                    style="padding: 4px 8px; border: 1px solid var(--border-color); border-radius: 4px;"
                >
                    <option value="accrual_date">"Sort by Date"</option>
                    <option value="sale_date">"Sort by Sale Date"</option>
                    <option value="posting_number">"Sort by Posting"</option>
                    <option value="sku">"Sort by SKU"</option>
                    <option value="amount">"Sort by Amount"</option>
                </select>

                <button
                    on:click=move |_| set_sort_desc.set(!sort_desc.get())
                    style="padding: 4px 8px; border: 1px solid var(--border-color); border-radius: 4px; background: white; cursor: pointer;"
                >
                    {move || if sort_desc.get() { "↓ Desc" } else { "↑ Asc" }}
                </button>
            </div>

            {move || {
                if loading.get() {
                    view! { <p>"Loading..."</p> }.into_any()
                } else if let Some(err) = error.get() {
                    view! { <p style="color: red;">"Error: " {err}</p> }.into_any()
                } else {
                    let items = data.get();
                    if items.is_empty() {
                        view! { <p>"No data found"</p> }.into_any()
                    } else {
                        let current_sort = sort_by.get();
                        let is_desc = sort_desc.get();

                        // Helper для отображения индикатора сортировки
                        let sort_indicator = move |col: &str| {
                            if current_sort == col {
                                if is_desc { " ↓" } else { " ↑" }
                            } else {
                                ""
                            }
                        };

                        // Расчет итогов
                        let total_qty: f64 = items.iter().map(|item| item.quantity).sum();
                        let total_amount: f64 = items.iter().map(|item| item.amount).sum();
                        let total_payout: f64 = items.iter().map(|item| item.payout_amount.unwrap_or(0.0)).sum();
                        let total_price: f64 = items.iter().map(|item| item.price.unwrap_or(0.0)).sum();

                        view! {
                            <div style="padding: 8px 12px; margin-bottom: 8px; background: var(--secondary-bg-color); border: 1px solid var(--border-color); border-radius: 4px; font-weight: bold; display: flex; gap: 24px;">
                                <span>"ИТОГО:"</span>
                                <span>"Qty: " {format!("{:.2}", total_qty)}</span>
                                <span>"Amount: " {format!("{:.2}", total_amount)}</span>
                                <span>"Payout: " {format!("{:.2}", total_payout)}</span>
                                <span>"Price: " {format!("{:.2}", total_price)}</span>
                            </div>

                            <div style="max-height: calc(100vh - 300px); overflow-y: auto; border: 1px solid var(--border-color); border-radius: 4px;">
                                <table style="width: 100%; border-collapse: collapse; font-size: var(--font-size-sm);">
                                    <thead style="position: sticky; top: 0; z-index: 10; background: var(--secondary-bg-color);">
                                        <tr style="border-bottom: 2px solid var(--border-color);">
                                            <th
                                                on:click=move |_| handle_column_sort("accrual_date")
                                                style="padding: 8px; text-align: left; cursor: pointer; user-select: none; background: var(--secondary-bg-color);"
                                            >
                                                "Date" {sort_indicator("accrual_date")}
                                            </th>
                                            <th
                                                on:click=move |_| handle_column_sort("sale_date")
                                                style="padding: 8px; text-align: left; cursor: pointer; user-select: none; background: var(--secondary-bg-color);"
                                            >
                                                "Дата продажи" {sort_indicator("sale_date")}
                                            </th>
                                            <th
                                                on:click=move |_| handle_column_sort("posting_number")
                                                style="padding: 8px; text-align: left; cursor: pointer; user-select: none; background: var(--secondary-bg-color);"
                                            >
                                                "Posting" {sort_indicator("posting_number")}
                                            </th>
                                            <th
                                                on:click=move |_| handle_column_sort("sku")
                                                style="padding: 8px; text-align: left; cursor: pointer; user-select: none; background: var(--secondary-bg-color);"
                                            >
                                                "SKU" {sort_indicator("sku")}
                                            </th>
                                            <th
                                                on:click=move |_| handle_column_sort("quantity")
                                                style="padding: 8px; text-align: right; cursor: pointer; user-select: none; background: var(--secondary-bg-color);"
                                            >
                                                "Qty" {sort_indicator("quantity")}
                                            </th>
                                            <th
                                                on:click=move |_| handle_column_sort("amount")
                                                style="padding: 8px; text-align: right; cursor: pointer; user-select: none; background: var(--secondary-bg-color);"
                                            >
                                                "Amount" {sort_indicator("amount")}
                                            </th>
                                            <th
                                                on:click=move |_| handle_column_sort("commission_amount")
                                                style="padding: 8px; text-align: right; cursor: pointer; user-select: none; background: var(--secondary-bg-color);"
                                            >
                                                "Commission" {sort_indicator("commission_amount")}
                                            </th>
                                            <th
                                                on:click=move |_| handle_column_sort("payout_amount")
                                                style="padding: 8px; text-align: right; cursor: pointer; user-select: none; background: var(--secondary-bg-color);"
                                            >
                                                "Payout" {sort_indicator("payout_amount")}
                                            </th>
                                            <th
                                                on:click=move |_| handle_column_sort("price")
                                                style="padding: 8px; text-align: right; cursor: pointer; user-select: none; background: var(--secondary-bg-color);"
                                            >
                                                "Price" {sort_indicator("price")}
                                            </th>
                                            <th
                                                on:click=move |_| handle_column_sort("operation_type")
                                                style="padding: 8px; text-align: left; cursor: pointer; user-select: none; background: var(--secondary-bg-color);"
                                            >
                                                "Type" {sort_indicator("operation_type")}
                                            </th>
                                            <th
                                                on:click=move |_| handle_column_sort("loaded_at_utc")
                                                style="padding: 8px; text-align: left; cursor: pointer; user-select: none; background: var(--secondary-bg-color);"
                                            >
                                                "Loaded At" {sort_indicator("loaded_at_utc")}
                                            </th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                    <For
                                        each=move || items.clone()
                                        key=|item| (item.posting_number.clone(), item.sku.clone(), item.operation_type.clone())
                                        children=move |item: OzonFinanceRealizationDto| {
                                            let posting_ref = item.posting_ref.clone();
                                            let posting_number = item.posting_number.clone();
                                            let sale_date_display = item.sale_date.clone().unwrap_or_else(|| "-".to_string());
                                            let is_return = item.is_return;
                                            let row_style = if is_return {
                                                "border-bottom: 1px solid var(--border-color); background: #fff3cd; hover:background: #ffe69c;"
                                            } else {
                                                "border-bottom: 1px solid var(--border-color); hover:background: var(--hover-bg-color);"
                                            };

                                            view! {
                                                <tr style={row_style}>
                                                    <td style="padding: 6px 8px;">{item.accrual_date}</td>
                                                    <td style="padding: 6px 8px; color: #28a745; font-weight: 500;">
                                                        {sale_date_display}
                                                    </td>
                                                    <td style="padding: 6px 8px;">
                                                        {if let Some(ref_id) = posting_ref {
                                                            let ref_id_clone = ref_id.clone();
                                                            view! {
                                                                <a
                                                                    href="#"
                                                                    on:click=move |ev| {
                                                                        ev.prevent_default();
                                                                        handle_posting_click(ref_id_clone.clone());
                                                                    }
                                                                    style="color: var(--primary-color); text-decoration: underline; cursor: pointer;"
                                                                >
                                                                    {posting_number.clone()}
                                                                </a>
                                                            }.into_any()
                                                        } else {
                                                            view! { <span>{posting_number.clone()}</span> }.into_any()
                                                        }}
                                                    </td>
                                                    <td style="padding: 6px 8px;">{item.sku}</td>
                                                    <td style="padding: 6px 8px; text-align: right; color: {if is_return { \"#dc3545\" } else { \"inherit\" }}; font-weight: {if is_return { \"600\" } else { \"normal\" }};">
                                                        {format!("{:.2}", item.quantity)}
                                                    </td>
                                                    <td style="padding: 6px 8px; text-align: right; color: {if is_return { \"#dc3545\" } else { \"inherit\" }}; font-weight: {if is_return { \"600\" } else { \"normal\" }};">
                                                        {format!("{:.2}", item.amount)}
                                                    </td>
                                                    <td style="padding: 6px 8px; text-align: right;">
                                                        {item.commission_amount.map(|c| format!("{:.2}", c)).unwrap_or_else(|| "-".to_string())}
                                                    </td>
                                                    <td style="padding: 6px 8px; text-align: right;">
                                                        {item.payout_amount.map(|p| format!("{:.2}", p)).unwrap_or_else(|| "-".to_string())}
                                                    </td>
                                                    <td style="padding: 6px 8px; text-align: right;">
                                                        {item.price.map(|p| format!("{:.2}", p)).unwrap_or_else(|| "-".to_string())}
                                                    </td>
                                                    <td style="padding: 6px 8px; {if is_return { \"color: #dc3545; font-weight: 600;\" } else { \"\" }}">
                                                        {item.operation_type}
                                                        {if is_return { " (Возврат)" } else { "" }}
                                                    </td>
                                                    <td style="padding: 6px 8px;">{item.loaded_at_utc.clone()}</td>
                                                </tr>
                                            }
                                        }
                                    />
                                </tbody>
                            </table>
                        </div>
                        }.into_any()
                    }
                }
            }}

            // Details panel для OZON FBS Posting
            {move || {
                if let Some(selected) = selected_posting.get() {
                    view! {
                        <div style="position: fixed; top: 0; right: 0; width: 50%; height: 100%; background: white; box-shadow: -2px 0 8px rgba(0,0,0,0.1); overflow-y: auto; z-index: 1000;">
                            <div style="padding: 16px;">
                                <OzonFbsPostingDetail
                                    id=selected.posting_id.clone()
                                    on_close=move || close_details()
                                />
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}
        </div>
    }
}

async fn fetch_finance_realization(
    query_params: &str,
) -> Result<OzonFinanceRealizationListResponse, String> {
    let window = web_sys::window().ok_or("No window object")?;
    let url = format!("/api/p902/finance-realization{}", query_params);

    let resp_value = JsFuture::from(window.fetch_with_str(&url))
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;

    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| "Failed to cast to Response")?;

    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }

    let json = JsFuture::from(resp.json().map_err(|_| "Failed to get JSON")?)
        .await
        .map_err(|e| format!("Failed to parse JSON: {:?}", e))?;

    serde_wasm_bindgen::from_value(json).map_err(|e| format!("Failed to deserialize: {:?}", e))
}
