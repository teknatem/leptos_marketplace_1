use leptos::prelude::*;
use leptos::logging::log;
use serde::{Deserialize, Serialize};
use gloo_net::http::Request;

// DTO структуры для детального представления
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonTransactionsDetailDto {
    pub id: String,
    pub code: String,
    pub description: String,
    pub header: HeaderDto,
    pub posting: PostingDto,
    pub items: Vec<ItemDto>,
    pub services: Vec<ServiceDto>,
    #[serde(rename = "is_posted")]
    pub is_posted: bool,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "updated_at")]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderDto {
    pub operation_id: i64,
    pub operation_type: String,
    pub operation_date: String,
    pub operation_type_name: String,
    pub delivery_charge: f64,
    pub return_delivery_charge: f64,
    pub accruals_for_sale: f64,
    pub sale_commission: f64,
    pub amount: f64,
    pub transaction_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostingDto {
    pub delivery_schema: String,
    pub order_date: String,
    pub posting_number: String,
    pub warehouse_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDto {
    pub name: String,
    pub sku: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDto {
    pub name: String,
    pub price: f64,
}

/// Форматирует дату из "2025-10-11 00:00:00" в dd.mm.yyyy HH:MM
fn format_datetime(date_str: &str) -> String {
    // Парсим формат "2025-10-11 00:00:00"
    let parts: Vec<&str> = date_str.split_whitespace().collect();
    if parts.len() >= 2 {
        let date_part = parts[0];
        let time_part = parts[1];
        if let Some((year, rest)) = date_part.split_once('-') {
            if let Some((month, day)) = rest.split_once('-') {
                let time_short: String = time_part.chars().take(5).collect(); // HH:MM
                return format!("{}.{}.{} {}", day, month, year, time_short);
            }
        }
    }
    date_str.to_string() // fallback
}

#[component]
pub fn OzonTransactionsDetail(
    transaction_id: String,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let (transaction_data, set_transaction_data) = signal::<Option<OzonTransactionsDetailDto>>(None);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal::<Option<String>>(None);
    let (active_tab, set_active_tab) = signal("general");

    let transaction_id_for_effect = transaction_id.clone();

    // Загрузить детальные данные
    Effect::new(move || {
        let id = transaction_id_for_effect.clone();
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let url = format!("http://localhost:3000/api/ozon_transactions/{}", id);

            match Request::get(&url).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status == 200 {
                        match response.text().await {
                            Ok(text) => {
                                match serde_json::from_str::<OzonTransactionsDetailDto>(&text) {
                                    Ok(data) => {
                                        set_transaction_data.set(Some(data));
                                        set_loading.set(false);
                                    }
                                    Err(e) => {
                                        log!("Failed to parse transaction detail: {:?}", e);
                                        set_error.set(Some(format!("Ошибка парсинга: {}", e)));
                                        set_loading.set(false);
                                    }
                                }
                            }
                            Err(e) => {
                                log!("Failed to read response: {:?}", e);
                                set_error.set(Some(format!("Ошибка чтения ответа: {}", e)));
                                set_loading.set(false);
                            }
                        }
                    } else {
                        set_error.set(Some(format!("Ошибка сервера: {}", status)));
                        set_loading.set(false);
                    }
                }
                Err(e) => {
                    log!("Failed to fetch transaction: {:?}", e);
                    set_error.set(Some(format!("Ошибка загрузки: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    });

    view! {
        <div class="modal-overlay" on:click=move |_| on_close.run(())>
            <div class="modal-content-wide" on:click=move |e| e.stop_propagation() style="display: flex; flex-direction: column; max-height: 90vh;">
                <div style="padding: 20px; border-bottom: 1px solid #ddd; display: flex; justify-content: space-between; align-items: center; flex-shrink: 0;">
                    <h3 style="margin: 0; font-size: 20px; font-weight: 600; color: #333;">"Детали транзакции OZON"</h3>
                    <button
                        on:click=move |_| on_close.run(())
                        style="background: none; border: none; font-size: 24px; cursor: pointer; color: #999; line-height: 1; padding: 0; width: 30px; height: 30px;"
                    >
                        "✕"
                    </button>
                </div>

                <div style="flex: 1; overflow-y: auto; min-height: 0;">
                    {move || {
                        if loading.get() {
                            view! {
                                <div style="text-align: center; padding: 40px;">
                                    <p>"Загрузка..."</p>
                                </div>
                            }.into_any()
                        } else if let Some(err) = error.get() {
                            view! {
                                <div style="padding: 20px; background: #ffebee; border: 1px solid #ffcdd2; border-radius: 4px; color: #c62828; margin: 20px;">
                                    <strong>"Ошибка: "</strong>{err}
                                </div>
                            }.into_any()
                        } else if let Some(data) = transaction_data.get() {
                            view! {
                                <div style="height: 100%; display: flex; flex-direction: column;">
                                    <div class="tabs" style="border-bottom: 2px solid #ddd; margin-bottom: 20px; flex-shrink: 0; background: white; position: sticky; top: 0; z-index: 10;">
                                        <button
                                            on:click=move |_| set_active_tab.set("general")
                                            style=move || format!(
                                                "padding: 10px 20px; border: none; border-radius: 4px 4px 0 0; cursor: pointer; margin-right: 5px; font-weight: 500; {}",
                                                if active_tab.get() == "general" {
                                                    "background: #2196F3; color: white; border-bottom: 2px solid #2196F3;"
                                                } else {
                                                    "background: #f5f5f5; color: #666;"
                                                }
                                            )
                                        >
                                            "Общие данные"
                                        </button>
                                        <button
                                            on:click=move |_| set_active_tab.set("items")
                                            style=move || format!(
                                                "padding: 10px 20px; border: none; border-radius: 4px 4px 0 0; cursor: pointer; margin-right: 5px; font-weight: 500; {}",
                                                if active_tab.get() == "items" {
                                                    "background: #2196F3; color: white; border-bottom: 2px solid #2196F3;"
                                                } else {
                                                    "background: #f5f5f5; color: #666;"
                                                }
                                            )
                                        >
                                            "Товары (" {data.items.len()} ")"
                                        </button>
                                        <button
                                            on:click=move |_| set_active_tab.set("services")
                                            style=move || format!(
                                                "padding: 10px 20px; border: none; border-radius: 4px 4px 0 0; cursor: pointer; margin-right: 5px; font-weight: 500; {}",
                                                if active_tab.get() == "services" {
                                                    "background: #2196F3; color: white; border-bottom: 2px solid #2196F3;"
                                                } else {
                                                    "background: #f5f5f5; color: #666;"
                                                }
                                            )
                                        >
                                            "Сервисы (" {data.services.len()} ")"
                                        </button>
                                    </div>

                                    <div style="flex: 1; overflow-y: auto; padding: 20px; background: #fafafa;">
                                        {move || {
                                            let data = transaction_data.get().unwrap();
                                            let tab = active_tab.get();
                                            match tab.as_ref() {
                                                "general" => view! {
                                                    <div style="display: flex; flex-direction: column; gap: 20px;">
                                                        <div style="background: white; padding: 20px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                                                            <h3 style="margin: 0 0 15px 0; color: #333; font-size: 16px; font-weight: 600; border-bottom: 2px solid #2196F3; padding-bottom: 8px;">"Заголовок транзакции"</h3>
                                                            <div style="display: grid; grid-template-columns: 200px 1fr; gap: 15px 20px; align-items: center;">
                                                                <div style="font-weight: 600; color: #555;">"Operation ID:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{data.header.operation_id}</div>

                                                                <div style="font-weight: 600; color: #555;">"Тип операции:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">
                                                                    <span style="padding: 2px 8px; background: #e3f2fd; color: #1976d2; border-radius: 3px; font-weight: 500;">
                                                                        {data.header.operation_type_name.clone()}
                                                                    </span>
                                                                </div>

                                                                <div style="font-weight: 600; color: #555;">"Дата операции:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{format_datetime(&data.header.operation_date)}</div>

                                                                <div style="font-weight: 600; color: #555;">"Тип транзакции:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{data.header.transaction_type.clone()}</div>

                                                                <div style="font-weight: 600; color: #555;">"Сумма:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 18px;">
                                                                    <span style=move || format!(
                                                                        "font-weight: 600; {}",
                                                                        if data.header.amount >= 0.0 {
                                                                            "color: #4caf50;"
                                                                        } else {
                                                                            "color: #f44336;"
                                                                        }
                                                                    )>
                                                                        {format!("{:.2} ₽", data.header.amount)}
                                                                    </span>
                                                                </div>

                                                                <div style="font-weight: 600; color: #555;">"Начисления за продажу:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{format!("{:.2} ₽", data.header.accruals_for_sale)}</div>

                                                                <div style="font-weight: 600; color: #555;">"Комиссия за продажу:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{format!("{:.2} ₽", data.header.sale_commission)}</div>

                                                                <div style="font-weight: 600; color: #555;">"Стоимость доставки:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{format!("{:.2} ₽", data.header.delivery_charge)}</div>
                                                            </div>
                                                        </div>

                                                        <div style="background: white; padding: 20px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                                                            <h3 style="margin: 0 0 15px 0; color: #333; font-size: 16px; font-weight: 600; border-bottom: 2px solid #2196F3; padding-bottom: 8px;">"Информация о постинге"</h3>
                                                            <div style="display: grid; grid-template-columns: 200px 1fr; gap: 15px 20px; align-items: center;">
                                                                <div style="font-weight: 600; color: #555;">"Posting Number:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">
                                                                    <span style="color: #2196F3; font-weight: 500; cursor: pointer;">{data.posting.posting_number.clone()}</span>
                                                                </div>

                                                                <div style="font-weight: 600; color: #555;">"Схема доставки:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{data.posting.delivery_schema.clone()}</div>

                                                                <div style="font-weight: 600; color: #555;">"Дата заказа:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{format_datetime(&data.posting.order_date)}</div>

                                                                <div style="font-weight: 600; color: #555;">"Warehouse ID:"</div>
                                                                <div style="font-family: 'Segoe UI', system-ui, sans-serif; font-size: 14px;">{data.posting.warehouse_id}</div>
                                                            </div>
                                                        </div>
                                                    </div>
                                                }.into_any(),
                                                "items" => view! {
                                                    <div style="background: white; padding: 20px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                                                        <h3 style="margin: 0 0 15px 0; color: #333; font-size: 16px; font-weight: 600; border-bottom: 2px solid #2196F3; padding-bottom: 8px;">"Товары"</h3>
                                                        {if data.items.is_empty() {
                                                            view! {
                                                                <p style="text-align: center; padding: 40px; color: #999;">"Нет товаров"</p>
                                                            }.into_any()
                                                        } else {
                                                            view! {
                                                                <table style="width: 100%; border-collapse: collapse;">
                                                                    <thead>
                                                                        <tr style="background: #f5f5f5;">
                                                                            <th style="padding: 12px; text-align: left; font-weight: 600; border-bottom: 2px solid #ddd; color: #555;">"SKU"</th>
                                                                            <th style="padding: 12px; text-align: left; font-weight: 600; border-bottom: 2px solid #ddd; color: #555;">"Название"</th>
                                                                        </tr>
                                                                    </thead>
                                                                    <tbody>
                                                                        {data.items.iter().map(|item| view! {
                                                                            <tr style="border-bottom: 1px solid #eee;">
                                                                                <td style="padding: 12px; font-family: 'Courier New', monospace; color: #333;">{item.sku}</td>
                                                                                <td style="padding: 12px; color: #333;">{item.name.clone()}</td>
                                                                            </tr>
                                                                        }).collect::<Vec<_>>()}
                                                                    </tbody>
                                                                </table>
                                                            }.into_any()
                                                        }}
                                                    </div>
                                                }.into_any(),
                                                "services" => view! {
                                                    <div style="background: white; padding: 20px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                                                        <h3 style="margin: 0 0 15px 0; color: #333; font-size: 16px; font-weight: 600; border-bottom: 2px solid #4caf50; padding-bottom: 8px;">"Сервисы"</h3>
                                                        {if data.services.is_empty() {
                                                            view! {
                                                                <p style="text-align: center; padding: 40px; color: #999;">"Нет сервисов"</p>
                                                            }.into_any()
                                                        } else {
                                                            view! {
                                                                <table style="width: 100%; border-collapse: collapse;">
                                                                    <thead>
                                                                        <tr style="background: #f5f5f5;">
                                                                            <th style="padding: 12px; text-align: left; font-weight: 600; border-bottom: 2px solid #ddd; color: #555;">"Название"</th>
                                                                            <th style="padding: 12px; text-align: right; font-weight: 600; border-bottom: 2px solid #ddd; color: #555;">"Цена"</th>
                                                                        </tr>
                                                                    </thead>
                                                                    <tbody>
                                                                        {data.services.iter().map(|service| view! {
                                                                            <tr style="border-bottom: 1px solid #eee;">
                                                                                <td style="padding: 12px; color: #333;">{service.name.clone()}</td>
                                                                                <td style="padding: 12px; text-align: right; font-weight: 600; color: #4caf50;">{format!("{:.2} ₽", service.price)}</td>
                                                                            </tr>
                                                                        }).collect::<Vec<_>>()}
                                                                    </tbody>
                                                                </table>
                                                            }.into_any()
                                                        }}
                                                    </div>
                                                }.into_any(),
                                                _ => view! {
                                                    <div>"Unknown tab"</div>
                                                }.into_any()
                                            }
                                        }}
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div>"Нет данных"</div>
                            }.into_any()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}
