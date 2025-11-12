use chrono::{Datelike, Utc};
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

// Импорты компонентов деталей документов
use crate::domain::a010_ozon_fbs_posting::ui::details::OzonFbsPostingDetail;
use crate::domain::a012_wb_sales::ui::details::WbSalesDetail;
use crate::domain::a013_ym_order::ui::details::YmOrderDetail;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesRegisterDto {
    pub marketplace: String,
    pub document_no: String,
    pub line_id: String,
    pub scheme: Option<String>,
    pub document_type: String,
    pub document_version: i32,
    pub connection_mp_ref: String,
    pub organization_ref: String,
    pub marketplace_product_ref: Option<String>,
    pub nomenclature_ref: Option<String>,
    pub registrator_ref: String,
    pub event_time_source: String,
    pub sale_date: String,
    pub source_updated_at: Option<String>,
    pub status_source: String,
    pub status_norm: String,
    pub seller_sku: Option<String>,
    pub mp_item_id: String,
    pub barcode: Option<String>,
    pub title: Option<String>,
    pub qty: f64,
    pub price_list: Option<f64>,
    pub discount_total: Option<f64>,
    pub price_effective: Option<f64>,
    pub amount_line: Option<f64>,
    pub currency_code: Option<String>,
    pub loaded_at_utc: String,
    pub payload_version: i32,
    pub extra: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesRegisterListResponse {
    pub items: Vec<SalesRegisterDto>,
    pub total_count: i32,
    pub has_more: bool,
}

#[derive(Debug, Clone)]
struct SelectedDocument {
    document_type: String,
    document_id: String,
}

#[derive(Debug, Clone, PartialEq)]
enum SortColumn {
    Date,
    Marketplace,
    DocumentNo,
    Product,
    Sku,
    Qty,
    Amount,
    Status,
}

#[derive(Debug, Clone, PartialEq)]
enum SortDirection {
    Asc,
    Desc,
}

#[component]
pub fn SalesRegisterList() -> impl IntoView {
    let (sales, set_sales) = signal(Vec::<SalesRegisterDto>::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(None::<String>);
    let (selected_document, set_selected_document) = signal::<Option<SelectedDocument>>(None);

    // Состояние сортировки
    let (sort_column, set_sort_column) = signal::<Option<SortColumn>>(None);
    let (sort_direction, set_sort_direction) = signal(SortDirection::Asc);

    // Фильтры - период по умолчанию: текущий месяц
    let now = Utc::now().date_naive();
    let year = now.year();
    let month = now.month();
    let month_start =
        chrono::NaiveDate::from_ymd_opt(year, month, 1).expect("Invalid month start date");
    // Вычисляем последний день месяца: первый день следующего месяца минус 1 день
    let month_end = if month == 12 {
        chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
            .map(|d| d - chrono::Duration::days(1))
            .expect("Invalid month end date")
    } else {
        chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
            .map(|d| d - chrono::Duration::days(1))
            .expect("Invalid month end date")
    };

    let (date_from, set_date_from) = signal(month_start.format("%Y-%m-%d").to_string());
    let (date_to, set_date_to) = signal(month_end.format("%Y-%m-%d").to_string());
    let (marketplace_filter, set_marketplace_filter) = signal("".to_string());

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
    let sorted_sales = move || {
        let mut data = sales.get();
        if let Some(col) = sort_column.get() {
            let direction = sort_direction.get();
            data.sort_by(|a, b| {
                let cmp = match col {
                    SortColumn::Date => a.sale_date.cmp(&b.sale_date),
                    SortColumn::Marketplace => a.marketplace.cmp(&b.marketplace),
                    SortColumn::DocumentNo => a.document_no.cmp(&b.document_no),
                    SortColumn::Product => {
                        let a_title = a.title.as_deref().unwrap_or("");
                        let b_title = b.title.as_deref().unwrap_or("");
                        a_title.cmp(b_title)
                    }
                    SortColumn::Sku => {
                        let a_sku = a.seller_sku.as_deref().unwrap_or("");
                        let b_sku = b.seller_sku.as_deref().unwrap_or("");
                        a_sku.cmp(b_sku)
                    }
                    SortColumn::Qty => a.qty.partial_cmp(&b.qty).unwrap_or(std::cmp::Ordering::Equal),
                    SortColumn::Amount => {
                        let a_amt = a.amount_line.unwrap_or(0.0);
                        let b_amt = b.amount_line.unwrap_or(0.0);
                        a_amt.partial_cmp(&b_amt).unwrap_or(std::cmp::Ordering::Equal)
                    }
                    SortColumn::Status => a.status_norm.cmp(&b.status_norm),
                };
                match direction {
                    SortDirection::Asc => cmp,
                    SortDirection::Desc => cmp.reverse(),
                }
            });
        }
        data
    };

    let load_sales = move || {
        set_loading.set(true);
        set_error.set(None);

        let date_from_val = date_from.get();
        let date_to_val = date_to.get();
        let marketplace_val = marketplace_filter.get();

        let mut query_params = format!(
            "?date_from={}&date_to={}&limit=10000&offset=0",
            date_from_val, date_to_val
        );

        if !marketplace_val.is_empty() {
            query_params.push_str(&format!("&marketplace={}", marketplace_val));
        }

        spawn_local(async move {
            match fetch_sales(&query_params).await {
                Ok(data) => {
                    set_sales.set(data.items);
                    set_loading.set(false);
                }
                Err(e) => {
                    log!("Failed to fetch sales: {:?}", e);
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    };

    view! {
        <div class="sales-register-list">
            <div style="display: flex; align-items: center; gap: 12px; margin-bottom: 12px;">
                <h2 style="margin: 0; font-size: var(--font-size-h3); line-height: 1.2;">"Sales Register (P900)"</h2>

                <label style="margin: 0; font-size: var(--font-size-sm); white-space: nowrap;">"From:"</label>
                <input
                    type="date"
                    prop:value=date_from
                    on:input=move |ev| {
                        set_date_from.set(event_target_value(&ev));
                    }
                    style="padding: 4px 8px; border: 1px solid var(--color-border-light); border-radius: 4px; font-size: var(--font-size-sm);"
                />

                <label style="margin: 0; font-size: var(--font-size-sm); white-space: nowrap;">"To:"</label>
                <input
                    type="date"
                    prop:value=date_to
                    on:input=move |ev| {
                        set_date_to.set(event_target_value(&ev));
                    }
                    style="padding: 4px 8px; border: 1px solid var(--color-border-light); border-radius: 4px; font-size: var(--font-size-sm);"
                />

                <label style="margin: 0; font-size: var(--font-size-sm); white-space: nowrap;">"MP:"</label>
                <select
                    prop:value=marketplace_filter
                    on:change=move |ev| {
                        set_marketplace_filter.set(event_target_value(&ev));
                    }
                    style="padding: 4px 8px; border: 1px solid var(--color-border-light); border-radius: 4px; font-size: var(--font-size-sm);"
                >
                    <option value="">"All"</option>
                    <option value="OZON">"OZON"</option>
                    <option value="WB">"Wildberries"</option>
                    <option value="YM">"Yandex Market"</option>
                </select>

                <button
                    on:click=move |_| {
                        load_sales();
                    }
                    style="padding: 4px 12px; background: #4CAF50; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: var(--font-size-sm);"
                >
                    "Обновить"
                </button>

                {move || if !loading.get() {
                    view! {
                        <span style="margin-left: 8px; font-size: var(--font-size-sm); color: var(--color-text-muted);">
                            "Total: " {sales.get().len()} " records"
                        </span>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }}
            </div>

            {move || {
                if loading.get() {
                    view! { <div>"Loading..."</div> }.into_any()
                } else if let Some(err) = error.get() {
                    view! { <div style="color: red;">{err}</div> }.into_any()
                } else {
                    view! {
                        <div>
                            // Модальное окно для деталей документа
                            {move || {
                                if let Some(selected) = selected_document.get() {
                                    view! {
                                        <div class="modal-overlay" style="align-items: flex-start; padding-top: 40px;">
                                            <div class="modal-content" style="max-width: 1200px; height: calc(100vh - 80px); overflow: hidden; margin: 0;">
                                                {match selected.document_type.as_str() {
                                                    "OZON_FBS_Posting" => {
                                                        view! {
                                                            <OzonFbsPostingDetail
                                                                id=selected.document_id.clone()
                                                                on_close=move || set_selected_document.set(None)
                                                            />
                                                        }.into_any()
                                                    }
                                                    "OZON_FBO_Posting" => {
                                                        view! {
                                                            <div style="padding: 20px;">
                                                                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                                                                    <h2 style="margin: 0;">"OZON FBO Posting Details"</h2>
                                                                    <button
                                                                        on:click=move |_| set_selected_document.set(None)
                                                                        style="padding: 8px 16px; background: #f44336; color: white; border: none; border-radius: 4px; cursor: pointer;"
                                                                    >
                                                                        "✕ Close"
                                                                    </button>
                                                                </div>
                                                                <div style="padding: 20px; background: #fff3cd; border: 1px solid #ffc107; border-radius: 4px;">
                                                                    <p>"OZON FBO Posting details component not yet implemented."</p>
                                                                    <p style="font-family: monospace; font-size: 0.9em; margin-top: 10px;">
                                                                        "Document ID: " {selected.document_id.clone()}
                                                                    </p>
                                                                </div>
                                                            </div>
                                                        }.into_any()
                                                    }
                                                    "WB_Sales" => {
                                                        view! {
                                                            <WbSalesDetail
                                                                id=selected.document_id.clone()
                                                                on_close=move || set_selected_document.set(None)
                                                            />
                                                        }.into_any()
                                                    }
                                                    "YM_Order" => {
                                                        view! {
                                                            <YmOrderDetail
                                                                id=selected.document_id.clone()
                                                                on_close=move || set_selected_document.set(None)
                                                            />
                                                        }.into_any()
                                                    }
                                                    _ => {
                                                        view! {
                                                            <div style="padding: 20px;">
                                                                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                                                                    <h2 style="margin: 0;">"Unknown Document Type"</h2>
                                                                    <button
                                                                        on:click=move |_| set_selected_document.set(None)
                                                                        style="padding: 8px 16px; background: #f44336; color: white; border: none; border-radius: 4px; cursor: pointer;"
                                                                    >
                                                                        "✕ Close"
                                                                    </button>
                                                                </div>
                                                                <div style="padding: 20px; background: #ffebee; border: 1px solid #ef5350; border-radius: 4px; color: #c62828;">
                                                                    <p>"Unknown document type: " {selected.document_type.clone()}</p>
                                                                    <p style="font-family: monospace; font-size: 0.9em; margin-top: 10px;">
                                                                        "Document ID: " {selected.document_id.clone()}
                                                                    </p>
                                                                </div>
                                                            </div>
                                                        }.into_any()
                                                    }
                                                }}
                                            </div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <></> }.into_any()
                                }
                            }}

                            <div style="overflow-y: auto; max-height: calc(100vh - 200px); border: 1px solid #ddd;">
                                <table class="data-table" style="width: 100%; border-collapse: collapse; margin: 0;">
                                    <thead style="position: sticky; top: 0; z-index: 10; background: #f5f5f5;">
                                        <tr style="background: #f5f5f5;">
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=move |_| handle_column_click(SortColumn::Date)
                                            >
                                                "Date "
                                                {move || {
                                                    if sort_column.get() == Some(SortColumn::Date) {
                                                        match sort_direction.get() {
                                                            SortDirection::Asc => "↑",
                                                            SortDirection::Desc => "↓",
                                                        }
                                                    } else {
                                                        ""
                                                    }
                                                }}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=move |_| handle_column_click(SortColumn::Marketplace)
                                            >
                                                "Marketplace "
                                                {move || {
                                                    if sort_column.get() == Some(SortColumn::Marketplace) {
                                                        match sort_direction.get() {
                                                            SortDirection::Asc => "↑",
                                                            SortDirection::Desc => "↓",
                                                        }
                                                    } else {
                                                        ""
                                                    }
                                                }}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=move |_| handle_column_click(SortColumn::DocumentNo)
                                            >
                                                "Document № "
                                                {move || {
                                                    if sort_column.get() == Some(SortColumn::DocumentNo) {
                                                        match sort_direction.get() {
                                                            SortDirection::Asc => "↑",
                                                            SortDirection::Desc => "↓",
                                                        }
                                                    } else {
                                                        ""
                                                    }
                                                }}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=move |_| handle_column_click(SortColumn::Product)
                                            >
                                                "Product "
                                                {move || {
                                                    if sort_column.get() == Some(SortColumn::Product) {
                                                        match sort_direction.get() {
                                                            SortDirection::Asc => "↑",
                                                            SortDirection::Desc => "↓",
                                                        }
                                                    } else {
                                                        ""
                                                    }
                                                }}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=move |_| handle_column_click(SortColumn::Sku)
                                            >
                                                "SKU "
                                                {move || {
                                                    if sort_column.get() == Some(SortColumn::Sku) {
                                                        match sort_direction.get() {
                                                            SortDirection::Asc => "↑",
                                                            SortDirection::Desc => "↓",
                                                        }
                                                    } else {
                                                        ""
                                                    }
                                                }}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=move |_| handle_column_click(SortColumn::Qty)
                                            >
                                                "Qty "
                                                {move || {
                                                    if sort_column.get() == Some(SortColumn::Qty) {
                                                        match sort_direction.get() {
                                                            SortDirection::Asc => "↑",
                                                            SortDirection::Desc => "↓",
                                                        }
                                                    } else {
                                                        ""
                                                    }
                                                }}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=move |_| handle_column_click(SortColumn::Amount)
                                            >
                                                "Amount "
                                                {move || {
                                                    if sort_column.get() == Some(SortColumn::Amount) {
                                                        match sort_direction.get() {
                                                            SortDirection::Asc => "↑",
                                                            SortDirection::Desc => "↓",
                                                        }
                                                    } else {
                                                        ""
                                                    }
                                                }}
                                            </th>
                                            <th
                                                style="border: 1px solid #ddd; padding: 8px; cursor: pointer; user-select: none;"
                                                on:click=move |_| handle_column_click(SortColumn::Status)
                                            >
                                                "Status "
                                                {move || {
                                                    if sort_column.get() == Some(SortColumn::Status) {
                                                        match sort_direction.get() {
                                                            SortDirection::Asc => "↑",
                                                            SortDirection::Desc => "↓",
                                                        }
                                                    } else {
                                                        ""
                                                    }
                                                }}
                                            </th>
                                            <th style="border: 1px solid #ddd; padding: 8px;">"Organization"</th>
                                        </tr>
                                    </thead>
                                <tbody>
                                    {sorted_sales().into_iter().map(|sale| {
                                        let sale_date = sale.sale_date.clone();
                                        let marketplace = sale.marketplace.clone();
                                        let document_no = sale.document_no.clone();
                                        let title = sale.title.clone().unwrap_or_default();
                                        let seller_sku = sale.seller_sku.clone().unwrap_or_default();
                                        let qty = sale.qty;
                                        let amount_line = sale.amount_line.unwrap_or(0.0);
                                        let status_norm = sale.status_norm.clone();
                                        let org_ref = sale.organization_ref.clone();
                                        let org_ref_short = org_ref[..8.min(org_ref.len())].to_string();

                                        // Данные для открытия документа
                                        let document_type = sale.document_type.clone();
                                        let registrator_ref = sale.registrator_ref.clone();
                                        let document_no_for_display = document_no.clone();

                                        view! {
                                            <tr>
                                                <td style="border: 1px solid #ddd; padding: 8px;">{sale_date}</td>
                                                <td style="border: 1px solid #ddd; padding: 8px;">{marketplace}</td>
                                                <td style="border: 1px solid #ddd; padding: 8px;">
                                                    <a
                                                        href="#"
                                                        style="color: #2196F3; text-decoration: underline; cursor: pointer;"
                                                        on:click=move |ev| {
                                                            ev.prevent_default();
                                                            set_selected_document.set(Some(SelectedDocument {
                                                                document_type: document_type.clone(),
                                                                document_id: registrator_ref.clone(),
                                                            }));
                                                        }
                                                    >
                                                        {document_no_for_display}
                                                    </a>
                                                </td>
                                                <td style="border: 1px solid #ddd; padding: 8px;">{title}</td>
                                                <td style="border: 1px solid #ddd; padding: 8px;">{seller_sku}</td>
                                                <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{format!("{:.2}", qty)}</td>
                                                <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{format!("{:.2}", amount_line)}</td>
                                                <td style="border: 1px solid #ddd; padding: 8px;">{status_norm}</td>
                                                <td style="border: 1px solid #ddd; padding: 8px;">
                                                    // UUID ссылка на организацию
                                                    <span title=org_ref style="font-family: monospace; font-size: 0.85em; color: #666;">
                                                        {org_ref_short}
                                                        "..."
                                                    </span>
                                                </td>
                                            </tr>
                                        }
                                    }).collect_view()}
                                </tbody>
                            </table>
                            </div>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}

async fn fetch_sales(query_params: &str) -> Result<SalesRegisterListResponse, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("/api/p900/sales-register{}", query_params);
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    let data: SalesRegisterListResponse =
        serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}
