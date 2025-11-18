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
pub struct WbFinanceReportDetailResponse {
    pub item: WbFinanceReportDto,
}

#[derive(Debug, Clone)]
struct FieldRow {
    description: String,
    field_id: String,
    value: String,
}

#[component]
pub fn WbFinanceReportDetail(
    rr_dt: String,
    rrd_id: i64,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let (data, set_data) = signal::<Option<WbFinanceReportDto>>(None);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(None::<String>);
    let (active_tab, set_active_tab) = signal("fields");
    let (sort_by, set_sort_by) = signal("description".to_string());
    let (sort_desc, set_sort_desc) = signal(false);

    // Загрузка данных
    let rr_dt_clone = rr_dt.clone();
    Effect::new(move || {
        let rr_dt = rr_dt_clone.clone();
        let rrd_id_val = rrd_id;

        spawn_local(async move {
            match fetch_detail(&rr_dt, rrd_id_val).await {
                Ok(response) => {
                    set_data.set(Some(response.item));
                    set_loading.set(false);
                }
                Err(e) => {
                    log!("Failed to fetch finance report detail: {:?}", e);
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    });

    // Преобразование данных в таблицу полей
    let get_field_rows = move || -> Vec<FieldRow> {
        let Some(item) = data.get() else {
            return Vec::new();
        };

        let mut rows = vec![
            FieldRow {
                description: "Эквайринг/Комиссии за организацию платежей".to_string(),
                field_id: "acquiring_fee".to_string(),
                value: item
                    .acquiring_fee
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Размер комиссии за эквайринг/Комиссии за организацию платежей, %"
                    .to_string(),
                field_id: "acquiring_percent".to_string(),
                value: item
                    .acquiring_percent
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Корректировка Вознаграждения Вайлдберриз (ВВ)".to_string(),
                field_id: "additional_payment".to_string(),
                value: item
                    .additional_payment
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Виды логистики, штрафов и корректировок ВВ".to_string(),
                field_id: "bonus_type_name".to_string(),
                value: item
                    .bonus_type_name
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Размер кВВ, %".to_string(),
                field_id: "commission_percent".to_string(),
                value: item
                    .commission_percent
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Количество доставок".to_string(),
                field_id: "delivery_amount".to_string(),
                value: item
                    .delivery_amount
                    .map(|v| format!("{:.0}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Услуги по доставке товара покупателю".to_string(),
                field_id: "delivery_rub".to_string(),
                value: item
                    .delivery_rub
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Артикул WB".to_string(),
                field_id: "nm_id".to_string(),
                value: item
                    .nm_id
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Общая сумма штрафов".to_string(),
                field_id: "penalty".to_string(),
                value: item
                    .penalty
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Вознаграждение Вайлдберриз (ВВ), без НДС".to_string(),
                field_id: "ppvz_vw".to_string(),
                value: item
                    .ppvz_vw
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "НДС с вознаграждения Вайлдберриз".to_string(),
                field_id: "ppvz_vw_nds".to_string(),
                value: item
                    .ppvz_vw_nds
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Вознаграждение с продаж до вычета услуг поверенного, без НДС"
                    .to_string(),
                field_id: "ppvz_sales_commission".to_string(),
                value: item
                    .ppvz_sales_commission
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Количество".to_string(),
                field_id: "quantity".to_string(),
                value: item
                    .quantity
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Возмещение издержек по перевозке/по складским операциям с товаром"
                    .to_string(),
                field_id: "rebill_logistic_cost".to_string(),
                value: item
                    .rebill_logistic_cost
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Вайлдберриз реализовал Товар (Пр)".to_string(),
                field_id: "retail_amount".to_string(),
                value: item
                    .retail_amount
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Цена розничная".to_string(),
                field_id: "retail_price".to_string(),
                value: item
                    .retail_price
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Цена розничная с учётом согласованной скидки".to_string(),
                field_id: "retail_price_withdisc_rub".to_string(),
                value: item
                    .retail_price_withdisc_rub
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Количество возврата".to_string(),
                field_id: "return_amount".to_string(),
                value: item
                    .return_amount
                    .map(|v| format!("{:.0}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Дата операции".to_string(),
                field_id: "rr_dt".to_string(),
                value: item.rr_dt.clone(),
            },
            FieldRow {
                description: "Артикул продавца".to_string(),
                field_id: "sa_name".to_string(),
                value: item.sa_name.clone().unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Хранение".to_string(),
                field_id: "storage_fee".to_string(),
                value: item
                    .storage_fee
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Предмет".to_string(),
                field_id: "subject_name".to_string(),
                value: item.subject_name.clone().unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Обоснование для оплаты".to_string(),
                field_id: "supplier_oper_name".to_string(),
                value: item
                    .supplier_oper_name
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Сумма, удержанная за начисленные баллы программы лояльности"
                    .to_string(),
                field_id: "cashback_amount".to_string(),
                value: item
                    .cashback_amount
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "К перечислению продавцу за реализованный товар".to_string(),
                field_id: "ppvz_for_pay".to_string(),
                value: item
                    .ppvz_for_pay
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Итоговый кВВ без НДС, %".to_string(),
                field_id: "ppvz_kvw_prc".to_string(),
                value: item
                    .ppvz_kvw_prc
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Размер кВВ без НДС, % базовый".to_string(),
                field_id: "ppvz_kvw_prc_base".to_string(),
                value: item
                    .ppvz_kvw_prc_base
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Признак услуги платной доставки".to_string(),
                field_id: "srv_dbs".to_string(),
                value: item
                    .srv_dbs
                    .map(|v| {
                        if v == 1 {
                            "Да".to_string()
                        } else {
                            "Нет".to_string()
                        }
                    })
                    .unwrap_or_else(|| "-".to_string()),
            },
        ];

        // Сортировка
        let sort_field = sort_by.get();
        let is_desc = sort_desc.get();

        rows.sort_by(|a, b| {
            let cmp = match &*sort_field {
                "field_id" => a.field_id.cmp(&b.field_id),
                "value" => a.value.cmp(&b.value),
                _ => a.description.cmp(&b.description),
            };
            if is_desc {
                cmp.reverse()
            } else {
                cmp
            }
        });

        rows
    };

    let handle_column_sort = move |column: &'static str| {
        let current_sort = sort_by.get();
        if current_sort == column {
            set_sort_desc.set(!sort_desc.get());
        } else {
            set_sort_by.set(column.to_string());
            set_sort_desc.set(false);
        }
    };

    // Экспорт в Excel
    let export_to_excel = move || {
        let field_rows = get_field_rows();
        if field_rows.is_empty() {
            log!("No data to export");
            return;
        }

        // UTF-8 BOM для правильного отображения кириллицы в Excel
        let mut csv = String::from("\u{FEFF}");

        // Заголовок с точкой с запятой как разделитель
        csv.push_str("Описание;Идентификатор;Значение\n");

        for row in field_rows {
            csv.push_str(&format!(
                "\"{}\";\"{}\";\"{}\"\n",
                row.description.replace('\"', "\"\""),
                row.field_id.replace('\"', "\"\""),
                row.value.replace('\"', "\"\"")
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
                                "wb_finance_report_detail_{}.csv",
                                chrono::Utc::now().format("%Y%m%d_%H%M%S")
                            );
                            a.set_download(&filename);
                            let _ = a.click();
                            let _ = web_sys::Url::revoke_object_url(&url);
                        }
                    }
                }
            }
        }
    };

    view! {
        <div style="display: flex; flex-direction: column; height: 100%; overflow: hidden;">
            <div style="padding: 16px; display: flex; justify-content: space-between; align-items: center; border-bottom: 2px solid var(--border-color); flex-shrink: 0;">
                <h3 style="margin: 0;">"WB Finance Report Details"</h3>
                <button
                    on:click=move |_| on_close.run(())
                    style="padding: 8px 16px; background: #dc3545; color: white; border: none; border-radius: 4px; cursor: pointer;"
                >
                    "Close"
                </button>
            </div>

            <div style="padding: 16px; overflow-y: auto; flex: 1;">
            {move || {
                if loading.get() {
                    view! { <p>"Loading..."</p> }.into_any()
                } else if let Some(err) = error.get() {
                    view! { <p style="color: red;">"Error: " {err}</p> }.into_any()
                } else if data.get().is_some() {
                    view! {
                        <div>
                            // Tabs and Export button
                            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; border-bottom: 2px solid var(--border-color);">
                                <div style="display: flex; gap: 8px;">
                                    <button
                                        on:click=move |_| set_active_tab.set("fields")
                                        style=move || {
                                            if active_tab.get() == "fields" {
                                                "padding: 8px 16px; background: var(--primary-color); color: white; border: none; border-bottom: 3px solid var(--primary-color); cursor: pointer; font-weight: bold;"
                                            } else {
                                                "padding: 8px 16px; background: transparent; color: var(--text-color); border: none; cursor: pointer;"
                                            }
                                        }
                                    >
                                        "Fields"
                                    </button>
                                    <button
                                        on:click=move |_| set_active_tab.set("json")
                                        style=move || {
                                            if active_tab.get() == "json" {
                                                "padding: 8px 16px; background: var(--primary-color); color: white; border: none; border-bottom: 3px solid var(--primary-color); cursor: pointer; font-weight: bold;"
                                            } else {
                                                "padding: 8px 16px; background: transparent; color: var(--text-color); border: none; cursor: pointer;"
                                            }
                                        }
                                    >
                                        "Raw JSON"
                                    </button>
                                </div>
                                {
                                    let export_excel = export_to_excel.clone();
                                    view! {
                                        <button
                                            on:click=move |_| export_excel()
                                            style="padding: 8px 16px; background: #28a745; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 14px; margin-bottom: 2px;"
                                            title="Экспорт таблицы в Excel"
                                        >
                                            "📥 Export to Excel"
                                        </button>
                                    }
                                }
                            </div>

                            // Tab Content
                            {move || {
                                if active_tab.get() == "fields" {
                                    let field_rows = get_field_rows();
                                    let current_sort = sort_by.get();
                                    let is_desc = sort_desc.get();
                                    let sort_indicator = move |col: &str| {
                                        if current_sort == col {
                                            if is_desc { " ↓" } else { " ↑" }
                                        } else {
                                            ""
                                        }
                                    };
                                    view! {
                                        <div style="max-height: calc(100vh - 200px); overflow-y: auto;">
                                            <table style="width: 100%; border-collapse: collapse; font-size: var(--font-size-sm);">
                                                <thead style="position: sticky; top: 0; z-index: 10; background: var(--secondary-bg-color);">
                                                    <tr style="border-bottom: 2px solid var(--border-color);">
                                                        <th
                                                            on:click=move |_| handle_column_sort("description")
                                                            style="padding: 8px; text-align: left; cursor: pointer; user-select: none; background: var(--secondary-bg-color); width: 40%;"
                                                        >
                                                            "Описание" {sort_indicator("description")}
                                                        </th>
                                                        <th
                                                            on:click=move |_| handle_column_sort("field_id")
                                                            style="padding: 8px; text-align: left; cursor: pointer; user-select: none; background: var(--secondary-bg-color); width: 25%;"
                                                        >
                                                            "Идентификатор" {sort_indicator("field_id")}
                                                        </th>
                                                        <th
                                                            on:click=move |_| handle_column_sort("value")
                                                            style="padding: 8px; text-align: left; cursor: pointer; user-select: none; background: var(--secondary-bg-color); width: 35%;"
                                                        >
                                                            "Значение" {sort_indicator("value")}
                                                        </th>
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    <For
                                                        each=move || field_rows.clone()
                                                        key=|row| row.field_id.clone()
                                                        children=move |row: FieldRow| {
                                                            view! {
                                                                <tr style="border-bottom: 1px solid var(--border-color);">
                                                                    <td style="padding: 6px 8px;">{row.description}</td>
                                                                    <td style="padding: 6px 8px; font-family: monospace; color: #666;">
                                                                        {row.field_id}
                                                                    </td>
                                                                    <td style="padding: 6px 8px; font-weight: 500;">
                                                                        {row.value}
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
                                } else {
                                    let json_text = data
                                        .get()
                                        .and_then(|d| d.extra)
                                        .map(|json_str| {
                                            serde_json::from_str::<serde_json::Value>(&json_str)
                                                .ok()
                                                .and_then(|v| serde_json::to_string_pretty(&v).ok())
                                                .unwrap_or(json_str)
                                        })
                                        .unwrap_or_else(|| "No JSON data available".to_string());
                                    view! {
                                        <div style="max-height: calc(100vh - 200px); overflow-y: auto;">
                                            <pre style="background: #f5f5f5; padding: 12px; border-radius: 4px; font-size: var(--font-size-sm); font-family: monospace; white-space: pre-wrap; word-wrap: break-word;">
                                                {json_text}
                                            </pre>
                                        </div>
                                    }
                                        .into_any()
                                }
                            }}

                        </div>
                    }
                        .into_any()
                } else {
                    view! { <p>"No data"</p> }.into_any()
                }
            }}
            </div>
        </div>
    }
}

async fn fetch_detail(rr_dt: &str, rrd_id: i64) -> Result<WbFinanceReportDetailResponse, String> {
    let window = web_sys::window().ok_or("No window object")?;
    let url = format!("/api/p903/finance-report/{}/{}", rr_dt, rrd_id);

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
