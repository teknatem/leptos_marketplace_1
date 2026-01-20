use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::icons::icon;
use crate::shared::modal_stack::ModalStackService;
use contracts::domain::a017_llm_agent::aggregate::LlmAgent;
use contracts::domain::a018_llm_chat::aggregate::LlmChatListItem;
use leptos::prelude::*;
use thaw::*;

#[component]
#[allow(non_snake_case)]
pub fn LlmChatList() -> impl IntoView {
    let modal_stack =
        use_context::<ModalStackService>().expect("ModalStackService not found in context");
    let tabs_store = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    
    let (items, set_items) = signal::<Vec<LlmChatListItem>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);
    let show_create_modal = RwSignal::new(false);

    let fetch = move || {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_chats_with_stats().await {
                Ok(v) => {
                    set_items.set(v);
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    let handle_create_new = move || {
        show_create_modal.set(true);
    };

    let handle_open_chat = move |id: String| {
        let tab_key = format!("a018_llm_chat_detail_{}", id);
        let tab_label = format!("Чат");
        tabs_store.open_tab(&tab_key, &tab_label);
    };

    let handle_delete = move |id: String| {
        let confirmed = {
            if let Some(win) = web_sys::window() {
                win.confirm_with_message("Удалить чат?").unwrap_or(false)
            } else {
                false
            }
        };
        if !confirmed {
            return;
        }

        wasm_bindgen_futures::spawn_local(async move {
            let _ = delete_chat(&id).await;
        });
        fetch();
    };

    fetch();

    view! {
        <div style="padding: 20px;">
            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center>
                <h1 style="font-size: 24px; font-weight: bold;">{"LLM Чаты"}</h1>
                <Space>
                    <Button appearance=ButtonAppearance::Primary on_click=move |_| handle_create_new()>
                        {icon("plus")}
                        " Новый чат"
                    </Button>
                    <Button appearance=ButtonAppearance::Secondary on_click=move |_| fetch()>
                        {icon("refresh")}
                        " Обновить"
                    </Button>
                </Space>
            </Flex>
            <div style="margin-top: 16px;">
                {move || error.get().map(|e| view! {
                    <div style="padding: 12px; background: var(--color-error-50); border: 1px solid var(--color-error-100); border-radius: 8px; display: flex; align-items: center; gap: 8px;">
                        <span style="color: var(--color-error); font-size: 18px;">"⚠"</span>
                        <span style="color: var(--color-error);">{e}</span>
                    </div>
                })}
            </div>

            <Table>
                <TableHeader>
                    <TableRow>
                        <TableHeaderCell resizable=true min_width=250.0>"Название"</TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=200.0>"Агент"</TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=150.0>"Модель"</TableHeaderCell>
                        <TableHeaderCell min_width=100.0>"Сообщений"</TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=150.0>"Последнее сообщение"</TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=150.0>"Создан"</TableHeaderCell>
                        <TableHeaderCell min_width=80.0>"Действия"</TableHeaderCell>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {move || items.get().into_iter().map(|item| {
                        let id = item.id.clone();
                        let id_for_link = id.clone();
                        let id_for_delete = id.clone();
                        
                        let msg_count = item.message_count.unwrap_or(0);
                        let last_msg = item.last_message_at.map(|dt| {
                            dt.format("%d.%m.%Y %H:%M").to_string()
                        }).unwrap_or_else(|| "-".to_string());
                        let created = item.created_at.format("%d.%m.%Y %H:%M").to_string();
                        
                        view! {
                            <TableRow>
                                <TableCell>
                                    <TableCellLayout>
                                        <a
                                            href="#"
                                            style="color: var(--colorBrandForeground1); text-decoration: none; cursor: pointer;"
                                            on:click=move |e| {
                                                e.prevent_default();
                                                handle_open_chat(id_for_link.clone());
                                            }
                                        >
                                            {item.description.clone()}
                                        </a>
                                    </TableCellLayout>
                                </TableCell>
                                <TableCell>
                                    <TableCellLayout>
                                        {item.agent_name.clone().unwrap_or_else(|| "Неизвестный агент".to_string())}
                                    </TableCellLayout>
                                </TableCell>
                                <TableCell>
                                    <TableCellLayout>{item.model_name.clone()}</TableCellLayout>
                                </TableCell>
                                <TableCell>
                                    <TableCellLayout>{msg_count}</TableCellLayout>
                                </TableCell>
                                <TableCell>
                                    <TableCellLayout>{last_msg}</TableCellLayout>
                                </TableCell>
                                <TableCell>
                                    <TableCellLayout>{created}</TableCellLayout>
                                </TableCell>
                                <TableCell>
                                    <TableCellLayout>
                                        <Button
                                            size=ButtonSize::Small
                                            appearance=ButtonAppearance::Subtle
                                            on_click=move |_| handle_delete(id_for_delete.clone())
                                        >
                                            {icon("delete")}
                                        </Button>
                                    </TableCellLayout>
                                </TableCell>
                            </TableRow>
                        }
                    }).collect_view()}
                </TableBody>
            </Table>

            <Show when=move || show_create_modal.get()>
                {move || {
                    modal_stack.push_with_frame(
                        Some("max-width: min(600px, 95vw); width: min(600px, 95vw);".to_string()),
                        Some("llm-chat-create-modal".to_string()),
                        move |handle| {
                            view! {
                                <CreateChatModal
                                    on_saved=Callback::new({
                                        let handle = handle.clone();
                                        move |chat_id: String| {
                                            handle.close();
                                            handle_open_chat(chat_id);
                                            fetch();
                                        }
                                    })
                                    on_cancel=Callback::new({
                                        let handle = handle.clone();
                                        move |_| handle.close()
                                    })
                                />
                            }
                            .into_any()
                        },
                    );

                    show_create_modal.set(false);
                    view! { <></> }
                }}
            </Show>
        </div>
    }
}

#[component]
#[allow(non_snake_case)]
fn CreateChatModal(on_saved: Callback<String>, on_cancel: Callback<()>) -> impl IntoView {
    let (agents, set_agents) = signal::<Vec<LlmAgent>>(Vec::new());
    let new_chat_desc = RwSignal::new(String::new());
    let (selected_agent_id, set_selected_agent_id) = signal(String::new());
    let new_chat_model = RwSignal::new(String::new());
    let (available_models, set_available_models) = signal::<Vec<serde_json::Value>>(Vec::new());
    let is_models_dropdown_open = RwSignal::new(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (is_saving, set_is_saving) = signal(false);

    // Load agents
    Effect::new(move |_| {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_agents().await {
                Ok(v) => {
                    if selected_agent_id.get().is_empty() {
                        if let Some(first) = v.first() {
                            set_selected_agent_id.set(first.to_string_id());
                            new_chat_model.set(first.model_name.clone());

                            if let Some(models_json) = &first.available_models {
                                if let Ok(models) =
                                    serde_json::from_str::<Vec<serde_json::Value>>(models_json)
                                {
                                    set_available_models.set(models);
                                }
                            }
                        }
                    }
                    set_agents.set(v);
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    });

    let handle_save = move || {
        let desc = new_chat_desc.get();
        let agent_id = selected_agent_id.get();
        let model = new_chat_model.get();

        if desc.trim().is_empty() {
            set_error.set(Some("Введите название чата".to_string()));
            return;
        }
        if agent_id.trim().is_empty() {
            set_error.set(Some("Выберите агента".to_string()));
            return;
        }

        set_is_saving.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            match create_chat(&desc, &agent_id, &model).await {
                Ok(chat_id) => {
                    on_saved.run(chat_id);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_is_saving.set(false);
                }
            }
        });
    };

    view! {
        <div style="padding: 20px;">
            <h2 style="font-size: 18px; font-weight: bold; margin-bottom: 16px;">
                "Создать новый чат"
            </h2>

            {move || error.get().map(|e| view! {
                <div style="padding: 12px; margin-bottom: 16px; background: var(--color-error-50); border: 1px solid var(--color-error-100); border-radius: 8px;">
                    <span style="color: var(--color-error);">{e}</span>
                </div>
            })}

            <div style="display: flex; flex-direction: column; gap: 16px;">
                <div>
                    <label class="form__label">"Название чата"</label>
                    <Input value=new_chat_desc placeholder="Например: Анализ продаж" />
                </div>

                <div>
                    <label class="form__label">"Агент"</label>
                    <select
                        style="height: 32px; padding: 4px 8px; border: 1px solid var(--colorNeutralStroke2); border-radius: 6px; width: 100%;"
                        prop:value=move || selected_agent_id.get()
                        on:change=move |ev| {
                            let selected_id = event_target_value(&ev);
                            set_selected_agent_id.set(selected_id.clone());

                            if let Some(agent) = agents.get().iter().find(|a| a.to_string_id() == selected_id) {
                                new_chat_model.set(agent.model_name.clone());

                                if let Some(models_json) = &agent.available_models {
                                    if let Ok(models) = serde_json::from_str::<Vec<serde_json::Value>>(models_json) {
                                        set_available_models.set(models);
                                    } else {
                                        set_available_models.set(Vec::new());
                                    }
                                } else {
                                    set_available_models.set(Vec::new());
                                }
                                is_models_dropdown_open.set(false);
                            }
                        }
                    >
                        <For each=move || agents.get() key=|agent| agent.to_string_id() let:agent>
                            {{
                                let id = agent.to_string_id();
                                let desc = agent.base.description.clone();
                                view! { <option value=id>{desc}</option> }
                            }}
                        </For>
                    </select>
                </div>

                <div>
                    <label class="form__label">"Модель"</label>
                    <div style="position: relative;">
                        <Input
                            value=new_chat_model
                            placeholder="gpt-4o"
                            attr:style="width: 100%; padding-right: 0px;"
                        >
                            <InputSuffix slot>
                                <div style="display: flex; gap: 0px;">
                                    <Show when=move || !available_models.get().is_empty()>
                                        <Button
                                            appearance=ButtonAppearance::Subtle
                                            shape=ButtonShape::Square
                                            size=ButtonSize::Small
                                            on_click=move |_| {
                                                let is_open = is_models_dropdown_open.get();
                                                is_models_dropdown_open.set(!is_open);
                                            }
                                            attr:style="width: 28px; height: 28px; min-width: 28px; padding: 0; display: flex; align-items: center; justify-content: center;"
                                            attr:title="Выбрать из списка"
                                        >
                                            "▼"
                                        </Button>
                                    </Show>
                                </div>
                            </InputSuffix>
                        </Input>

                        {move || {
                            if !is_models_dropdown_open.get() || available_models.get().is_empty() {
                                return view! { <></> }.into_any();
                            }

                            let current = new_chat_model.get().to_lowercase();
                            let opts = available_models
                                .get()
                                .into_iter()
                                .filter_map(|m| {
                                    m.get("id").and_then(|v| v.as_str()).map(|s| s.to_string())
                                })
                                .filter(|model_id| {
                                    if current.trim().is_empty() {
                                        true
                                    } else {
                                        model_id.to_lowercase().contains(&current)
                                    }
                                })
                                .take(50)
                                .collect::<Vec<_>>();

                            view! {
                                <div style="position: absolute; top: calc(100% + 4px); left: 0; right: 0; max-height: 220px; overflow-y: auto; background: var(--color-surface); border: 1px solid var(--color-border); border-radius: var(--radius-md); box-shadow: var(--shadow-md); z-index: 1000;">
                                    {if opts.is_empty() {
                                        view! {
                                            <div style="padding: 8px 12px; color: var(--color-text-tertiary);">
                                                "Нет совпадений"
                                            </div>
                                        }
                                            .into_any()
                                    } else {
                                        opts.into_iter()
                                            .map(|opt| {
                                                let opt2 = opt.clone();
                                                view! {
                                                    <div
                                                        style="padding: 8px 12px; cursor: pointer; border-bottom: 1px solid var(--color-border-light);"
                                                        on:mousedown=move |_| {
                                                            new_chat_model.set(opt2.clone());
                                                            is_models_dropdown_open.set(false);
                                                        }
                                                    >
                                                        {opt}
                                                    </div>
                                                }
                                            })
                                            .collect_view()
                                            .into_any()
                                    }}
                                </div>
                            }
                                .into_any()
                        }}
                    </div>
                </div>

                <Flex justify=FlexJustify::End style="gap: 8px; margin-top: 8px;">
                    <Button appearance=ButtonAppearance::Secondary on_click=move |_| on_cancel.run(())>
                        {icon("close")}
                        " Отмена"
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Primary
                        disabled=is_saving
                        on_click=move |_| handle_save()
                    >
                        {icon("save")}
                        {move || if is_saving.get() { " Создание..." } else { " Создать" }}
                    </Button>
                </Flex>
            </div>
        </div>
    }
}

// API Functions

async fn fetch_chats_with_stats() -> Result<Vec<LlmChatListItem>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a018-llm-chat/with-stats", api_base());
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
    let data: Vec<LlmChatListItem> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
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

async fn create_chat(description: &str, agent_id: &str, model_name: &str) -> Result<String, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a018-llm-chat", api_base());
    let model_value: Option<&str> = if model_name.trim().is_empty() {
        None
    } else {
        Some(model_name)
    };

    let dto = serde_json::json!({
        "id": serde_json::Value::Null,
        "code": serde_json::Value::Null,
        "description": description,
        "comment": serde_json::Value::Null,
        "agent_id": agent_id,
        "model_name": model_value
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
    
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    let response: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    
    let chat_id = response
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "No chat ID in response".to_string())?
        .to_string();
    
    Ok(chat_id)
}

async fn delete_chat(id: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("DELETE");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a018-llm-chat/{}", api_base(), id);
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
    Ok(())
}
