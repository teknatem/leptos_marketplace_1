use crate::domain::a002_organization::ui::{OrganizationPicker, OrganizationPickerItem};
use crate::domain::a005_marketplace::ui::{MarketplacePicker, MarketplacePickerItem};
use crate::shared::icons::icon;
use crate::shared::picker_aggregate::{Modal, ModalService};
use contracts::domain::a006_connection_mp::{
    AuthorizationType, ConnectionMPDto, ConnectionTestResult,
};
use contracts::domain::common::AggregateId;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn ConnectionMPDetails(
    #[prop(into)] id: Signal<Option<String>>,
    on_saved: Callback<()>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let modal = use_context::<ModalService>().expect("ModalService not found");

    // RwSignal для полей формы (для двухсторонней привязки с Thaw)
    let description = RwSignal::new(String::new());
    let comment = RwSignal::new(String::new());
    let api_key = RwSignal::new(String::new());
    let supplier_id = RwSignal::new(String::new());
    let application_id = RwSignal::new(String::new());
    let business_account_id = RwSignal::new(String::new());
    let api_key_stats = RwSignal::new(String::new());
    let is_used = RwSignal::new(false);
    let test_mode = RwSignal::new(false);

    let marketplace_id = RwSignal::new(String::new());
    let organization = RwSignal::new(String::new());

    let (conn_id, set_conn_id) = signal::<Option<String>>(None);
    let (conn_code, set_conn_code) = signal::<Option<String>>(None);

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

    // Реактивная загрузка данных при изменении id
    Effect::new(move |_| {
        match id.get() {
            Some(conn_id_val) => {
                // Загрузка существующего подключения
                wasm_bindgen_futures::spawn_local(async move {
                    if let Ok(conn) = fetch_connection(&conn_id_val).await {
                        description.set(conn.base.description);
                        comment.set(conn.base.comment.unwrap_or_default());
                        api_key.set(conn.api_key);
                        supplier_id.set(conn.supplier_id.unwrap_or_default());
                        application_id.set(conn.application_id.unwrap_or_default());
                        business_account_id.set(conn.business_account_id.unwrap_or_default());
                        api_key_stats.set(conn.api_key_stats.unwrap_or_default());
                        is_used.set(conn.is_used);
                        test_mode.set(conn.test_mode);
                        marketplace_id.set(conn.marketplace_id.clone());
                        organization.set(conn.organization.clone());
                        set_conn_id.set(Some(conn.base.id.as_string()));
                        set_conn_code.set(Some(conn.base.code));
                        set_organization_name.set(conn.organization.clone());

                        if let Ok(mp_info) = fetch_marketplace_info(&conn.marketplace_id).await {
                            set_marketplace_name.set(mp_info.name);
                            set_marketplace_code.set(mp_info.code);
                        }
                    }
                });
            }
            None => {
                // Сброс формы для нового подключения
                description.set(String::new());
                comment.set(String::new());
                api_key.set(String::new());
                supplier_id.set(String::new());
                application_id.set(String::new());
                business_account_id.set(String::new());
                api_key_stats.set(String::new());
                is_used.set(false);
                test_mode.set(false);
                marketplace_id.set(String::new());
                organization.set(String::new());
                set_conn_id.set(None);
                set_conn_code.set(None);
                set_marketplace_name.set(String::new());
                set_marketplace_code.set(String::new());
                set_organization_name.set(String::new());
                set_organization_id.set(None);
                set_error.set(None);
                set_test_result.set(None);
            }
        }
    });

    let handle_save = move |_: leptos::ev::MouseEvent| {
        let dto = ConnectionMPDto {
            id: conn_id.get(),
            code: conn_code.get(),
            description: description.get(),
            comment: if comment.get().is_empty() {
                None
            } else {
                Some(comment.get())
            },
            marketplace_id: marketplace_id.get(),
            organization: organization.get(),
            api_key: api_key.get(),
            supplier_id: if supplier_id.get().is_empty() {
                None
            } else {
                Some(supplier_id.get())
            },
            application_id: if application_id.get().is_empty() {
                None
            } else {
                Some(application_id.get())
            },
            is_used: is_used.get(),
            business_account_id: if business_account_id.get().is_empty() {
                None
            } else {
                Some(business_account_id.get())
            },
            api_key_stats: if api_key_stats.get().is_empty() {
                None
            } else {
                Some(api_key_stats.get())
            },
            test_mode: test_mode.get(),
            authorization_type: AuthorizationType::default(),
        };

        wasm_bindgen_futures::spawn_local(async move {
            match save_connection(dto).await {
                Ok(_) => on_saved.run(()),
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    let handle_test = move |_: leptos::ev::MouseEvent| {
        set_is_testing.set(true);
        set_test_result.set(None);

        let dto = ConnectionMPDto {
            id: conn_id.get(),
            code: conn_code.get(),
            description: description.get(),
            comment: if comment.get().is_empty() {
                None
            } else {
                Some(comment.get())
            },
            marketplace_id: marketplace_id.get(),
            organization: organization.get(),
            api_key: api_key.get(),
            supplier_id: if supplier_id.get().is_empty() {
                None
            } else {
                Some(supplier_id.get())
            },
            application_id: if application_id.get().is_empty() {
                None
            } else {
                Some(application_id.get())
            },
            is_used: is_used.get(),
            business_account_id: if business_account_id.get().is_empty() {
                None
            } else {
                Some(business_account_id.get())
            },
            api_key_stats: if api_key_stats.get().is_empty() {
                None
            } else {
                Some(api_key_stats.get())
            },
            test_mode: test_mode.get(),
            authorization_type: AuthorizationType::default(),
        };

        wasm_bindgen_futures::spawn_local(async move {
            match test_connection(dto).await {
                Ok(result) => {
                    // Debug log для проверки что приходит с сервера
                    web_sys::console::log_1(
                        &format!(
                            "Test result: success={}, message={}, details={:?}",
                            result.success, result.message, result.details
                        )
                        .into(),
                    );
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
            marketplace_id.set(item.id.clone());
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
            organization.set(item.description.clone());
        }
    };

    let handle_organization_cancel = move |_| {
        modal.hide();
        set_show_organization_picker.set(false);
    };

    view! {
        <div class="details-container connection-mp-details">
            {move || error.get().map(|e| view! {
                <div style="padding: 8px 12px; margin-bottom: 10px; background: var(--color-error-50); border: 1px solid var(--color-error-100); border-radius: 6px; color: var(--color-error); font-size: 13px;">
                    {e}
                </div>
            })}

            // Секция 1: Основная информация
            <div style="margin-bottom: 12px;">
                <h4 style="margin: 0 0 8px 0; padding-bottom: 4px; border-bottom: 2px solid var(--color-border); font-size: 14px; font-weight: 600;">
                    "Основная информация"
                </h4>
                <div style="display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 10px;">
                    <div class="form__group">
                        <label style="font-size: 13px; display: block; margin-bottom: 4px;">{"Наименование"}</label>
                        <Input
                            value=description
                            placeholder="Например: Озон (Сантехсистем)"
                        />
                    </div>

                    <div class="form__group">
                        <label style="font-size: 13px; display: block; margin-bottom: 4px;">{"Маркетплейс"}</label>
                        <div style="display: flex; gap: 6px;">
                            <input
                                type="text"
                                value=move || marketplace_name.get()
                                readonly
                                placeholder="Выберите"
                                style="flex: 1; padding: 6px 10px; border: 1px solid #d1d1d1; border-radius: 4px; background: #f5f5f5;"
                            />
                            <Button
                                appearance=ButtonAppearance::Secondary
                                on_click=move |_| {
                                    set_show_organization_picker.set(false);
                                    set_show_marketplace_picker.set(true);
                                    modal.show();
                                }
                            >
                                {icon("search")}
                            </Button>
                        </div>
                    </div>

                    <div class="form__group">
                        <label style="font-size: 13px; display: block; margin-bottom: 4px;">{"Организация"}</label>
                        <div style="display: flex; gap: 6px;">
                            <input
                                type="text"
                                value=move || organization_name.get()
                                readonly
                                placeholder="Выберите"
                                style="flex: 1; padding: 6px 10px; border: 1px solid #d1d1d1; border-radius: 4px; background: #f5f5f5;"
                            />
                            <Button
                                appearance=ButtonAppearance::Secondary
                                on_click=move |_| {
                                    set_show_marketplace_picker.set(false);
                                    set_show_organization_picker.set(true);
                                    modal.show();
                                }
                            >
                                {icon("search")}
                            </Button>
                        </div>
                    </div>

                    <div class="form__group" style="grid-column: 1 / -1;">
                        <label style="font-size: 13px; display: block; margin-bottom: 4px;">{"Комментарий"}</label>
                        <Textarea
                            value=comment
                            placeholder="Дополнительная информация"
                        />
                    </div>
                </div>
            </div>

            // Секция 2: API конфигурация
            <div style="margin-bottom: 12px;">
                <h4 style="margin: 0 0 8px 0; padding-bottom: 4px; border-bottom: 2px solid var(--color-border); font-size: 14px; font-weight: 600;">
                    "API конфигурация"
                </h4>
                <div style="display: grid; grid-template-columns: 2fr 1fr 1fr; gap: 10px;">
                    <div class="form__group" style="grid-row: span 2;">
                        <label style="font-size: 13px; display: block; margin-bottom: 4px;">{"API Key"}</label>
                        <Textarea
                            value=api_key
                            placeholder="Вставьте API ключ"
                        />
                        <small class="help-text" style="font-size: 10px; line-height: 1.2;">
                            {"• WB: Bearer • Ozon: Api-Key • Яндекс: OAuth"}
                        </small>
                    </div>

                    <div class="form__group">
                        <label style="font-size: 13px; display: block; margin-bottom: 4px;">{"Client ID"}</label>
                        <Input
                            value=supplier_id
                            placeholder="Ozon, Яндекс"
                        />
                        <small class="help-text" style="font-size: 10px;">{"Ozon, Яндекс"}</small>
                    </div>

                    <div class="form__group">
                        <label style="font-size: 13px; display: block; margin-bottom: 4px;">{"App ID"}</label>
                        <Input
                            value=application_id
                            placeholder="Ozon"
                        />
                        <small class="help-text" style="font-size: 10px;">{"Ozon"}</small>
                    </div>

                    <div class="form__group">
                        <label style="font-size: 13px; display: block; margin-bottom: 4px;">{"Business ID"}</label>
                        <Input
                            value=business_account_id
                            placeholder="Яндекс"
                        />
                        <small class="help-text" style="font-size: 10px;">{"Яндекс"}</small>
                    </div>

                    <div class="form__group">
                        <label style="font-size: 13px; display: block; margin-bottom: 4px;">{"Stats Key"}</label>
                        <Input
                            value=api_key_stats
                            placeholder="Опционально"
                        />
                    </div>
                </div>
            </div>

            // Секция 3: Настройки и действия
            <div style="display: flex; justify-content: space-between; align-items: center; padding: 8px 12px; background: var(--color-background-secondary); border-radius: 6px; margin-bottom: 12px;">
                <div style="display: flex; gap: 24px; align-items: center;">
                    <Checkbox checked=is_used label="Используется"/>

                    <Checkbox checked=test_mode label="Тестовый режим"/>
                </div>

                <div style="display: flex; gap: 8px;">
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=handle_test
                        disabled=Signal::derive(move || is_testing.get())
                    >
                        {icon("test")}
                        {move || if is_testing.get() { " Тест..." } else { " Тест" }}
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=handle_save
                    >
                        {icon("save")}
                        " Сохранить"
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| on_cancel.run(())
                    >
                        "Отмена"
                    </Button>
                </div>
            </div>

            {move || test_result.get().map(|result| {
                let class = if result.success { "success" } else { "error" };
                let mp_code = marketplace_code.get();
                view! {
                    <div class={class} style="margin-top: 12px; padding: 12px; border-radius: 6px; font-size: 13px;">
                        <h4 style="margin-top: 0; margin-bottom: 8px; font-size: 14px;">
                            {if result.success { "✅ Тест успешен" } else { "❌ Тест не пройден" }}
                        </h4>
                        <div style="margin-bottom: 6px;">
                            <strong>{"Статус: "}</strong>
                            {result.message.clone()}
                            {" "}
                            <span style="color: #666; font-size: 11px;">{"("}{result.duration_ms}{"ms)"}</span>
                        </div>

                        {if let Some(details) = result.details.as_ref() {
                            view! {
                                <div style="margin-top: 8px; padding: 10px; background: rgba(255,193,7,0.1); border-left: 3px solid #ffc107; border-radius: 4px;">
                                    <div style="font-weight: bold; margin-bottom: 4px; color: #856404; font-size: 12px;">
                                        {"📝 Подробности:"}
                                    </div>
                                    <div style="color: #856404; font-size: 12px;">
                                    {details.clone()}
                                    </div>
                                </div>
                            }.into_any()
                        } else if !result.success {
                            view! {
                                <div style="margin-top: 8px; padding: 10px; background: rgba(220,53,69,0.1); border-left: 3px solid #dc3545; border-radius: 4px;">
                                    <div style="font-weight: bold; margin-bottom: 4px; color: #721c24; font-size: 12px;">
                                        {"⚠️ Внимание:"}
                                    </div>
                                    <div style="color: #721c24; font-size: 12px;">
                                        {"Подробная информация об ошибке отсутствует. Проверьте логи сервера."}
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! { <div></div> }.into_any()
                        }}

                        <details style="margin-top: 10px;" open={!result.success}>
                            <summary style="cursor: pointer; font-weight: bold; margin-bottom: 6px; font-size: 12px;">
                                {"📋 Информация о тестировании"}
                            </summary>
                            <div style="margin-top: 6px; padding: 10px; background: rgba(0,0,0,0.03); border-radius: 4px; font-size: 11px;">
                                <div style="margin-bottom: 10px;">
                                    <strong>{"🌐 Эндпоинт:"}</strong>
                                    <div style="font-family: monospace; margin-top: 3px; padding: 6px; background: rgba(0,0,0,0.05); border-radius: 3px; font-size: 10px;">
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

                                <div style="margin-bottom: 10px;">
                                    <strong>{"📤 Отправляемые данные:"}</strong>
                                    <div style="font-family: monospace; margin-top: 3px; padding: 6px; background: rgba(0,0,0,0.05); border-radius: 3px; white-space: pre-wrap; word-break: break-all; font-size: 10px;">
                                        {match mp_code.as_str() {
                                            "mp-ozon" => {
                                                format!("Headers:\n  Client-Id: {}\n  Api-Key: ****\n  Content-Type: application/json\n\nBody:\n{{\n  \"filter\": {{ \"visibility\": \"ALL\" }},\n  \"last_id\": \"\",\n  \"limit\": 1\n}}",
                                                    application_id.get())
                                            },
                                            "mp-wb" => {
                                                "Headers:\n  Authorization: ****\n  Accept: application/json\n\nQuery: ?locale=ru".to_string()
                                            },
                                            "mp-yandex" => {
                                                format!("Headers:\n  Authorization: OAuth ****\n  Content-Type: application/json\n\nBody:\n{{\n  \"businessId\": {},\n  \"pageToken\": \"\"\n}}",
                                                    business_account_id.get())
                                            },
                                            "mp-lemana" => {
                                                "Headers:\n  Authorization: Bearer ****\n  Accept: application/json\n\nQuery: ?page=1&perPage=1".to_string()
                                            },
                                            _ => "Нет данных".to_string()
                                        }}
                                    </div>
                                </div>

                                <div style="margin-bottom: 10px;">
                                    <strong>{"📥 Ожидаемый ответ:"}</strong>
                                    <div style="font-family: monospace; margin-top: 3px; padding: 6px; background: rgba(0,0,0,0.05); border-radius: 3px; white-space: pre-wrap; word-break: break-all; font-size: 10px;">
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
                                        <div style="margin-bottom: 10px;">
                                            <strong>{"❌ Фактический ответ API:"}</strong>
                                            <div style="font-family: monospace; margin-top: 3px; padding: 6px; background: rgba(220,53,69,0.1); border: 1px solid rgba(220,53,69,0.3); border-radius: 3px; white-space: pre-wrap; word-break: break-all; color: #721c24; font-size: 10px;">
                                                {result.details.clone().unwrap_or_default()}
                                            </div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <div></div> }.into_any()
                                }}

                                {if !result.success {
                                    view! {
                                        <div style="margin-top: 10px; padding: 6px; background: #fff3cd; border: 1px solid #ffc107; border-radius: 4px; color: #856404; font-size: 11px;">
                                            <strong>{"💡 Подсказка:"}</strong>
                                            <div style="margin-top: 3px;">
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

            <Modal>
                {move || {
                    if show_marketplace_picker.get() {
                        let selected_id = {
                            let mp_id = marketplace_id.get();
                            if mp_id.is_empty() {
                                None
                            } else {
                                Some(mp_id)
                            }
                        };
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
