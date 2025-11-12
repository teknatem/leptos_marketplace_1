use leptos::prelude::*;
use leptos::logging::log;
use serde::{Deserialize, Serialize};
use gloo_net::http::Request;
use crate::shared::icons::icon;

// DTO структуры для детального представления
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonReturnsDetailDto {
    pub id: String,
    pub code: String,
    pub description: String,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "organizationId")]
    pub organization_id: String,
    #[serde(rename = "marketplaceId")]
    pub marketplace_id: String,
    #[serde(rename = "returnId")]
    pub return_id: String,
    #[serde(rename = "returnDate")]
    pub return_date: String,
    #[serde(rename = "returnReasonName")]
    pub return_reason_name: String,
    #[serde(rename = "returnType")]
    pub return_type: String,
    #[serde(rename = "orderId")]
    pub order_id: String,
    #[serde(rename = "orderNumber")]
    pub order_number: String,
    pub sku: String,
    #[serde(rename = "productName")]
    pub product_name: String,
    pub price: f64,
    pub quantity: i32,
    #[serde(rename = "postingNumber")]
    pub posting_number: String,
    #[serde(rename = "clearingId")]
    pub clearing_id: Option<String>,
    #[serde(rename = "returnClearingId")]
    pub return_clearing_id: Option<String>,
    pub comment: Option<String>,
    pub metadata: MetadataDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataDto {
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "isDeleted")]
    pub is_deleted: bool,
    #[serde(rename = "isPosted")]
    pub is_posted: bool,
    pub version: i32,
}

#[component]
pub fn OzonReturnsDetail(
    id: String,
    #[prop(into)] on_close: Callback<()>,
    #[prop(optional)] reload_trigger: Option<ReadSignal<u32>>,
) -> impl IntoView {
    let (return_data, set_return_data) = signal::<Option<OzonReturnsDetailDto>>(None);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal::<Option<String>>(None);
    let (active_tab, set_active_tab) = signal("general");

    // Загрузить детальные данные
    Effect::new(move || {
        // Отслеживаем reload_trigger если передан
        if let Some(trigger) = reload_trigger {
            let _ = trigger.get();
        }

        let id = id.clone();
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let url = format!("http://localhost:3000/api/ozon_returns/{}", id);

            match Request::get(&url).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status == 200 {
                        match response.text().await {
                            Ok(text) => {
                                match serde_json::from_str::<OzonReturnsDetailDto>(&text) {
                                    Ok(data) => {
                                        set_return_data.set(Some(data));
                                        set_loading.set(false);
                                    }
                                    Err(e) => {
                                        log!("Failed to parse return detail: {:?}", e);
                                        set_error.set(Some(format!("Ошибка парсинга: {}", e)));
                                        set_loading.set(false);
                                    }
                                }
                            }
                            Err(e) => {
                                log!("Failed to get text from response: {:?}", e);
                                set_error.set(Some(format!("Ошибка чтения ответа: {}", e)));
                                set_loading.set(false);
                            }
                        }
                    } else {
                        log!("Failed to load return detail, status: {}", status);
                        set_error.set(Some(format!("HTTP {}", status)));
                        set_loading.set(false);
                    }
                }
                Err(e) => {
                    log!("Failed to send request: {:?}", e);
                    set_error.set(Some(format!("Ошибка сети: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    });

    view! {
        <div class="ozon-returns-detail" style="padding: 20px; height: 100%; display: flex; flex-direction: column;">
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; flex-shrink: 0;">
                <h2 style="margin: 0;">"Возврат OZON"</h2>
                <button
                    on:click=move |_| on_close.run(())
                    style="padding: 8px 16px; background: #f44336; color: white; border: none; border-radius: 4px; cursor: pointer;"
                >
                    "✕ Закрыть"
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
                            <div style="padding: 20px; background: #ffebee; border: 1px solid #ffcdd2; border-radius: 4px; color: #c62828;">
                                <strong>"Ошибка: "</strong>{err}
                            </div>
                        }.into_any()
                    } else if let Some(data) = return_data.get() {
                        view! {
                            <div style="height: 100%; display: flex; flex-direction: column;">
                                // Tabs
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
                                        "Основное"
                                    </button>
                                    <button
                                        on:click=move |_| set_active_tab.set("product")
                                        style=move || format!(
                                            "padding: 10px 20px; border: none; border-radius: 4px 4px 0 0; cursor: pointer; margin-right: 5px; font-weight: 500; {}",
                                            if active_tab.get() == "product" {
                                                "background: #2196F3; color: white; border-bottom: 2px solid #2196F3;"
                                            } else {
                                                "background: #f5f5f5; color: #666;"
                                            }
                                        )
                                    >
                                        "Товар"
                                    </button>
                                    <button
                                        on:click=move |_| set_active_tab.set("metadata")
                                        style=move || format!(
                                            "padding: 10px 20px; border: none; border-radius: 4px 4px 0 0; cursor: pointer; margin-right: 5px; font-weight: 500; {}",
                                            if active_tab.get() == "metadata" {
                                                "background: #2196F3; color: white; border-bottom: 2px solid #2196F3;"
                                            } else {
                                                "background: #f5f5f5; color: #666;"
                                            }
                                        )
                                    >
                                        "Метаданные"
                                    </button>
                                </div>

                                // Tab content
                                <div style="flex: 1; overflow-y: auto; padding: 20px; background: #fafafa;">
                                    {move || {
                                        let tab = active_tab.get();
                                        let data = data.clone();
                                        match tab {
                                            "general" => render_general_tab(data).into_any(),
                                            "product" => render_product_tab(data).into_any(),
                                            "metadata" => render_metadata_tab(data).into_any(),
                                            _ => view! { <div></div> }.into_any(),
                                        }
                                    }}
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }
                }}
            </div>
        </div>
    }
}

// Вкладка "Основное"
fn render_general_tab(data: OzonReturnsDetailDto) -> impl IntoView {
    let total_amount = data.price * data.quantity as f64;

    view! {
        <div style="display: flex; flex-direction: column; gap: 20px;">
            <div style="background: white; padding: 20px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                <h3 style="margin: 0 0 15px 0; color: #333; font-size: 16px; font-weight: 600; border-bottom: 2px solid #2196F3; padding-bottom: 8px;">"Информация о возврате"</h3>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"ID возврата:"</label>
                    <span style="color: #333;">{data.return_id.clone()}</span>
                </div>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"Дата возврата:"</label>
                    <span style="color: #333;">{data.return_date.clone()}</span>
                </div>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"Тип возврата:"</label>
                    <span style="display: inline-block; padding: 4px 12px; background: #e3f2fd; color: #1976d2; border-radius: 12px; font-size: 13px;">{data.return_type.clone()}</span>
                </div>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"Причина возврата:"</label>
                    <span style="color: #333;">{data.return_reason_name.clone()}</span>
                </div>
            </div>

            <div style="background: white; padding: 20px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                <h3 style="margin: 0 0 15px 0; color: #333; font-size: 16px; font-weight: 600; border-bottom: 2px solid #2196F3; padding-bottom: 8px;">"Информация о заказе"</h3>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"ID заказа:"</label>
                    <span style="color: #333;">{data.order_id.clone()}</span>
                </div>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"Номер заказа:"</label>
                    <span style="color: #333;">{data.order_number.clone()}</span>
                </div>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"Номер отправления:"</label>
                    <span style="color: #333;">{data.posting_number.clone()}</span>
                </div>
            </div>

            <div style="background: white; padding: 20px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                <h3 style="margin: 0 0 15px 0; color: #333; font-size: 16px; font-weight: 600; border-bottom: 2px solid #4caf50; padding-bottom: 8px;">"Финансовая информация"</h3>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"Сумма возврата:"</label>
                    <span style="color: #4caf50; font-weight: 600; font-size: 18px;">{format!("{:.2} ₽", total_amount)}</span>
                </div>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"Clearing ID:"</label>
                    <span style="color: #333;">{data.clearing_id.clone().unwrap_or_else(|| "-".to_string())}</span>
                </div>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"Return Clearing ID:"</label>
                    <span style="color: #333;">{data.return_clearing_id.clone().unwrap_or_else(|| "-".to_string())}</span>
                </div>
            </div>

            <div style="background: white; padding: 20px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                <h3 style="margin: 0 0 15px 0; color: #333; font-size: 16px; font-weight: 600; border-bottom: 2px solid #ff9800; padding-bottom: 8px;">"UUID связей"</h3>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"Подключение:"</label>
                    <div style="display: flex; gap: 8px; align-items: center;">
                        <code style="font-size: 12px; background: #f5f5f5; padding: 4px 8px; border-radius: 4px; font-family: monospace;">{data.connection_id.clone()}</code>
                        <button
                            on:click=move |_| {
                                if let Some(window) = web_sys::window() {
                                    let clipboard = window.navigator().clipboard();
                                    let text = data.connection_id.clone();
                                    let _ = clipboard.write_text(&text);
                                }
                            }
                            style="padding: 4px 8px; background: #2196F3; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 12px;"
                        >
                            {icon("copy")}
                        </button>
                    </div>
                </div>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"Организация:"</label>
                    <div style="display: flex; gap: 8px; align-items: center;">
                        <code style="font-size: 12px; background: #f5f5f5; padding: 4px 8px; border-radius: 4px; font-family: monospace;">{data.organization_id.clone()}</code>
                        <button
                            on:click=move |_| {
                                if let Some(window) = web_sys::window() {
                                    let clipboard = window.navigator().clipboard();
                                    let text = data.organization_id.clone();
                                    let _ = clipboard.write_text(&text);
                                }
                            }
                            style="padding: 4px 8px; background: #2196F3; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 12px;"
                        >
                            {icon("copy")}
                        </button>
                    </div>
                </div>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"Маркетплейс:"</label>
                    <div style="display: flex; gap: 8px; align-items: center;">
                        <code style="font-size: 12px; background: #f5f5f5; padding: 4px 8px; border-radius: 4px; font-family: monospace;">{data.marketplace_id.clone()}</code>
                        <button
                            on:click=move |_| {
                                if let Some(window) = web_sys::window() {
                                    let clipboard = window.navigator().clipboard();
                                    let text = data.marketplace_id.clone();
                                    let _ = clipboard.write_text(&text);
                                }
                            }
                            style="padding: 4px 8px; background: #2196F3; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 12px;"
                        >
                            {icon("copy")}
                        </button>
                    </div>
                </div>
            </div>

            {data.comment.clone().map(|comment| {
                if !comment.is_empty() {
                    view! {
                        <div style="background: white; padding: 20px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                            <h3 style="margin: 0 0 15px 0; color: #333; font-size: 16px; font-weight: 600; border-bottom: 2px solid #9c27b0; padding-bottom: 8px;">"Комментарий"</h3>
                            <p style="color: #555; line-height: 1.6; margin: 0;">{comment}</p>
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            })}
        </div>
    }
}

// Вкладка "Товар"
fn render_product_tab(data: OzonReturnsDetailDto) -> impl IntoView {
    let total_amount = data.price * data.quantity as f64;

    view! {
        <div style="display: flex; flex-direction: column; gap: 20px;">
            <div style="background: white; padding: 20px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                <h3 style="margin: 0 0 15px 0; color: #333; font-size: 16px; font-weight: 600; border-bottom: 2px solid #2196F3; padding-bottom: 8px;">"Информация о товаре"</h3>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"SKU:"</label>
                    <span style="color: #333;">{data.sku.clone()}</span>
                </div>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"Название:"</label>
                    <span style="color: #333;">{data.product_name.clone()}</span>
                </div>
            </div>

            <div style="background: white; padding: 20px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                <h3 style="margin: 0 0 15px 0; color: #333; font-size: 16px; font-weight: 600; border-bottom: 2px solid #4caf50; padding-bottom: 8px;">"Количество и цена"</h3>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"Количество:"</label>
                    <span style="color: #333;">{data.quantity}</span>
                </div>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"Цена за единицу:"</label>
                    <span style="color: #333;">{format!("{:.2} ₽", data.price)}</span>
                </div>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"Общая сумма:"</label>
                    <span style="color: #4caf50; font-weight: 600; font-size: 18px;">{format!("{:.2} ₽", total_amount)}</span>
                </div>
            </div>
        </div>
    }
}

// Вкладка "Метаданные"
fn render_metadata_tab(data: OzonReturnsDetailDto) -> impl IntoView {
    view! {
        <div style="display: flex; flex-direction: column; gap: 20px;">
            <div style="background: white; padding: 20px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                <h3 style="margin: 0 0 15px 0; color: #333; font-size: 16px; font-weight: 600; border-bottom: 2px solid #2196F3; padding-bottom: 8px;">"Системная информация"</h3>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"ID записи:"</label>
                    <code style="font-size: 12px; background: #f5f5f5; padding: 4px 8px; border-radius: 4px; font-family: monospace;">{data.id.clone()}</code>
                </div>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"Код:"</label>
                    <span style="color: #333;">{data.code.clone()}</span>
                </div>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"Описание:"</label>
                    <span style="color: #333;">{data.description.clone()}</span>
                </div>
            </div>

            <div style="background: white; padding: 20px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                <h3 style="margin: 0 0 15px 0; color: #333; font-size: 16px; font-weight: 600; border-bottom: 2px solid #ff9800; padding-bottom: 8px;">"Временные метки"</h3>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"Создано:"</label>
                    <span style="color: #333;">{format_datetime(&data.metadata.created_at)}</span>
                </div>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"Обновлено:"</label>
                    <span style="color: #333;">{format_datetime(&data.metadata.updated_at)}</span>
                </div>
            </div>

            <div style="background: white; padding: 20px; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                <h3 style="margin: 0 0 15px 0; color: #333; font-size: 16px; font-weight: 600; border-bottom: 2px solid #9c27b0; padding-bottom: 8px;">"Статусы"</h3>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"Версия:"</label>
                    <span style="color: #333;">{data.metadata.version}</span>
                </div>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"Проведен:"</label>
                    <span style=move || {
                        if data.metadata.is_posted {
                            "display: inline-block; padding: 4px 12px; background: #c8e6c9; color: #2e7d32; border-radius: 12px; font-size: 13px;"
                        } else {
                            "display: inline-block; padding: 4px 12px; background: #e0e0e0; color: #616161; border-radius: 12px; font-size: 13px;"
                        }
                    }>
                        {if data.metadata.is_posted { "Да" } else { "Нет" }}
                    </span>
                </div>
                <div style="display: grid; grid-template-columns: 200px 1fr; gap: 12px; align-items: center;">
                    <label style="font-weight: 500; color: #666;">"Удален:"</label>
                    <span style=move || {
                        if data.metadata.is_deleted {
                            "display: inline-block; padding: 4px 12px; background: #ffcdd2; color: #c62828; border-radius: 12px; font-size: 13px;"
                        } else {
                            "display: inline-block; padding: 4px 12px; background: #c8e6c9; color: #2e7d32; border-radius: 12px; font-size: 13px;"
                        }
                    }>
                        {if data.metadata.is_deleted { "Да" } else { "Нет" }}
                    </span>
                </div>
            </div>
        </div>
    }
}

// Утилита для форматирования datetime
fn format_datetime(dt_str: &str) -> String {
    // Простое форматирование ISO 8601
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(dt_str) {
        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        dt_str.to_string()
    }
}
