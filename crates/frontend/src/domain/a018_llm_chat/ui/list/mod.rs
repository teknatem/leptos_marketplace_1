use crate::domain::a018_llm_chat::ui::pending_first_message_key;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::icons::icon;
use contracts::domain::a017_llm_agent::aggregate::{AgentType, LlmAgent};
use contracts::domain::a018_llm_chat::aggregate::LlmChatListItem;
use leptos::prelude::*;
use thaw::*;

/// Сформировать заголовок чата из первого вопроса пользователя.
/// Берёт первую непустую строку и обрезает до разумной длины.
fn derive_title(question: &str) -> String {
    let first_line = question
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim();
    if first_line.is_empty() {
        return "Новый чат".to_string();
    }
    const MAX_CHARS: usize = 70;
    let chars: Vec<char> = first_line.chars().collect();
    if chars.len() > MAX_CHARS {
        let truncated: String = chars.into_iter().take(MAX_CHARS).collect();
        format!("{}…", truncated.trim_end())
    } else {
        first_line.to_string()
    }
}

#[component]
#[allow(non_snake_case)]
pub fn LlmChatList() -> impl IntoView {
    let tabs_store = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    let (items, set_items) = signal::<Vec<LlmChatListItem>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);

    // Состояние быстрого создания чата (один вопрос — остальное опционально).
    let question = RwSignal::new(String::new());
    let (agents, set_agents) = signal::<Vec<LlmAgent>>(Vec::new());
    let (selected_agent_id, set_selected_agent_id) = signal(String::new());
    let model = RwSignal::new(String::new());
    let advanced_open = RwSignal::new(false);
    let (is_creating, set_is_creating) = signal(false);
    let (create_error, set_create_error) = signal::<Option<String>>(None);

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

    // Загрузить агентов и выбрать агента по умолчанию (основной, иначе первый).
    Effect::new(move |_| {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_agents().await {
                Ok(v) => {
                    if selected_agent_id.get_untracked().is_empty() {
                        if let Some(default) = v.iter().find(|a| a.is_primary).or_else(|| v.first())
                        {
                            set_selected_agent_id.set(default.to_string_id());
                            model.set(default.model_name.clone());
                        }
                    }
                    set_agents.set(v);
                }
                Err(e) => set_create_error.set(Some(e)),
            }
        });
    });

    let handle_open_chat = move |id: String, description: String| {
        use crate::layout::tabs::{detail_tab_label, pick_identifier};
        use contracts::domain::a018_llm_chat::ENTITY_METADATA as A018;
        let tab_key = format!("a018_llm_chat_details_{}", id);
        let identifier = pick_identifier(None, None, Some(&description), &id);
        let tab_label = detail_tab_label(A018.ui.element_name, identifier);
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

    // Быстрое создание: достаточно вопроса. Агент/модель — опционально (по умолчанию).
    let handle_create = move || {
        let q = question.get();
        if q.trim().is_empty() {
            set_create_error.set(Some("Введите вопрос или тему для LLM".to_string()));
            return;
        }
        let agent_id = selected_agent_id.get();
        if agent_id.trim().is_empty() {
            set_create_error.set(Some(
                "Нет доступного LLM-агента. Сначала создайте агента.".to_string(),
            ));
            return;
        }
        let model_name = model.get();
        let desc = derive_title(&q);

        set_is_creating.set(true);
        set_create_error.set(None);
        wasm_bindgen_futures::spawn_local(async move {
            match create_chat(&desc, &agent_id, &model_name).await {
                Ok(chat_id) => {
                    // Передать вопрос странице деталей чата для авто-отправки.
                    tabs_store.set_form_state(
                        pending_first_message_key(&chat_id),
                        serde_json::Value::String(q.clone()),
                    );
                    question.set(String::new());
                    advanced_open.set(false);
                    set_is_creating.set(false);
                    handle_open_chat(chat_id, desc.clone());
                    fetch();
                }
                Err(e) => {
                    set_create_error.set(Some(e));
                    set_is_creating.set(false);
                }
            }
        });
    };

    fetch();

    view! {
        <div id="a018_llm_chat--list" data-page-category="legacy" style="padding: 20px;">
            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center>
                <h1 style="font-size: 24px; font-weight: bold;">{"LLM Чаты"}</h1>
                <Space>
                    <Button
                        appearance=ButtonAppearance::Subtle
                        on_click=move |_| advanced_open.update(|v| *v = !*v)
                    >
                        {icon("settings")}
                        {move || if advanced_open.get() { " Скрыть настройки" } else { " Настройки" }}
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Primary
                        disabled=is_creating
                        on_click=move |_| handle_create()
                    >
                        {icon("plus")}
                        {move || if is_creating.get() { " Создание..." } else { " Создать чат" }}
                    </Button>
                    <Button appearance=ButtonAppearance::Secondary on_click=move |_| fetch()>
                        {icon("refresh")}
                        " Обновить"
                    </Button>
                </Space>
            </Flex>

            // ── Быстрое создание чата: достаточно вопроса ──────────────────────────
            <div style="margin-top: 16px; padding: 16px; background: var(--colorNeutralBackground2); border: 1px solid var(--colorNeutralStroke2); border-radius: 12px;">
                <Textarea
                    value=question
                    placeholder="Спросите LLM — например: «Выручка WB за май» или «Собери плагин для отчёта по остаткам». Ctrl+Enter — создать чат."
                    attr:style="width: 100%; min-height: 64px; max-height: 200px; resize: vertical;"
                    disabled=is_creating
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                        if ev.key() == "Enter" && ev.ctrl_key() {
                            ev.prevent_default();
                            handle_create();
                        }
                    }
                />

                <Show when=move || advanced_open.get()>
                    <Flex attr:style="margin-top: 12px; gap: 12px; flex-wrap: wrap; align-items: flex-end;">
                        <div style="display: flex; flex-direction: column; gap: 4px; width: 240px;">
                            <label class="form__label" style="font-size: 12px; margin: 0;">"Агент"</label>
                            <select
                                style="height: 32px; padding: 4px 8px; border: 1px solid var(--colorNeutralStroke2); border-radius: 6px; width: 100%;"
                                prop:value=move || selected_agent_id.get()
                                on:change=move |ev| {
                                    let selected_id = event_target_value(&ev);
                                    set_selected_agent_id.set(selected_id.clone());
                                    if let Some(agent) = agents.get().iter().find(|a| a.to_string_id() == selected_id) {
                                        model.set(agent.model_name.clone());
                                    }
                                }
                            >
                                <For each=move || agents.get() key=|agent| agent.to_string_id() let:agent>
                                    {{
                                        let id = agent.to_string_id();
                                        let type_label = agent_type_short_label(&agent.agent_type);
                                        let desc = format!("{} · {}", agent.base.description.clone(), type_label);
                                        view! { <option value=id>{desc}</option> }
                                    }}
                                </For>
                            </select>
                        </div>
                        <div style="display: flex; flex-direction: column; gap: 4px; width: 200px;">
                            <label class="form__label" style="font-size: 12px; margin: 0;">"Модель"</label>
                            <Input value=model placeholder="gpt-4o" attr:style="width: 100%;" />
                        </div>
                    </Flex>
                </Show>

                {move || create_error.get().map(|e| view! {
                    <div style="margin-top: 12px; color: var(--color-error);">{e}</div>
                })}
            </div>
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
                        <TableHeaderCell resizable=true min_width=140.0>"Тип агента"</TableHeaderCell>
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
                        let description_for_link = item.description.clone();

                        let msg_count = item.message_count.unwrap_or(0);
                        let last_msg = item.last_message_at.map(|dt| {
                            dt.format("%d.%m.%Y %H:%M").to_string()
                        }).unwrap_or_else(|| "-".to_string());
                        let created = item.created_at.format("%d.%m.%Y %H:%M").to_string();
                        let item_agent_type = AgentType::from_str(
                            item.agent_type.as_deref().unwrap_or("business_analyst")
                        );

                        view! {
                            <TableRow>
                                <TableCell>
                                    <TableCellLayout>
                                        <a
                                            href="#"
                                            style="color: var(--colorBrandForeground1); text-decoration: none; cursor: pointer;"
                                            on:click=move |e| {
                                                e.prevent_default();
                                                handle_open_chat(id_for_link.clone(), description_for_link.clone());
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
                                    <TableCellLayout>
                                        {agent_type_badge(&item_agent_type)}
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
        </div>
    }
}

// ─── Вспомогательные ─────────────────────────────────────────────────────────

fn agent_type_color(agent_type: &AgentType) -> &'static str {
    match agent_type {
        AgentType::BusinessAnalyst => "var(--colorBrandBackground)",
        AgentType::SystemAdmin => "#8b5cf6",
        AgentType::General => "#059669",
        AgentType::KbAdmin => "#0f766e",
        AgentType::PluginAdmin => "#d97706",
    }
}

/// Короткая подпись типа агента — чтобы бейдж в таблице не обрезался.
fn agent_type_short_label(agent_type: &AgentType) -> &'static str {
    match agent_type {
        AgentType::BusinessAnalyst => "Аналитик",
        AgentType::SystemAdmin => "Система",
        AgentType::General => "Общий",
        AgentType::KbAdmin => "База знаний",
        AgentType::PluginAdmin => "Плагины",
    }
}

fn agent_type_badge(agent_type: &AgentType) -> impl IntoView {
    let label = agent_type_short_label(agent_type);
    let color = agent_type_color(agent_type);
    let full = agent_type.display_name();
    view! {
        <span
            title=full
            style=format!(
                "display: inline-block; max-width: 100%; padding: 1px 8px; border-radius: 10px; \
                 font-size: 11px; font-weight: 600; line-height: 18px; color: #fff; background: {}; \
                 white-space: nowrap; overflow: hidden; text-overflow: ellipsis; vertical-align: middle;",
                color
            )
        >
            {label}
        </span>
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

async fn create_chat(
    description: &str,
    agent_id: &str,
    model_name: &str,
) -> Result<String, String> {
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
