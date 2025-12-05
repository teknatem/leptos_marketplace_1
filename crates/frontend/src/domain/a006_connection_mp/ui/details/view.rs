use crate::domain::a002_organization::ui::{OrganizationPicker, OrganizationPickerItem};
use crate::domain::a005_marketplace::ui::{MarketplacePicker, MarketplacePickerItem};
use crate::shared::picker_aggregate::{Modal, ModalService};
use crate::shared::icons::icon;
use contracts::domain::a006_connection_mp::{ConnectionMPDto, ConnectionTestResult};
use contracts::domain::common::AggregateId;
use leptos::prelude::*;
use std::rc::Rc;

#[component]
pub fn ConnectionMPDetails(
    id: Option<String>,
    on_saved: Rc<dyn Fn(())>,
    on_cancel: Rc<dyn Fn(())>,
) -> impl IntoView {
    let modal = use_context::<ModalService>().expect("ModalService not found");

    let (form, set_form) = signal(ConnectionMPDto::default());
    let (error, set_error) = signal::<Option<String>>(None);
    let (test_result, set_test_result) = signal::<Option<ConnectionTestResult>>(None);
    let (is_testing, set_is_testing) = signal(false);
    let (show_marketplace_picker, set_show_marketplace_picker) = signal(false);
    let (show_organization_picker, set_show_organization_picker) = signal(false);
    let (marketplace_name, set_marketplace_name) = signal(String::new());
    let (marketplace_code, set_marketplace_code) = signal(String::new());
    let (organization_name, set_organization_name) = signal(String::new());
    // Храним ID для предвыбора в пикерах
    let (organization_id, set_organization_id) = signal::<Option<String>>(None);

    // Load existing connection if id is provided
    if let Some(ref conn_id) = id {
        let id_clone = conn_id.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(conn) = fetch_connection(&id_clone).await {
                // Сохраняем organization в organization_name для отображения
                set_organization_name.set(conn.organization.clone());

                // Загружаем информацию о маркетплейсе
                if let Ok(mp_info) = fetch_marketplace_info(&conn.marketplace_id).await {
                    set_marketplace_name.set(mp_info.name);
                    set_marketplace_code.set(mp_info.code);
                }

                let dto = ConnectionMPDto {
                    id: Some(conn.base.id.as_string()),
                    code: Some(conn.base.code),
                    description: conn.base.description,
                    comment: conn.base.comment,
                    marketplace_id: conn.marketplace_id.clone(),
                    organization: conn.organization,
                    api_key: conn.api_key,
                    supplier_id: conn.supplier_id,
                    application_id: conn.application_id,
                    is_used: conn.is_used,
                    business_account_id: conn.business_account_id,
                    api_key_stats: conn.api_key_stats,
                    test_mode: conn.test_mode,
                    authorization_type: conn.authorization_type,
                };
                set_form.set(dto);
            }
        });
    }

    let handle_save = move |_| {
        let dto = form.get();
        let on_saved = on_saved.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match save_connection(dto).await {
                Ok(_) => on_saved(()),
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    let handle_test = move |_| {
        set_is_testing.set(true);
        set_test_result.set(None);
        let dto = form.get();
        wasm_bindgen_futures::spawn_local(async move {
            match test_connection(dto).await {
                Ok(result) => {
                    // Debug log для проверки что приходит с сервера
                    web_sys::console::log_1(&format!(
                        "Test result: success={}, message={}, details={:?}", 
                        result.success, result.message, result.details
                    ).into());
                    set_test_result.set(Some(result));
                    set_is_testing.set(false);
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("Test connection error: {}", e).into());
                    set_error.set(Some(e));
                    set_is_testing.set(false);
                }
            }
        });
    };

    let handle_marketplace_selected = move |selected: Option<MarketplacePickerItem>| {
        modal.hide();
        set_show_marketplace_picker.set(false);
        if let Some(item) = selected {
            set_marketplace_name.set(item.description.clone());
            set_form.update(|f| f.marketplace_id = item.id.clone());
        }
    };

    let handle_marketplace_cancel = move |_| {
        modal.hide();
        set_show_marketplace_picker.set(false);
    };

    let handle_organization_selected = move |selected: Option<OrganizationPickerItem>| {
        modal.hide();
        set_show_organization_picker.set(false);
        if let Some(item) = selected {
            set_organization_id.set(Some(item.id.clone()));
            set_organization_name.set(item.description.clone());
            set_form.update(|f| f.organization = item.description.clone());
        }
    };

    let handle_organization_cancel = move |_| {
        modal.hide();
        set_show_organization_picker.set(false);
    };

    view! {
        <div class="details-container connection-mp-details" style="max-width: 1200px;">
            <div class="details-header">
                <h3>{if id.is_some() { "Редактирование подключения" } else { "Новое подключение" }}</h3>
            </div>

            {move || error.get().map(|e| view! { <div class="error">{e}</div> })}

            <div class="details-form" style="display: grid; grid-template-columns: 1fr 1fr; gap: 12px; column-gap: 24px;">
                // Колонка 1
                <div class="form-group">
                    <label for="description">{"Наименование"}</label>
                    <input
                        type="text"
                        id="description"
                        prop:value={move || form.get().description}
                        on:input=move |ev| set_form.update(|f| f.description = event_target_value(&ev))
                        placeholder="Например: Озон (Сантехсистем)"
                    />
                </div>

                // Колонка 2
                <div class="form-group">
                    <label for="api_key">{"API Key"}</label>
                    <textarea
                        id="api_key"
                        prop:value={move || form.get().api_key}
                        on:input=move |ev| set_form.update(|f| f.api_key = event_target_value(&ev))
                        placeholder="Вставьте API ключ"
                        rows="3"
                    />
                    <small class="help-text">
                        {"• Wildberries: API ключ (Bearer token)"}<br/>
                        {"• Ozon: Api-Key (в связке с Client-Id)"}<br/>
                        {"• Яндекс.Маркет: OAuth токен (Bearer token)"}
                    </small>
                </div>

                // Колонка 1
                <div class="form-group">
                    <label for="marketplace">{"Маркетплейс"}</label>
                    <div style="display: flex; gap: 8px; align-items: center;">
                        <input
                            type="text"
                            id="marketplace"
                            prop:value={move || marketplace_name.get()}
                            readonly
                            placeholder="Выберите маркетплейс"
                            style="flex: 1;"
                        />
                        <button
                            type="button"
                            class="btn btn-secondary"
                            on:click=move |_| {
                                set_show_marketplace_picker.set(true);
                                modal.show();
                            }
                        >
                            {icon("search")}
                            {"Выбрать"}
                        </button>
                    </div>
                </div>

                // Колонка 2
                <div class="form-group">
                    <label for="supplier_id">{"ID Поставщика / Client ID"}</label>
                    <input
                        type="text"
                        id="supplier_id"
                        prop:value={move || form.get().supplier_id.clone().unwrap_or_default()}
                        on:input=move |ev| {
                            let val = event_target_value(&ev);
                            set_form.update(|f| f.supplier_id = if val.is_empty() { None } else { Some(val) });
                        }
                        placeholder="Для Озон"
                    />
                    <small class="help-text">
                        {"• Ozon: Client-Id (для всех API запросов)"}<br/>
                        {"• Яндекс.Маркет: Campaign ID / Идентификатор магазина (для заказов и возвратов)"}
                    </small>
                </div>

                // Колонка 1
                <div class="form-group">
                    <label for="organization">{"Организация"}</label>
                    <div style="display: flex; gap: 8px; align-items: center;">
                        <input
                            type="text"
                            id="organization"
                            prop:value={move || organization_name.get()}
                            readonly
                            placeholder="Выберите организацию"
                            style="flex: 1;"
                        />
                        <button
                            type="button"
                            class="btn btn-secondary"
                            on:click=move |_| {
                                set_show_organization_picker.set(true);
                                modal.show();
                            }
                        >
                            {icon("search")}
                            {"Выбрать"}
                        </button>
                    </div>
                </div>

                // Колонка 2
                <div class="form-group">
                    <label for="application_id">{"ID Приложения"}</label>
                    <input
                        type="text"
                        id="application_id"
                        prop:value={move || form.get().application_id.clone().unwrap_or_default()}
                        on:input=move |ev| {
                            let val = event_target_value(&ev);
                            set_form.update(|f| f.application_id = if val.is_empty() { None } else { Some(val) });
                        }
                    />
                    <small class="help-text">
                        {"• Ozon: Application ID / Client-Id (обязательно для всех запросов)"}
                    </small>
                </div>

                // Колонка 1
                <div class="form-group" style="display: flex; align-items: center; gap: 8px;">
                    <input
                        type="checkbox"
                        id="is_used"
                        prop:checked={move || form.get().is_used}
                        on:change=move |ev| set_form.update(|f| f.is_used = event_target_checked(&ev))
                    />
                    <label for="is_used" style="margin: 0; cursor: pointer;">{"Используется"}</label>
                </div>

                // Колонка 2
                <div class="form-group">
                    <label for="business_account_id">{"Бизнес Аккаунт ID"}</label>
                    <input
                        type="text"
                        id="business_account_id"
                        prop:value={move || form.get().business_account_id.clone().unwrap_or_default()}
                        on:input=move |ev| {
                            let val = event_target_value(&ev);
                            set_form.update(|f| f.business_account_id = if val.is_empty() { None } else { Some(val) });
                        }
                        placeholder="Для Яндекс.Маркет"
                    />
                    <small class="help-text">
                        {"• Яндекс.Маркет: Business ID / БизнесАккаунтID (для работы с товарами)"}
                    </small>
                </div>

                // Колонка 1
                <div class="form-group" style="display: flex; align-items: center; gap: 8px;">
                    <input
                        type="checkbox"
                        id="test_mode"
                        prop:checked={move || form.get().test_mode}
                        on:change=move |ev| set_form.update(|f| f.test_mode = event_target_checked(&ev))
                    />
                    <label for="test_mode" style="margin: 0; cursor: pointer;">{"Тестовый режим"}</label>
                </div>

                // Колонка 2 - пустое место
                <div></div>

                <div class="form-group" style="grid-column: 1 / -1;">
                    <label for="comment">{"Комментарий"}</label>
                    <textarea
                        id="comment"
                        prop:value={move || form.get().comment.clone().unwrap_or_default()}
                        on:input=move |ev| {
                            let val = event_target_value(&ev);
                            set_form.update(|f| f.comment = if val.is_empty() { None } else { Some(val) });
                        }
                        rows="2"
                    />
                </div>
            </div>

            {move || test_result.get().map(|result| {
                let class = if result.success { "success" } else { "error" };
                let mp_code = marketplace_code.get();
                view! {
                    <div class={class} style="margin-top: 16px; padding: 16px; border-radius: 8px;">
                        <h4 style="margin-top: 0; margin-bottom: 12px;">
                            {if result.success { "✅ Тест успешен" } else { "❌ Тест не пройден" }}
                        </h4>
                        <div style="margin-bottom: 8px;">
                            <strong>{"Статус: "}</strong>
                            {result.message.clone()}
                            {" "}
                            <span style="color: #666; font-size: 12px;">{"("}{result.duration_ms}{"ms)"}</span>
                        </div>
                        
                        {if let Some(details) = result.details.as_ref() {
                            view! {
                                <div style="margin-top: 12px; padding: 12px; background: rgba(255,193,7,0.1); border-left: 4px solid #ffc107; border-radius: 4px;">
                                    <div style="font-weight: bold; margin-bottom: 4px; color: #856404;">
                                        {"📝 Подробности:"}
                                    </div>
                                    <div style="color: #856404;">
                                    {details.clone()}
                                    </div>
                                </div>
                            }.into_any()
                        } else if !result.success {
                            view! {
                                <div style="margin-top: 12px; padding: 12px; background: rgba(220,53,69,0.1); border-left: 4px solid #dc3545; border-radius: 4px;">
                                    <div style="font-weight: bold; margin-bottom: 4px; color: #721c24;">
                                        {"⚠️ Внимание:"}
                                    </div>
                                    <div style="color: #721c24;">
                                        {"Подробная информация об ошибке отсутствует. Проверьте логи сервера для дополнительной информации."}
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! { <div></div> }.into_any()
                        }}
                        
                        <details style="margin-top: 12px;" open={!result.success}>
                            <summary style="cursor: pointer; font-weight: bold; margin-bottom: 8px;">
                                {"📋 Информация о тестировании"}
                            </summary>
                            <div style="margin-top: 8px; padding: 12px; background: rgba(0,0,0,0.03); border-radius: 4px; font-size: 12px;">
                                <div style="margin-bottom: 12px;">
                                    <strong>{"🌐 Эндпоинт:"}</strong>
                                    <div style="font-family: monospace; margin-top: 4px; padding: 8px; background: rgba(0,0,0,0.05); border-radius: 4px;">
                                        {match mp_code.as_str() {
                                            "mp-ozon" => "POST https://api-seller.ozon.ru/v3/product/list",
                                            "mp-wb" => "GET https://suppliers-api.wildberries.ru/public/api/v1/info",
                                            "mp-yandex" => "POST https://api.partner.market.yandex.ru/campaigns",
                                            "mp-lemana" => "GET https://api.lemanapro.ru/b2bintegration-products/v1/products?page=1&perPage=1",
                                            "mp-kuper" => "Не реализовано",
                                            _ => "Неизвестный маркетплейс"
                                        }}
                                    </div>
                                </div>
                                
                                <div style="margin-bottom: 12px;">
                                    <strong>{"📤 Отправляемые данные:"}</strong>
                                    <div style="font-family: monospace; margin-top: 4px; padding: 8px; background: rgba(0,0,0,0.05); border-radius: 4px; white-space: pre-wrap; word-break: break-all;">
                                        {match mp_code.as_str() {
                                            "mp-ozon" => {
                                                format!("Headers:\n  Client-Id: {}\n  Api-Key: ****\n  Content-Type: application/json\n\nBody:\n{{\n  \"filter\": {{ \"visibility\": \"ALL\" }},\n  \"last_id\": \"\",\n  \"limit\": 1\n}}",
                                                    form.get().application_id.clone().unwrap_or_else(|| "не указан".to_string()))
                                            },
                                            "mp-wb" => {
                                                "Headers:\n  Authorization: ****\n  Accept: application/json\n\nQuery: ?locale=ru".to_string()
                                            },
                                            "mp-yandex" => {
                                                format!("Headers:\n  Authorization: OAuth ****\n  Content-Type: application/json\n\nBody:\n{{\n  \"businessId\": {},\n  \"pageToken\": \"\"\n}}",
                                                    form.get().business_account_id.clone().unwrap_or_else(|| "не указан".to_string()))
                                            },
                                            "mp-lemana" => {
                                                "Headers:\n  Authorization: Bearer ****\n  Accept: application/json\n\nQuery: ?page=1&perPage=1".to_string()
                                            },
                                            _ => "Нет данных".to_string()
                                        }}
                                    </div>
                                </div>
                                
                                <div style="margin-bottom: 12px;">
                                    <strong>{"📥 Ожидаемый ответ:"}</strong>
                                    <div style="font-family: monospace; margin-top: 4px; padding: 8px; background: rgba(0,0,0,0.05); border-radius: 4px; white-space: pre-wrap; word-break: break-all;">
                                        {match mp_code.as_str() {
                                            "mp-ozon" => "{\n  \"result\": {\n    \"items\": [...],\n    \"total\": 123,\n    \"last_id\": \"...\"\n  }\n}",
                                            "mp-wb" => "HTTP 204 No Content (успешная авторизация)",
                                            "mp-yandex" => "{\n  \"campaigns\": [...],\n  \"pager\": { ... }\n}",
                                            "mp-lemana" => "{\n  \"products\": [...],\n  \"paging\": {\n    \"page\": 1,\n    \"perPage\": 1,\n    \"totalCount\": 123\n  }\n}",
                                            _ => "Нет данных"
                                        }}
                                    </div>
                                </div>
                                
                                {if !result.success && result.details.is_some() {
                                    view! {
                                        <div style="margin-bottom: 12px;">
                                            <strong>{"❌ Фактический ответ API:"}</strong>
                                            <div style="font-family: monospace; margin-top: 4px; padding: 8px; background: rgba(220,53,69,0.1); border: 1px solid rgba(220,53,69,0.3); border-radius: 4px; white-space: pre-wrap; word-break: break-all; color: #721c24;">
                                                {result.details.clone().unwrap_or_default()}
                                            </div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <div></div> }.into_any()
                                }}
                                
                                {if !result.success {
                                    view! {
                                        <div style="margin-top: 12px; padding: 8px; background: #fff3cd; border: 1px solid #ffc107; border-radius: 4px; color: #856404;">
                                            <strong>{"💡 Подсказка:"}</strong>
                                            <div style="margin-top: 4px;">
                                                {match mp_code.as_str() {
                                                    "mp-ozon" => "Проверьте правильность Client-Id и Api-Key. Убедитесь, что токен активен.",
                                                    "mp-wb" => "Убедитесь, что API ключ Wildberries действителен и имеет права на чтение.",
                                                    "mp-yandex" => "Проверьте OAuth токен и businessAccountId. Токен должен иметь права на campaigns:read.",
                                                    "mp-lemana" => "Убедитесь, что Bearer токен действителен и имеет доступ к products API.",
                                                    _ => "Проверьте правильность учетных данных."
                                                }}
                                            </div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <div></div> }.into_any()
                                }}
                            </div>
                            </details>
                    </div>
                }
            })}

            <div class="details-actions">
                <button
                    class="btn btn-warning"
                    on:click=handle_test
                    disabled=move || is_testing.get()
                >
                    {icon("test")}
                    {move || if is_testing.get() { "Тестирование..." } else { "Тест подключения" }}
                </button>
                <button
                    class="btn btn-primary"
                    on:click=handle_save
                >
                    {icon("save")}
                    {"Сохранить"}
                </button>
                <button
                    class="btn btn-secondary"
                    on:click=move |_| on_cancel(())
                >
                    {"Отмена"}
                </button>
            </div>

            <Modal>
                {move || {
                    if show_marketplace_picker.get() {
                        let selected_id = form.with(|f| {
                            if f.marketplace_id.is_empty() {
                                None
                            } else {
                                Some(f.marketplace_id.clone())
                            }
                        });
                        view! {
                            <MarketplacePicker
                                initial_selected_id=selected_id
                                on_selected=handle_marketplace_selected
                                on_cancel=handle_marketplace_cancel
                            />
                        }.into_any()
                    } else if show_organization_picker.get() {
                        let selected_id = organization_id.get();
                        view! {
                            <OrganizationPicker
                                initial_selected_id=selected_id
                                on_confirm=handle_organization_selected
                                on_cancel=handle_organization_cancel
                            />
                        }.into_any()
                    } else {
                        view! { <></> }.into_any()
                    }
                }}
            </Modal>
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

async fn fetch_connection(
    id: &str,
) -> Result<contracts::domain::a006_connection_mp::ConnectionMP, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/connection_mp/{}", api_base(), id);
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
    serde_json::from_str(&text).map_err(|e| format!("{e}"))
}

async fn save_connection(dto: ConnectionMPDto) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let body = serde_json::to_string(&dto).map_err(|e| format!("{e}"))?;
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body));

    let url = format!("{}/api/connection_mp", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{e:?}"))?;
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
    Ok(())
}

async fn test_connection(dto: ConnectionMPDto) -> Result<ConnectionTestResult, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let body = serde_json::to_string(&dto).map_err(|e| format!("{e}"))?;
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body));

    let url = format!("{}/api/connection_mp/test", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{e:?}"))?;
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
    serde_json::from_str(&text).map_err(|e| format!("{e}"))
}

#[derive(Clone, Debug)]
struct MarketplaceInfo {
    code: String,
    name: String,
}

async fn fetch_marketplace_info(id: &str) -> Result<MarketplaceInfo, String> {
    use contracts::domain::a005_marketplace::aggregate::Marketplace;
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/marketplace/{}", api_base(), id);
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
    let marketplace: Marketplace = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(MarketplaceInfo {
        code: marketplace.base.code,
        name: marketplace.base.description,
    })
}

#[allow(dead_code)]
async fn fetch_marketplace_name(id: &str) -> Result<String, String> {
    fetch_marketplace_info(id).await.map(|info| info.name)
}
