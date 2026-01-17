use crate::shared::api_utils::api_base;
use crate::shared::icons::icon;
use contracts::domain::a017_llm_agent::aggregate::LlmAgent;
use leptos::prelude::*;
use thaw::*;

#[component]
#[allow(non_snake_case)]
pub fn LlmAgentDetails(
    id: Signal<Option<String>>,
    on_saved: Callback<()>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let code = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let comment = RwSignal::new(String::new());
    let provider_type = RwSignal::new("OpenAI".to_string());
    let api_endpoint = RwSignal::new("https://api.openai.com/v1".to_string());
    let api_key = RwSignal::new(String::new());
    let model_name = RwSignal::new("gpt-4o".to_string());
    let temperature = RwSignal::new("0.7".to_string());
    let max_tokens = RwSignal::new("4096".to_string());
    let system_prompt = RwSignal::new(String::new());
    let is_primary = RwSignal::new(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (test_result, set_test_result) = signal::<Option<(bool, String)>>(None);
    let (is_testing, set_is_testing) = signal(false);

    // Загрузка данных при редактировании
    Effect::new(move |_| {
        if let Some(agent_id) = id.get() {
            wasm_bindgen_futures::spawn_local(async move {
                match fetch_agent(&agent_id).await {
                    Ok(agent) => {
                        code.set(agent.base.code);
                        description.set(agent.base.description);
                        comment.set(agent.base.comment.unwrap_or_default());
                        provider_type.set(agent.provider_type.as_str().to_string());
                        api_endpoint.set(agent.api_endpoint);
                        api_key.set(agent.api_key);
                        model_name.set(agent.model_name);
                        temperature.set(agent.temperature.to_string());
                        max_tokens.set(agent.max_tokens.to_string());
                        system_prompt.set(agent.system_prompt.unwrap_or_default());
                        is_primary.set(agent.is_primary);
                    }
                    Err(e) => set_error.set(Some(e)),
                }
            });
        }
    });

    let handle_save = move |_| {
        let id_value = id.get();
        let temp_val: f64 = temperature.get().parse().unwrap_or(0.7);
        let tokens_val: i32 = max_tokens.get().parse().unwrap_or(4096);

        let dto = serde_json::json!({
            "id": id_value,
            "code": code.get(),
            "description": description.get(),
            "comment": if comment.get().is_empty() { None } else { Some(comment.get()) },
            "provider_type": provider_type.get(),
            "api_endpoint": api_endpoint.get(),
            "api_key": api_key.get(),
            "model_name": model_name.get(),
            "temperature": temp_val,
            "max_tokens": tokens_val,
            "system_prompt": if system_prompt.get().is_empty() { None } else { Some(system_prompt.get()) },
            "is_primary": is_primary.get(),
        });

        wasm_bindgen_futures::spawn_local(async move {
            match save_agent(dto).await {
                Ok(_) => on_saved.run(()),
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    let handle_test = move |_| {
        let id_value = match id.get() {
            Some(v) => v,
            None => {
                set_test_result.set(Some((
                    false,
                    "Сохраните агента перед тестированием".to_string(),
                )));
                return;
            }
        };

        set_is_testing.set(true);
        set_test_result.set(None);

        wasm_bindgen_futures::spawn_local(async move {
            match test_agent_connection(&id_value).await {
                Ok(result) => {
                    set_test_result.set(Some((result.success, result.message)));
                    set_is_testing.set(false);
                }
                Err(e) => {
                    set_test_result.set(Some((false, format!("Ошибка: {}", e))));
                    set_is_testing.set(false);
                }
            }
        });
    };

    let is_edit_mode = Signal::derive(move || id.get().is_some());

    view! {
        <div class="details-form" style="padding: 20px;">
            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center style="margin-bottom: 20px;">
                <h2 style="font-size: 20px; font-weight: bold;">
                    {move || if is_edit_mode.get() { "Редактирование агента LLM" } else { "Новый агент LLM" }}
                </h2>
                <Space>
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=handle_save
                    >
                        {icon("save")}
                        " Сохранить"
                    </Button>
                    <Show when=move || is_edit_mode.get()>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            on_click=handle_test
                            disabled=is_testing
                        >
                            {icon("link")}
                            {move || if is_testing.get() { " Тестирование..." } else { " Тест подключения" }}
                        </Button>
                    </Show>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| on_cancel.run(())
                    >
                        {icon("close")}
                        " Отмена"
                    </Button>
                </Space>
            </Flex>

            {move || error.get().map(|e| view! {
                <div style="padding: 12px; margin-bottom: 16px; background: var(--color-error-50); border: 1px solid var(--color-error-100); border-radius: 8px;">
                    <span style="color: var(--color-error);">{e}</span>
                </div>
            })}

            {move || test_result.get().map(|(success, message)| {
                let bg = if success { "var(--color-success-50)" } else { "var(--color-error-50)" };
                let border = if success { "var(--color-success-100)" } else { "var(--color-error-100)" };
                let color = if success { "var(--color-success)" } else { "var(--color-error)" };
                view! {
                    <div style=format!("padding: 12px; margin-bottom: 16px; background: {}; border: 1px solid {}; border-radius: 8px;", bg, border)>
                        <span style=format!("color: {};", color)>{message}</span>
                    </div>
                }
            })}

            <div style="display: grid; grid-template-columns: 500px 500px; gap: var(--spacing-md); max-width: 1050px; align-items: start; align-content: start;">
            <Card>
                <div class = "form__group">
                    <label class="form__label">"Код"</label>
                    <Input value=code placeholder="GPT4O-MAIN" />
                </div>

                <div class = "form__group">
                    <label class="form__label">"Наименование" <span style="color: red;">"*"</span></label>
                    <Input value=description placeholder="GPT-4o основной агент" />
                </div>

                <div class = "form__group">
                    <label class="form__label">"Провайдер"</label>
                    <Input value=provider_type placeholder="OpenAI" />
                    <div style="font-size: 12px; color: var(--colorNeutralForeground3);">"OpenAI, Anthropic, Ollama"</div>
                </div>

                <div class = "form__group">
                    <label class="form__label">"API Endpoint"</label>
                    <Input value=api_endpoint placeholder="https://api.openai.com/v1" />
                </div>

                <div class = "form__group">
                    <label class="form__label">"Комментарий"</label>
                    <Textarea value=comment placeholder="Используется для анализа продаж" />
                </div>

            </Card>
            <Card>
                <div class = "form__group">
                    <label class="form__label">"API Ключ" <span style="color: red;">"*"</span></label>
                    <Input value=api_key placeholder="sk-..." />
                </div>

                <div class = "form__group">
                    <label class="form__label">"Модель"</label>
                    <Input value=model_name placeholder="gpt-4o" />
                </div>

                <div class = "form__group">
                    <label class="form__label">"Temperature (0.0-2.0)"</label>
                    <Input value=temperature placeholder="0.7" />
                </div>

                <div class = "form__group">
                    <label class="form__label">"Max Tokens"</label>
                    <Input value=max_tokens placeholder="4096" />
                </div>

                <div class = "form__group">
                    <label class="form__label">"Системный промпт"</label>
                    <Textarea attr:style = "min-height : 80px" value=system_prompt placeholder="Ты аналитик данных маркетплейсов..." />
                </div>

                <div style="display: flex; align-items: center; gap: 8px;">
                    <input
                        type="checkbox"
                        prop:checked=move || is_primary.get()
                        on:change=move |ev| {
                            let checked = event_target_checked(&ev);
                            is_primary.set(checked);
                        }
                    />
                    <span>"Использовать как основной агент"</span>
                </div>

            </Card>

            </div>
        </div>
    }
}

async fn fetch_agent(id: &str) -> Result<LlmAgent, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a017-llm-agent/{}", api_base(), id);
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
    let agent: LlmAgent = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(agent)
}

async fn save_agent(dto: serde_json::Value) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let body = serde_json::to_string(&dto).map_err(|e| format!("{e}"))?;
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body));

    let url = format!("{}/api/a017-llm-agent", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Content-Type", "application/json")
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

#[derive(serde::Deserialize)]
struct TestConnectionResponse {
    success: bool,
    message: String,
}

async fn test_agent_connection(id: &str) -> Result<TestConnectionResponse, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a017-llm-agent/{}/test", api_base(), id);
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
    let result: TestConnectionResponse = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(result)
}
