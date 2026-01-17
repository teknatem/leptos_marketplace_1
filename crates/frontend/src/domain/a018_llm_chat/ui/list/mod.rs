use crate::shared::api_utils::api_base;
use crate::shared::icons::icon;
use contracts::domain::a017_llm_agent::aggregate::LlmAgent;
use contracts::domain::a018_llm_chat::aggregate::{LlmChat, LlmChatMessage};
use leptos::prelude::*;
use thaw::{Button, ButtonAppearance};

#[component]
#[allow(non_snake_case)]
pub fn LlmChatList() -> impl IntoView {
    let (chats, set_chats) = signal::<Vec<LlmChat>>(Vec::new());
    let (agents, set_agents) = signal::<Vec<LlmAgent>>(Vec::new());
    let (selected_chat_id, set_selected_chat_id) = signal::<Option<String>>(None);
    let (messages, set_messages) = signal::<Vec<LlmChatMessage>>(Vec::new());
    let (new_message, set_new_message) = signal(String::new());
    let (new_chat_desc, set_new_chat_desc) = signal(String::new());
    let (new_chat_agent_id, set_new_chat_agent_id) = signal(String::new());
    let (error, set_error) = signal::<Option<String>>(None);

    let fetch_chats = move || {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_chats().await {
                Ok(v) => {
                    set_chats.set(v);
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    let fetch_agents = move || {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_agents().await {
                Ok(v) => {
                    if new_chat_agent_id.get().is_empty() {
                        if let Some(first) = v.first() {
                            set_new_chat_agent_id.set(first.to_string_id());
                        }
                    }
                    set_agents.set(v);
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    let select_chat = move |chat_id: String| {
        set_selected_chat_id.set(Some(chat_id.clone()));
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_messages(&chat_id).await {
                Ok(v) => {
                    set_messages.set(v);
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    let handle_create_chat = move || {
        let desc = new_chat_desc.get();
        let agent_id = new_chat_agent_id.get();
        if desc.trim().is_empty() {
            set_error.set(Some("Введите название чата".to_string()));
            return;
        }
        if agent_id.trim().is_empty() {
            set_error.set(Some("Выберите агента".to_string()));
            return;
        }

        wasm_bindgen_futures::spawn_local(async move {
            match create_chat(&desc, &agent_id).await {
                Ok(_) => {
                    set_new_chat_desc.set(String::new());
                    fetch_chats();
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    let handle_send_message = move || {
        let content = new_message.get();
        let chat_id = selected_chat_id.get();
        if content.trim().is_empty() {
            return;
        }
        let Some(chat_id) = chat_id else {
            set_error.set(Some("Выберите чат".to_string()));
            return;
        };

        set_new_message.set(String::new());
        wasm_bindgen_futures::spawn_local(async move {
            match send_message(&chat_id, &content).await {
                Ok(_) => {
                    match fetch_messages(&chat_id).await {
                        Ok(v) => {
                            set_messages.set(v);
                            set_error.set(None);
                        }
                        Err(e) => set_error.set(Some(e)),
                    }
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    fetch_chats();
    fetch_agents();

    view! {
        <div style="padding: 20px; height: 100%;">
            <div style="display: flex; justify-content: space-between; align-items: center;">
                <h1 style="font-size: 24px; font-weight: bold;">{"LLM Чат"}</h1>
                <div>
                    <Button appearance=ButtonAppearance::Secondary on_click=move |_| fetch_chats()>
                        {icon("refresh")}
                        " Обновить"
                    </Button>
                </div>
            </div>

            <div style="margin-top: 12px;">
                {move || error.get().map(|e| view! {
                    <div style="padding: 12px; background: var(--color-error-50); border: 1px solid var(--color-error-100); border-radius: 8px; display: flex; align-items: center; gap: 8px;">
                        <span style="color: var(--color-error); font-size: 18px;">"⚠"</span>
                        <span style="color: var(--color-error);">{e}</span>
                    </div>
                })}
            </div>

            <div style="display: grid; grid-template-columns: 320px 1fr; gap: 16px; margin-top: 16px; height: calc(100% - 90px);">
                <div style="height: 100%; display: flex; flex-direction: column; padding: 16px; border: 1px solid var(--colorNeutralStroke2); border-radius: 12px; background: var(--colorNeutralBackground1);">
                    <div style="display: flex; flex-direction: column; gap: 8px;">
                        <label class="form__label">"Новый чат"</label>
                        <input
                            style="height: 32px; padding: 4px 8px; border: 1px solid var(--colorNeutralStroke2); border-radius: 6px;"
                            placeholder="Например: Анализ продаж"
                            prop:value=move || new_chat_desc.get()
                            on:input=move |ev| set_new_chat_desc.set(event_target_value(&ev))
                        />
                        <label class="form__label">"Агент"</label>
                        <select
                            style="height: 32px; padding: 4px 8px; border: 1px solid var(--colorNeutralStroke2); border-radius: 6px;"
                            prop:value=move || new_chat_agent_id.get()
                            on:change=move |ev| set_new_chat_agent_id.set(event_target_value(&ev))
                        >
                            <For
                                each=move || agents.get()
                                key=|agent| agent.to_string_id()
                                let:agent
                            >
                                {{
                                    let id = agent.to_string_id();
                                    let desc = agent.base.description.clone();
                                    view! {
                                        <option value=id>{desc}</option>
                                    }
                                }}
                            </For>
                        </select>
                        <Button appearance=ButtonAppearance::Primary on_click=move |_| handle_create_chat()>
                            {icon("plus")}
                            " Создать чат"
                        </Button>
                    </div>

                    <div style="margin-top: 16px; font-weight: 600;">"Список чатов"</div>
                    <div style="margin-top: 8px; display: flex; flex-direction: column; gap: 6px; overflow: auto;">
                        <For
                            each=move || chats.get()
                            key=|chat| chat.to_string_id()
                            let:chat
                        >
                            {{
                                let id = chat.to_string_id();
                                let id_for_style = id.clone();
                                let id_for_click = id.clone();
                                let desc = chat.base.description.clone();
                                view! {
                                    <div
                                        style=move || {
                                            let is_selected = selected_chat_id.get().as_deref() == Some(&id_for_style);
                                            if is_selected {
                                                "padding: 8px; border-radius: 6px; background: var(--colorBrandBackground2); cursor: pointer;"
                                            } else {
                                                "padding: 8px; border-radius: 6px; background: var(--colorNeutralBackground1); cursor: pointer;"
                                            }
                                        }
                                        on:click=move |_| select_chat(id_for_click.clone())
                                    >
                                        {desc}
                                    </div>
                                }
                            }}
                        </For>
                    </div>
                </div>

                <div style="height: 100%; display: flex; flex-direction: column; padding: 16px; border: 1px solid var(--colorNeutralStroke2); border-radius: 12px; background: var(--colorNeutralBackground1);">
                    <div style="font-weight: 600;">"Сообщения"</div>
                    <div style="margin-top: 12px; flex: 1; overflow: auto; display: flex; flex-direction: column; gap: 12px;">
                        <For
                            each=move || messages.get()
                            key=|msg| msg.id.to_string()
                            let:msg
                        >
                            <div
                                style=if matches!(msg.role, contracts::domain::a018_llm_chat::aggregate::ChatRole::User) {
                                    "align-self: flex-end; max-width: 70%; background: var(--colorBrandBackground2); padding: 8px 12px; border-radius: 10px;"
                                } else {
                                    "align-self: flex-start; max-width: 70%; background: var(--colorNeutralBackground2); padding: 8px 12px; border-radius: 10px;"
                                }
                            >
                                <div style="white-space: pre-wrap;">{msg.content}</div>
                            </div>
                        </For>
                    </div>
                    <div style="margin-top: 12px; display: flex; gap: 8px;">
                        <input
                            style="flex: 1; height: 32px; padding: 4px 8px; border: 1px solid var(--colorNeutralStroke2); border-radius: 6px;"
                            placeholder="Введите сообщение..."
                            prop:value=move || new_message.get()
                            on:input=move |ev| set_new_message.set(event_target_value(&ev))
                            on:keydown=move |ev| {
                                if ev.key() == "Enter" && !ev.shift_key() {
                                    ev.prevent_default();
                                    handle_send_message();
                                }
                            }
                        />
                        <Button appearance=ButtonAppearance::Primary on_click=move |_| handle_send_message()>
                            {icon("send")}
                            " Отправить"
                        </Button>
                    </div>
                </div>
            </div>
        </div>
    }
}

async fn fetch_chats() -> Result<Vec<LlmChat>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a018-llm-chat", api_base());
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
    let data: Vec<LlmChat> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}

async fn fetch_agents() -> Result<Vec<LlmAgent>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a017-llm-agent", api_base());
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
    let data: Vec<LlmAgent> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}

async fn create_chat(description: &str, agent_id: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a018-llm-chat", api_base());
    let dto = serde_json::json!({
        "id": null,
        "code": null,
        "description": description,
        "comment": null,
        "agent_id": agent_id
    });

    let body = wasm_bindgen::JsValue::from_str(&dto.to_string());
    opts.set_body(&body);

    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
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

async fn fetch_messages(chat_id: &str) -> Result<Vec<LlmChatMessage>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a018-llm-chat/{}/messages", api_base(), chat_id);
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
    let data: Vec<LlmChatMessage> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}

async fn send_message(chat_id: &str, content: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a018-llm-chat/{}/messages", api_base(), chat_id);
    let dto = serde_json::json!({ "content": content });
    let body = wasm_bindgen::JsValue::from_str(&dto.to_string());
    opts.set_body(&body);

    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
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
