use crate::domain::a005_marketplace::ui::{MarketplacePicker, MarketplacePickerItem};
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
    let (form, set_form) = signal(ConnectionMPDto::default());
    let (error, set_error) = signal::<Option<String>>(None);
    let (test_result, set_test_result) = signal::<Option<ConnectionTestResult>>(None);
    let (is_testing, set_is_testing) = signal(false);
    let (show_picker, set_show_picker) = signal(false);
    let (marketplace_name, set_marketplace_name) = signal::<String>(String::new());

    // Load existing connection if id is provided
    if let Some(ref conn_id) = id {
        let id_clone = conn_id.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(conn) = fetch_connection(&id_clone).await {
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
                // TODO: загрузить название маркетплейса по ID
                set_marketplace_name.set("Загружено...".to_string());
            }
        });
    }

    let handle_save = move |_| {
        let dto = form.get();
        let on_saved = on_saved.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match save_connection(dto).await {
                Ok(_) => {
                    on_saved(());
                }
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
                    set_test_result.set(Some(result));
                    set_is_testing.set(false);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_is_testing.set(false);
                }
            }
        });
    };

    let handle_picker_selected = move |selected: Option<MarketplacePickerItem>| {
        set_show_picker.set(false);
        if let Some(item) = selected {
            set_marketplace_name.set(item.description.clone());
            set_form.update(|f| f.marketplace_id = item.id.clone());
        }
    };

    let handle_picker_cancel = move |_| {
        set_show_picker.set(false);
    };

    view! {
        <div class="details-container connection-mp-details">
            <div class="details-header">
                <h3>{if id.is_some() { "Редактирование подключения" } else { "Новое подключение" }}</h3>
            </div>

            {move || error.get().map(|e| view! { <div class="error">{e}</div> })}

            {move || test_result.get().map(|result| {
                let class = if result.success { "success" } else { "error" };
                view! {
                    <div class={class}>
                        {result.message}
                        {" "}
                        <small>{"("}{result.duration_ms}{"ms)"}</small>
                    </div>
                }
            })}

            <div class="details-form">
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
                            on:click=move |_| set_show_picker.set(true)
                        >
                            {icon("search")}
                            {"Выбрать"}
                        </button>
                    </div>
                </div>

                <div class="form-group">
                    <label for="organization">{"Организация"}</label>
                    <input
                        type="text"
                        id="organization"
                        prop:value={move || form.get().organization}
                        on:input=move |ev| set_form.update(|f| f.organization = event_target_value(&ev))
                        placeholder="Название организации"
                    />
                </div>

                <div class="form-group">
                    <label for="api_key">{"API Key"}</label>
                    <textarea
                        id="api_key"
                        prop:value={move || form.get().api_key}
                        on:input=move |ev| set_form.update(|f| f.api_key = event_target_value(&ev))
                        placeholder="Вставьте API ключ"
                        rows="3"
                    />
                </div>

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
                </div>

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
                </div>

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
                </div>

                <div class="form-group">
                    <label>
                        <input
                            type="checkbox"
                            prop:checked={move || form.get().is_used}
                            on:change=move |ev| set_form.update(|f| f.is_used = event_target_checked(&ev))
                        />
                        {" Используется"}
                    </label>
                </div>

                <div class="form-group">
                    <label>
                        <input
                            type="checkbox"
                            prop:checked={move || form.get().test_mode}
                            on:change=move |ev| set_form.update(|f| f.test_mode = event_target_checked(&ev))
                        />
                        {" Тестовый режим"}
                    </label>
                </div>

                <div class="form-group">
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

            {move || if show_picker.get() {
                view! {
                    <div class="modal-overlay">
                        <div class="modal-content">
                            <MarketplacePicker
                                on_selected=handle_picker_selected
                                on_cancel=handle_picker_cancel
                            />
                        </div>
                    </div>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }}
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

async fn fetch_connection(id: &str) -> Result<contracts::domain::a006_connection_mp::ConnectionMP, String> {
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
