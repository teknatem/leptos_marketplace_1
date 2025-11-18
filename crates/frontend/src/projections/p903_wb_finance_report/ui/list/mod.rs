use chrono::Utc;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbFinanceReportDto {
    pub rr_dt: String,
    pub rrd_id: i64,
    pub connection_mp_ref: String,
    pub organization_ref: String,
    pub acquiring_fee: Option<f64>,
    pub acquiring_percent: Option<f64>,
    pub additional_payment: Option<f64>,
    pub bonus_type_name: Option<String>,
    pub commission_percent: Option<f64>,
    pub delivery_amount: Option<f64>,
    pub delivery_rub: Option<f64>,
    pub nm_id: Option<i64>,
    pub penalty: Option<f64>,
    pub ppvz_vw: Option<f64>,
    pub ppvz_vw_nds: Option<f64>,
    pub ppvz_sales_commission: Option<f64>,
    pub quantity: Option<i32>,
    pub rebill_logistic_cost: Option<f64>,
    pub retail_amount: Option<f64>,
    pub retail_price: Option<f64>,
    pub retail_price_withdisc_rub: Option<f64>,
    pub return_amount: Option<f64>,
    pub sa_name: Option<String>,
    pub storage_fee: Option<f64>,
    pub subject_name: Option<String>,
    pub supplier_oper_name: Option<String>,
    pub cashback_amount: Option<f64>,
    pub ppvz_for_pay: Option<f64>,
    pub ppvz_kvw_prc: Option<f64>,
    pub ppvz_kvw_prc_base: Option<f64>,
    pub srv_dbs: Option<i32>,
    pub loaded_at_utc: String,
    pub payload_version: i32,
    pub extra: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbFinanceReportListResponse {
    pub items: Vec<WbFinanceReportDto>,
    pub total_count: i32,
    pub has_more: bool,
}

#[derive(Debug, Clone)]
struct SelectedReport {
    rr_dt: String,
    rrd_id: i64,
}

async fn fetch_connections() -> Result<Vec<(String, String)>, String> {
    let window = web_sys::window().ok_or("No window object")?;
    let url = "/api/a006/connection-mp/list";

    let resp_value = JsFuture::from(window.fetch_with_str(url))
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

    let connections: serde_json::Value = serde_wasm_bindgen::from_value(json)
        .map_err(|e| format!("Failed to deserialize: {:?}", e))?;

    let mut result = Vec::new();
    if let Some(items) = connections.as_array() {
        for item in items {
            if let (Some(id), Some(name)) = (
                item.get("id").and_then(|v| v.as_str()),
                item.get("name").and_then(|v| v.as_str()),
            ) {
                result.push((id.to_string(), name.to_string()));
            }
        }
    }

    Ok(result)
}

#[component]
pub fn WbFinanceReportList() -> impl IntoView {
    let (data, set_data) = signal(Vec::<WbFinanceReportDto>::new());
    let (total_count, set_total_count) = signal(0);
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(None::<String>);
    let (selected_report, set_selected_report) = signal::<Option<SelectedReport>>(None);

    // Фильтры - период по умолчанию
    let now = Utc::now().date_naive();
    let default_start = now - chrono::Duration::days(30);
    let default_end = now;

    let (date_from, set_date_from) = signal(default_start.format("%Y-%m-%d").to_string());
    let (date_to, set_date_to) = signal(default_end.format("%Y-%m-%d").to_string());
    let (nm_id_filter, set_nm_id_filter) = signal("".to_string());
    let (sa_name_filter, set_sa_name_filter) = signal("".to_string());
    let (connection_filter, set_connection_filter) = signal("".to_string());
    let (operation_filter, set_operation_filter) = signal("".to_string());
    let (sort_by, set_sort_by) = signal("rr_dt".to_string());
    let (sort_desc, set_sort_desc) = signal(true);

    // Загрузка списка подключений для отображения названий
    let (connections, set_connections) = signal(Vec::<(String, String)>::new());
    
    Effect::new(move || {
        spawn_local(async move {
            // Загружаем подключения
            if let Ok(conns) = fetch_connections().await {
                set_connections.set(conns);
            }
        });
    });

    let load_data = move || {
        set_loading.set(true);
        set_error.set(None);

        let date_from_val = date_from.get();
        let date_to_val = date_to.get();
        let nm_id_val = nm_id_filter.get();
        let sa_name_val = sa_name_filter.get();
        let connection_val = connection_filter.get();
        let operation_val = operation_filter.get();
        let sort_by_val = sort_by.get();
        let sort_desc_val = sort_desc.get();

        let mut query_params = format!(
            "?date_from={}&date_to={}&sort_by={}&sort_desc={}&limit=10000&offset=0",
            date_from_val, date_to_val, sort_by_val, sort_desc_val
        );

        if !nm_id_val.is_empty() {
            if let Ok(nm_id) = nm_id_val.parse::<i64>() {
                query_params.push_str(&format!("&nm_id={}", nm_id));
            }
        }
        if !sa_name_val.is_empty() {
            query_params.push_str(&format!("&sa_name={}", sa_name_val));
        }
        if !connection_val.is_empty() {
            query_params.push_str(&format!("&connection_mp_ref={}", connection_val));
        }
        if !operation_val.is_empty() {
            query_params.push_str(&format!("&supplier_oper_name={}", operation_val));
        }

        spawn_local(async move {
            match fetch_finance_report(&query_params).await {
                Ok(response) => {
                    set_total_count.set(response.total_count);
                    set_data.set(response.items);
                    set_loading.set(false);
                }
                Err(e) => {
                    log!("Failed to fetch finance report: {:?}", e);
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    };

    let handle_row_click = move |rr_dt: String, rrd_id: i64| {
        set_selected_report.set(Some(SelectedReport { rr_dt, rrd_id }));
    };

    let close_details = move || {
        set_selected_report.set(None);
    };

    // Helper для получения имени подключения
    let get_connection_name = move |connection_id: &str| -> String {
        connections
            .get()
            .iter()
            .find(|(id, _)| id == connection_id)
            .map(|(_, name)| name.clone())
            .unwrap_or_else(|| connection_id.to_string())
    };

    // Функция для сортировки по колонке
    let handle_column_sort = move |column: &str| {
        let current_sort = sort_by.get();
        if current_sort == column {
            set_sort_desc.set(!sort_desc.get());
        } else {
            set_sort_by.set(column.to_string());
            set_sort_desc.set(true);
        }
        load_data();
    };

    // Экспорт в Excel
    let export_to_excel = move || {
        let items = data.get();
        if items.is_empty() {
            log!("No data to export");
            return;
        }

        // UTF-8 BOM для правильного отображения кириллицы в Excel
        let mut csv = String::from("\u{FEFF}");

        // Заголовок с точкой с запятой как разделитель
        csv.push_str("Date;RRD_ID;NM_ID;SA_Name;Subject;Operation;Qty;Retail_Amount;Price_withDisc;Commission%;Sales_Commission;Acquiring_Fee;Penalty;Storage_Fee;Loaded_At\n");

        for item in items {
            let nm_id_str = item
                .nm_id
                .map(|n| n.to_string())
                .unwrap_or_else(|| "-".to_string());
            let sa_name_str = item
                .sa_name
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("-");
            let subject_str = item
                .subject_name
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("-");
            let operation_str = item
                .supplier_oper_name
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("-");

            let qty_str = item
                .quantity
                .map(|q| format!("{}", q))
                .unwrap_or_else(|| "-".to_string());
            let retail_amount_str = item
                .retail_amount
                .map(|r| format!("{:.2}", r).replace(".", ","))
                .unwrap_or_else(|| "-".to_string());
            let price_withdisc_str = item
                .retail_price_withdisc_rub
                .map(|p| format!("{:.2}", p).replace(".", ","))
                .unwrap_or_else(|| "-".to_string());
            let commission_str = item
                .commission_percent
                .map(|c| format!("{:.2}", c).replace(".", ","))
                .unwrap_or_else(|| "-".to_string());
            let sales_commission_str = item
                .ppvz_sales_commission
                .map(|sc| format!("{:.2}", sc).replace(".", ","))
                .unwrap_or_else(|| "-".to_string());
            let acquiring_str = item
                .acquiring_fee
                .map(|a| format!("{:.2}", a).replace(".", ","))
                .unwrap_or_else(|| "-".to_string());
            let penalty_str = item
                .penalty
                .map(|p| format!("{:.2}", p).replace(".", ","))
                .unwrap_or_else(|| "-".to_string());
            let storage_str = item
                .storage_fee
                .map(|s| format!("{:.2}", s).replace(".", ","))
                .unwrap_or_else(|| "-".to_string());

            csv.push_str(&format!(
                "\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";\"{}\";{};{};{};{};{};{};{};{};\"{}\"\n",
                item.rr_dt,
                item.rrd_id,
                nm_id_str,
                sa_name_str.replace('\"', "\"\""),
                subject_str.replace('\"', "\"\""),
                operation_str.replace('\"', "\"\""),
                qty_str,
                retail_amount_str,
                price_withdisc_str,
                commission_str,
                sales_commission_str,
                acquiring_str,
                penalty_str,
                storage_str,
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
                                "wb_finance_report_{}.csv",
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
        <div class="finance-report-list">
            <div style="display: flex; align-items: center; gap: 12px; margin-bottom: 12px;">
                <h2 style="margin: 0; font-size: var(--font-size-h3);">
                    "WB Finance Report (P903)"
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

            <div style="display: flex; gap: 8px; margin-bottom: 12px; flex-wrap: wrap;">
                <input
                    type="text"
                    placeholder="NM ID..."
                    prop:value=move || nm_id_filter.get()
                    on:input=move |ev| {
                        set_nm_id_filter.set(event_target_value(&ev));
                    }

                    style="padding: 4px 8px; border: 1px solid var(--border-color); border-radius: 4px; flex: 1; min-width: 120px;"
                />
                <input
                    type="text"
                    placeholder="SA Name (Артикул продавца)..."
                    prop:value=move || sa_name_filter.get()
                    on:input=move |ev| {
                        set_sa_name_filter.set(event_target_value(&ev));
                    }

                    style="padding: 4px 8px; border: 1px solid var(--border-color); border-radius: 4px; flex: 2; min-width: 180px;"
                />
                
                <select
                    on:change=move |ev| {
                        set_connection_filter.set(event_target_value(&ev));
                    }

                    style="padding: 4px 8px; border: 1px solid var(--border-color); border-radius: 4px; flex: 1; min-width: 150px;"
                >
                    <option value="">"Все кабинеты"</option>
                    <For
                        each=move || connections.get()
                        key=|conn| conn.0.clone()
                        children=move |conn: (String, String)| {
                            view! {
                                <option value={conn.0.clone()}>{conn.1.clone()}</option>
                            }
                        }
                    />

                </select>

                <input
                    type="text"
                    placeholder="Operation (Тип операции)..."
                    prop:value=move || operation_filter.get()
                    on:input=move |ev| {
                        set_operation_filter.set(event_target_value(&ev));
                    }

                    style="padding: 4px 8px; border: 1px solid var(--border-color); border-radius: 4px; flex: 1; min-width: 150px;"
                />

                <select
                    on:change=move |ev| {
                        set_sort_by.set(event_target_value(&ev));
                    }

                    style="padding: 4px 8px; border: 1px solid var(--border-color); border-radius: 4px;"
                >
                    <option value="rr_dt">"Sort by Date"</option>
                    <option value="nm_id">"Sort by NM ID"</option>
                    <option value="sa_name">"Sort by SA Name"</option>
                    <option value="quantity">"Sort by Quantity"</option>
                    <option value="retail_amount">"Sort by Amount"</option>
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

                        let sort_indicator = move |col: &str| {
                            if current_sort == col {
                                if is_desc { " ↓" } else { " ↑" }
                            } else {
                                ""
                            }
                        };

                        // Расчет итогов
                        let total_qty: i32 = items
                            .iter()
                            .map(|item| item.quantity.unwrap_or(0))
                            .sum();
                        let total_retail: f64 = items
                            .iter()
                            .map(|item| item.retail_amount.unwrap_or(0.0))
                            .sum();
                        let total_penalty: f64 = items
                            .iter()
                            .map(|item| item.penalty.unwrap_or(0.0))
                            .sum();
                        let total_storage: f64 = items
                            .iter()
                            .map(|item| item.storage_fee.unwrap_or(0.0))
                            .sum();

                        view! {
                            <div style="padding: 8px 12px; margin-bottom: 8px; background: var(--secondary-bg-color); border: 1px solid var(--border-color); border-radius: 4px; font-weight: bold; display: flex; gap: 24px;">
                                <span>"ИТОГО:"</span>
                                <span>"Qty: " {total_qty}</span>
                                <span>"Retail: " {format!("{:.2}", total_retail)}</span>
                                <span>"Penalty: " {format!("{:.2}", total_penalty)}</span>
                                <span>"Storage: " {format!("{:.2}", total_storage)}</span>
                            </div>

                            <div style="max-height: calc(100vh - 300px); overflow-y: auto; border: 1px solid var(--border-color); border-radius: 4px;">
                                <table style="width: 100%; border-collapse: collapse; font-size: var(--font-size-sm);">
                                    <thead style="position: sticky; top: 0; z-index: 10; background: var(--secondary-bg-color);">
                                        <tr style="border-bottom: 2px solid var(--border-color);">
                                            <th
                                                on:click=move |_| handle_column_sort("rr_dt")
                                                style="padding: 8px; text-align: left; cursor: pointer; user-select: none; background: var(--secondary-bg-color);"
                                            >
                                                "Date" {sort_indicator("rr_dt")}
                                            </th>
                                            <th style="padding: 8px; text-align: left; background: var(--secondary-bg-color);">
                                                "Кабинет"
                                            </th>
                                            <th style="padding: 8px; text-align: left; background: var(--secondary-bg-color);">
                                                "RRD ID"
                                            </th>
                                            <th
                                                on:click=move |_| handle_column_sort("nm_id")
                                                style="padding: 8px; text-align: left; cursor: pointer; user-select: none; background: var(--secondary-bg-color);"
                                            >
                                                "NM ID" {sort_indicator("nm_id")}
                                            </th>
                                            <th
                                                on:click=move |_| handle_column_sort("sa_name")
                                                style="padding: 8px; text-align: left; cursor: pointer; user-select: none; background: var(--secondary-bg-color);"
                                            >
                                                "SA Name" {sort_indicator("sa_name")}
                                            </th>
                                            <th style="padding: 8px; text-align: left; background: var(--secondary-bg-color);">
                                                "Subject"
                                            </th>
                                            <th style="padding: 8px; text-align: left; background: var(--secondary-bg-color);">
                                                "Operation"
                                            </th>
                                            <th
                                                on:click=move |_| handle_column_sort("quantity")
                                                style="padding: 8px; text-align: right; cursor: pointer; user-select: none; background: var(--secondary-bg-color);"
                                            >
                                                "Qty" {sort_indicator("quantity")}
                                            </th>
                                            <th
                                                on:click=move |_| handle_column_sort("retail_amount")
                                                style="padding: 8px; text-align: right; cursor: pointer; user-select: none; background: var(--secondary-bg-color);"
                                            >
                                                "Retail" {sort_indicator("retail_amount")}
                                            </th>
                                            <th style="padding: 8px; text-align: right; background: var(--secondary-bg-color);">
                                                "Price w/Disc"
                                            </th>
                                            <th style="padding: 8px; text-align: right; background: var(--secondary-bg-color);">
                                                "Commission%"
                                            </th>
                                            <th style="padding: 8px; text-align: right; background: var(--secondary-bg-color);">
                                                "Sales Comm"
                                            </th>
                                            <th style="padding: 8px; text-align: right; background: var(--secondary-bg-color);">
                                                "Acquiring"
                                            </th>
                                            <th style="padding: 8px; text-align: right; background: var(--secondary-bg-color);">
                                                "Penalty"
                                            </th>
                                            <th style="padding: 8px; text-align: right; background: var(--secondary-bg-color);">
                                                "Logistics"
                                            </th>
                                            <th style="padding: 8px; text-align: right; background: var(--secondary-bg-color);">
                                                "Storage"
                                            </th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        <For
                                            each=move || items.clone()
                                            key=|item| (item.rr_dt.clone(), item.rrd_id)
                                            children=move |item: WbFinanceReportDto| {
                                                let rr_dt_clone = item.rr_dt.clone();
                                                let rrd_id_clone = item.rrd_id;
                                                view! {
                                                    <tr
                                                        on:click=move |_| {
                                                            handle_row_click(
                                                                rr_dt_clone.clone(),
                                                                rrd_id_clone,
                                                            )
                                                        }

                                                        style="border-bottom: 1px solid var(--border-color); cursor: pointer; hover:background: var(--hover-bg-color);"
                                                    >
                                                        <td style="padding: 6px 8px;">{item.rr_dt.clone()}</td>
                                                        <td style="padding: 6px 8px; font-size: 11px;">
                                                            {get_connection_name(&item.connection_mp_ref)}
                                                        </td>
                                                        <td style="padding: 6px 8px;">{item.rrd_id}</td>
                                                        <td style="padding: 6px 8px;">
                                                            {item
                                                                .nm_id
                                                                .map(|n| n.to_string())
                                                                .unwrap_or_else(|| "-".to_string())}
                                                        </td>
                                                        <td style="padding: 6px 8px;">
                                                            {item
                                                                .sa_name
                                                                .clone()
                                                                .unwrap_or_else(|| "-".to_string())}
                                                        </td>
                                                        <td style="padding: 6px 8px;">
                                                            {item
                                                                .subject_name
                                                                .clone()
                                                                .unwrap_or_else(|| "-".to_string())}
                                                        </td>
                                                        <td style="padding: 6px 8px;">
                                                            {item
                                                                .supplier_oper_name
                                                                .clone()
                                                                .unwrap_or_else(|| "-".to_string())}
                                                        </td>
                                                        <td style="padding: 6px 8px; text-align: right;">
                                                            {item
                                                                .quantity
                                                                .map(|q| q.to_string())
                                                                .unwrap_or_else(|| "-".to_string())}
                                                        </td>
                                                        <td style="padding: 6px 8px; text-align: right;">
                                                            {item
                                                                .retail_amount
                                                                .map(|r| format!("{:.2}", r))
                                                                .unwrap_or_else(|| "-".to_string())}
                                                        </td>
                                                        <td style="padding: 6px 8px; text-align: right;">
                                                            {item
                                                                .retail_price_withdisc_rub
                                                                .map(|p| format!("{:.2}", p))
                                                                .unwrap_or_else(|| "-".to_string())}
                                                        </td>
                                                        <td style="padding: 6px 8px; text-align: right;">
                                                            {item
                                                                .commission_percent
                                                                .map(|c| format!("{:.2}", c))
                                                                .unwrap_or_else(|| "-".to_string())}
                                                        </td>
                                                        <td style="padding: 6px 8px; text-align: right;">
                                                            {item
                                                                .ppvz_sales_commission
                                                                .map(|sc| format!("{:.2}", sc))
                                                                .unwrap_or_else(|| "-".to_string())}
                                                        </td>
                                                        <td style="padding: 6px 8px; text-align: right;">
                                                            {item
                                                                .acquiring_fee
                                                                .map(|a| format!("{:.2}", a))
                                                                .unwrap_or_else(|| "-".to_string())}
                                                        </td>
                                                        <td style="padding: 6px 8px; text-align: right;">
                                                            {item
                                                                .penalty
                                                                .map(|p| format!("{:.2}", p))
                                                                .unwrap_or_else(|| "-".to_string())}
                                                        </td>
                                                        <td style="padding: 6px 8px; text-align: right;">
                                                            {item
                                                                .rebill_logistic_cost
                                                                .map(|l| format!("{:.2}", l))
                                                                .unwrap_or_else(|| "-".to_string())}
                                                        </td>
                                                        <td style="padding: 6px 8px; text-align: right;">
                                                            {item
                                                                .storage_fee
                                                                .map(|s| format!("{:.2}", s))
                                                                .unwrap_or_else(|| "-".to_string())}
                                                        </td>
                                                    </tr>
                                                }
                                            }
                                        />

                                    </tbody>
                                </table>
                            </div>
                        }
                            .into_any()
                    }
                }
            }}

            // Details panel - Модальное окно в центре экрана
            {move || {
                if let Some(selected) = selected_report.get() {
                    view! {
                        <div style="position: fixed; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 1000;">
                            <div style="background: white; border-radius: 8px; box-shadow: 0 4px 16px rgba(0,0,0,0.2); width: 90%; max-width: 1200px; max-height: 90vh; overflow: hidden; display: flex; flex-direction: column;">
                                <crate::projections::p903_wb_finance_report::ui::details::WbFinanceReportDetail
                                    rr_dt=selected.rr_dt.clone()
                                    rrd_id=selected.rrd_id
                                    on_close=move || close_details()
                                />
                            </div>
                        </div>
                    }
                        .into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}

        </div>
    }
}

async fn fetch_finance_report(
    query_params: &str,
) -> Result<WbFinanceReportListResponse, String> {
    let window = web_sys::window().ok_or("No window object")?;
    let url = format!("/api/p903/finance-report{}", query_params);

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
