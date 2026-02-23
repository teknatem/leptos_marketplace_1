use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator};
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use thaw::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmPaymentReportDetailDto {
    pub record_key: String,
    pub connection_mp_ref: String,
    pub organization_ref: String,
    pub business_id: Option<i64>,
    pub partner_id: Option<i64>,
    pub shop_name: Option<String>,
    pub inn: Option<String>,
    pub model: Option<String>,
    pub transaction_id: Option<String>,
    pub transaction_date: Option<String>,
    pub transaction_type: Option<String>,
    pub transaction_source: Option<String>,
    pub transaction_sum: Option<f64>,
    pub payment_status: Option<String>,
    pub order_id: Option<i64>,
    pub shop_order_id: Option<String>,
    pub order_creation_date: Option<String>,
    pub order_delivery_date: Option<String>,
    pub order_type: Option<String>,
    pub shop_sku: Option<String>,
    pub offer_or_service_name: Option<String>,
    pub count: Option<i32>,
    pub act_id: Option<i64>,
    pub act_date: Option<String>,
    pub bank_order_id: Option<i64>,
    pub bank_order_date: Option<String>,
    pub bank_sum: Option<f64>,
    pub claim_number: Option<String>,
    pub bonus_account_year_month: Option<String>,
    pub comments: Option<String>,
    pub loaded_at_utc: String,
    pub payload_version: i32,
}

#[derive(Debug, Clone)]
struct FieldRow {
    description: String,
    field_id: String,
    value: String,
}

#[component]
pub fn YmPaymentReportDetail(
    record_key: String,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let (data, set_data) = signal::<Option<YmPaymentReportDetailDto>>(None);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(None::<String>);
    let (sort_by, set_sort_by) = signal("description".to_string());
    let (sort_desc, set_sort_desc) = signal(false);

    let record_key_clone = record_key.clone();
    Effect::new(move || {
        let rk = record_key_clone.clone();
        spawn_local(async move {
            match fetch_detail(&rk).await {
                Ok(dto) => {
                    set_data.set(Some(dto));
                    set_loading.set(false);
                }
                Err(e) => {
                    log!("Failed to fetch YM payment report detail: {:?}", e);
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    });

    let get_field_rows = move || -> Vec<FieldRow> {
        let Some(item) = data.get() else {
            return Vec::new();
        };

        let mut rows = vec![
            FieldRow {
                description: "Ключ записи".to_string(),
                field_id: "record_key".to_string(),
                value: item.record_key.clone(),
            },
            FieldRow {
                description: "ID транзакции (ЯМ)".to_string(),
                field_id: "transaction_id".to_string(),
                value: item.transaction_id.clone().unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Дата транзакции".to_string(),
                field_id: "transaction_date".to_string(),
                value: item
                    .transaction_date
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Тип транзакции".to_string(),
                field_id: "transaction_type".to_string(),
                value: item
                    .transaction_type
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Источник транзакции".to_string(),
                field_id: "transaction_source".to_string(),
                value: item
                    .transaction_source
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Сумма транзакции".to_string(),
                field_id: "transaction_sum".to_string(),
                value: item
                    .transaction_sum
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Статус платежа".to_string(),
                field_id: "payment_status".to_string(),
                value: item
                    .payment_status
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "ID заказа".to_string(),
                field_id: "order_id".to_string(),
                value: item
                    .order_id
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "ID заказа магазина".to_string(),
                field_id: "shop_order_id".to_string(),
                value: item
                    .shop_order_id
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Дата создания заказа".to_string(),
                field_id: "order_creation_date".to_string(),
                value: item
                    .order_creation_date
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Дата доставки заказа".to_string(),
                field_id: "order_delivery_date".to_string(),
                value: item
                    .order_delivery_date
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Тип заказа".to_string(),
                field_id: "order_type".to_string(),
                value: item.order_type.clone().unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "SKU магазина".to_string(),
                field_id: "shop_sku".to_string(),
                value: item.shop_sku.clone().unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Наименование товара / услуги".to_string(),
                field_id: "offer_or_service_name".to_string(),
                value: item
                    .offer_or_service_name
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Количество".to_string(),
                field_id: "count".to_string(),
                value: item
                    .count
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "ID акта".to_string(),
                field_id: "act_id".to_string(),
                value: item
                    .act_id
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Дата акта".to_string(),
                field_id: "act_date".to_string(),
                value: item.act_date.clone().unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "ID банковского ордера".to_string(),
                field_id: "bank_order_id".to_string(),
                value: item
                    .bank_order_id
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Дата банковского ордера".to_string(),
                field_id: "bank_order_date".to_string(),
                value: item
                    .bank_order_date
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Сумма ПП (банковский перевод)".to_string(),
                field_id: "bank_sum".to_string(),
                value: item
                    .bank_sum
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Номер претензии".to_string(),
                field_id: "claim_number".to_string(),
                value: item
                    .claim_number
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Год-месяц бонусного счёта".to_string(),
                field_id: "bonus_account_year_month".to_string(),
                value: item
                    .bonus_account_year_month
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Комментарий".to_string(),
                field_id: "comments".to_string(),
                value: item.comments.clone().unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "ID бизнеса".to_string(),
                field_id: "business_id".to_string(),
                value: item
                    .business_id
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "ID партнёра".to_string(),
                field_id: "partner_id".to_string(),
                value: item
                    .partner_id
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Название магазина".to_string(),
                field_id: "shop_name".to_string(),
                value: item.shop_name.clone().unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "ИНН".to_string(),
                field_id: "inn".to_string(),
                value: item.inn.clone().unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Модель работы".to_string(),
                field_id: "model".to_string(),
                value: item.model.clone().unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Подключение МП".to_string(),
                field_id: "connection_mp_ref".to_string(),
                value: item.connection_mp_ref.clone(),
            },
            FieldRow {
                description: "Организация".to_string(),
                field_id: "organization_ref".to_string(),
                value: item.organization_ref.clone(),
            },
            FieldRow {
                description: "Загружено (UTC)".to_string(),
                field_id: "loaded_at_utc".to_string(),
                value: item.loaded_at_utc.clone(),
            },
            FieldRow {
                description: "Версия payload".to_string(),
                field_id: "payload_version".to_string(),
                value: item.payload_version.to_string(),
            },
        ];

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

    let export_to_excel = move || {
        let field_rows = get_field_rows();
        if field_rows.is_empty() {
            return;
        }

        let mut csv = String::from("\u{FEFF}");
        csv.push_str("Описание;Идентификатор;Значение\n");

        for row in field_rows {
            csv.push_str(&format!(
                "\"{}\";\"{}\";\"{}\"\n",
                row.description.replace('\"', "\"\""),
                row.field_id.replace('\"', "\"\""),
                row.value.replace('\"', "\"\"")
            ));
        }

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
                                "ym_payment_report_detail_{}.csv",
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

    let title = record_key.clone();

    view! {
        <div class="page page--detail">
            <div class="modal-header">
                <h3 class="modal-title">"ЯМ Платёж: " {title}</h3>
                <Button
                    appearance=ButtonAppearance::Secondary
                    on_click=move |_| on_close.run(())
                >
                    <span>{vec![icon("x").into_view()]}</span>
                    <span>" Закрыть"</span>
                </Button>
            </div>

            <div class="page__content">
                {move || {
                    if loading.get() {
                        view! { <p class="text-muted">"Загрузка..."</p> }.into_any()
                    } else if let Some(err) = error.get() {
                        view! {
                            <div class="warning-box warning-box--error">
                                <span class="warning-box__icon">"⚠"</span>
                                <span class="warning-box__text">{err}</span>
                            </div>
                        }.into_any()
                    } else if data.get().is_some() {
                        let export_excel = export_to_excel.clone();
                        view! {
                            <div>
                                <div style="display: flex; justify-content: flex-end; margin-bottom: var(--spacing-md);">
                                    <Button
                                        appearance=ButtonAppearance::Primary
                                        on_click=move |_| export_excel()
                                    >
                                        <span>{vec![icon("download").into_view()]}</span>
                                        <span>" Export Excel"</span>
                                    </Button>
                                </div>

                                <div style="width: 100%; overflow-x: auto;">
                                    <Table attr:style="width: 100%;">
                                        <TableHeader>
                                            <TableRow>
                                                <TableHeaderCell resizable=true min_width=300.0>
                                                    "Описание"
                                                    <span
                                                        class={move || get_sort_class("description", &sort_by.get())}
                                                        on:click=move |_| handle_column_sort("description")
                                                    >
                                                        {move || get_sort_indicator("description", &sort_by.get(), !sort_desc.get())}
                                                    </span>
                                                </TableHeaderCell>
                                                <TableHeaderCell resizable=true min_width=200.0>
                                                    "Идентификатор"
                                                    <span
                                                        class={move || get_sort_class("field_id", &sort_by.get())}
                                                        on:click=move |_| handle_column_sort("field_id")
                                                    >
                                                        {move || get_sort_indicator("field_id", &sort_by.get(), !sort_desc.get())}
                                                    </span>
                                                </TableHeaderCell>
                                                <TableHeaderCell resizable=true min_width=200.0>
                                                    "Значение"
                                                    <span
                                                        class={move || get_sort_class("value", &sort_by.get())}
                                                        on:click=move |_| handle_column_sort("value")
                                                    >
                                                        {move || get_sort_indicator("value", &sort_by.get(), !sort_desc.get())}
                                                    </span>
                                                </TableHeaderCell>
                                            </TableRow>
                                        </TableHeader>
                                        <TableBody>
                                            {get_field_rows()
                                                .into_iter()
                                                .map(|row| {
                                                    view! {
                                                        <TableRow>
                                                            <TableCell>
                                                                <TableCellLayout>{row.description}</TableCellLayout>
                                                            </TableCell>
                                                            <TableCell>
                                                                <TableCellLayout>
                                                                    <span style="font-family: monospace; font-size: 0.85em;">{row.field_id}</span>
                                                                </TableCellLayout>
                                                            </TableCell>
                                                            <TableCell>
                                                                <TableCellLayout>{row.value}</TableCellLayout>
                                                            </TableCell>
                                                        </TableRow>
                                                    }.into_view()
                                                })
                                                .collect_view()}
                                        </TableBody>
                                    </Table>
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! { <p>"Нет данных"</p> }.into_any()
                    }
                }}
            </div>
        </div>
    }
}

async fn fetch_detail(record_key: &str) -> Result<YmPaymentReportDetailDto, String> {
    let window = web_sys::window().ok_or("No window object")?;
    let url = format!(
        "/api/p907/payment-report/{}",
        js_sys::encode_uri_component(record_key)
            .as_string()
            .unwrap_or_default()
    );

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
