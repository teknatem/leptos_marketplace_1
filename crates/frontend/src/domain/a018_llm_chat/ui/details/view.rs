//! LLM Chat Details - View Component
//!
//! Унифицирован с detail-страницами: PageFrame, page__header, page__content.
//! Агент отображается по имени (agent_name из API), а не по UUID.

use super::artifact_card::ArtifactCard;
use super::model::{fetch_chat, fetch_messages, poll_until_done, send_message};
use super::tool_calls_trace::ToolCallsTrace;
use super::view_model::LlmChatDetailsVm;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_DETAIL;
use contracts::domain::a018_llm_chat::aggregate::LlmChatMessage;
use contracts::domain::common::AggregateId;
use leptos::prelude::*;
use thaw::*;
use uuid::Uuid;

#[component]
#[allow(non_snake_case)]
pub fn LlmChatDetails(id: String, on_close: Callback<()>) -> impl IntoView {
    let vm = LlmChatDetailsVm::new();
    let chat_id = id.clone();
    let messages_container_ref = NodeRef::<leptos::html::Div>::new();

    // Scroll to bottom helper
    let scroll_to_bottom = {
        let messages_container_ref = messages_container_ref.clone();
        move || {
            if let Some(container) = messages_container_ref.get() {
                request_animation_frame(move || {
                    container.set_scroll_top(container.scroll_height());
                });
            }
        }
    };

    // Load chat and messages
    Effect::new({
        let chat_id = chat_id.clone();
        let scroll_to_bottom = scroll_to_bottom.clone();
        move |_| {
            let chat_id = chat_id.clone();
            let scroll_to_bottom = scroll_to_bottom.clone();
            wasm_bindgen_futures::spawn_local(async move {
                // Load chat
                match fetch_chat(&chat_id).await {
                    Ok(chat) => {
                        vm.chat.set(Some(chat));
                    }
                    Err(e) => vm.error.set(Some(e)),
                }

                // Load messages
                match fetch_messages(&chat_id).await {
                    Ok(msgs) => {
                        vm.messages.set(msgs);
                        vm.error.set(None);
                        scroll_to_bottom();
                    }
                    Err(e) => vm.error.set(Some(e)),
                }
            });
        }
    });

    // Send message handler - using Callback to avoid move issues
    let handle_send = Callback::new({
        let chat_id = chat_id.clone();
        let scroll_to_bottom = scroll_to_bottom.clone();
        move |_| {
            let content = vm.new_message.get();
            if content.trim().is_empty() {
                return;
            }

            vm.is_sending.set(true);
            vm.new_message.set(String::new());

            // Create optimistic user message
            let chat_uuid = Uuid::parse_str(&chat_id).unwrap_or_else(|_| Uuid::new_v4());
            let chat_id_obj =
                contracts::domain::a018_llm_chat::aggregate::LlmChatId::new(chat_uuid);
            let optimistic_msg = LlmChatMessage::user(chat_id_obj, content.clone());
            let optimistic_id = optimistic_msg.id;

            // Add optimistic message immediately
            let mut current_msgs = vm.messages.get();
            current_msgs.push(optimistic_msg);
            vm.messages.set(current_msgs);
            scroll_to_bottom();

            let chat_id = chat_id.clone();
            let scroll_to_bottom = scroll_to_bottom.clone();
            let attachment_ids = vm
                .uploaded_files
                .get()
                .iter()
                .map(|f| f.id.clone())
                .collect();
            wasm_bindgen_futures::spawn_local(async move {
                // 1. POST → immediately get job_id (server returns 202)
                let job_id = match send_message(&chat_id, &content, attachment_ids).await {
                    Ok(id) => id,
                    Err(e) => {
                        vm.error.set(Some(format!("Ошибка отправки: {}", e)));
                        vm.is_sending.set(false);
                        return;
                    }
                };

                // 2. Poll every 2s until done or error (max 90 attempts = 3 min)
                let poll_result = poll_until_done(&job_id, 90, 2000).await;

                // 3. Always reload messages from DB after completion
                match fetch_messages(&chat_id).await {
                    Ok(msgs) => {
                        vm.messages.set(msgs);
                        scroll_to_bottom();
                    }
                    Err(_) => {
                        let mut current_msgs = vm.messages.get();
                        current_msgs.retain(|msg| msg.id != optimistic_id);
                        vm.messages.set(current_msgs);
                    }
                }

                match poll_result {
                    Ok(_) => {
                        vm.uploaded_files.set(Vec::new());
                        vm.error.set(None);
                    }
                    Err(e) => {
                        vm.error.set(Some(format!("Ошибка LLM: {}", e)));
                    }
                }

                vm.is_sending.set(false);
            });
        }
    });

    view! {
        <PageFrame page_id="a018_llm_chat--detail" category=PAGE_CAT_DETAIL class="a018-llm-chat-detail">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">
                        {move || {
                            vm.chat
                                .get()
                                .map(|c| c.chat.base.description.clone())
                                .unwrap_or_else(|| "Загрузка...".to_string())
                        }}
                    </h1>
                    <span class="page__header-meta">
                        {move || {
                            vm.chat.get().map(|c| {
                                let agent_display = c.agent_name.clone().unwrap_or_else(|| c.chat.agent_id.as_string());
                                format!("Агент: {}", agent_display)
                            }).unwrap_or_default()
                        }}
                    </span>
                    <span class="page__header-meta">
                        {move || {
                            vm.chat
                                .get()
                                .map(|c| format!("Модель: {}", c.chat.model_name))
                                .unwrap_or_default()
                        }}
                    </span>
                    <span class="page__header-meta">
                        {move || format!("Сообщений: {}", vm.messages.get().len())}
                    </span>
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| on_close.run(())
                    >
                        {icon("x")}
                        " Закрыть"
                    </Button>
                </div>
            </div>

            <div class="page__content" style="display: flex; flex-direction: column; min-height: 0;">
                // Error display
                {move || {
                    vm.error
                        .get()
                        .map(|e| {
                            view! {
                                <div class="warning-box" style="background: var(--color-error-50); border-color: var(--color-error-100); margin-bottom: var(--spacing-md);">
                                    <span class="warning-box__text" style="color: var(--color-error);">{e}</span>
                                </div>
                            }
                        })
                }}

                // Messages area
                <div
                node_ref=messages_container_ref
                style="flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 12px; margin-bottom: 16px; padding: 12px; background: var(--colorNeutralBackground1); border: 1px solid var(--colorNeutralStroke2); border-radius: 8px;"
            >
                <For
                    each=move || vm.messages.get()
                    key=|msg| msg.id.to_string()
                    let:msg
                >
                    {{
                        let is_user = matches!(
                            msg.role,
                            contracts::domain::a018_llm_chat::aggregate::ChatRole::User
                        );
                        let tokens = msg.tokens_used;
                        let model = msg.model_name.clone();
                        let conf = msg.confidence;
                        let duration = msg.duration_ms;
                        let artifact_id = msg.artifact_id.as_ref().map(|id| id.as_string());
                        let tool_trace = msg.tool_trace.clone();
                        view! {
                            <div
                                style=if is_user {
                                    "align-self: flex-end; max-width: 70%;"
                                } else {
                                    "align-self: flex-start; max-width: 70%;"
                                }
                            >
                                <div
                                    style=if is_user {
                                        "background: var(--colorBrandBackground2); padding: 10px 14px; border-radius: 12px;"
                                    } else {
                                        "background: var(--colorNeutralBackground2); padding: 10px 14px; border-radius: 12px;"
                                    }
                                >
                                    <div style="white-space: pre-wrap;">{msg.content.clone()}</div>
                                    {move || {
                                        let mut meta_parts = Vec::new();
                                        if let Some(t) = tokens {
                                            meta_parts.push(format!("🎫 {} tokens", t));
                                        }
                                        if let Some(m) = &model {
                                            meta_parts.push(format!("🤖 {}", m));
                                        }
                                        if let Some(d) = duration {
                                            meta_parts.push(format!("⏱ {:.1}s", d as f64 / 1000.0));
                                        }
                                        if let Some(c) = conf {
                                            meta_parts.push(format!("📊 {:.1}%", c * 100.0));
                                        }
                                        if !meta_parts.is_empty() {
                                            Some(
                                                view! {
                                                    <div style="font-size: 11px; opacity: 0.7; margin-top: 6px;">
                                                        {meta_parts.join(" • ")}
                                                    </div>
                                                },
                                            )
                                        } else {
                                            None
                                        }
                                    }}
                                </div>

                                {move || {
                                    if !is_user {
                                        Some(view! { <ToolCallsTrace tool_trace=tool_trace.clone() /> })
                                    } else {
                                        None
                                    }
                                }}
                                {move || {
                                    artifact_id
                                        .clone()
                                        .map(|id| view! { <ArtifactCard artifact_id=id /> })
                                }}
                            </div>
                        }
                    }}
                </For>

                // Loading indicator — показывается пока LLM обрабатывает запрос
                {move || {
                    if vm.is_sending.get() {
                        Some(view! {
                            <div class="chat-typing" style="align-self: flex-start; max-width: 70%;">
                                <div class="chat-typing__bubble">
                                    <span class="chat-typing__dot"></span>
                                    <span class="chat-typing__dot"></span>
                                    <span class="chat-typing__dot"></span>
                                    <span class="chat-typing__label">" LLM обрабатывает запрос..."</span>
                                </div>
                            </div>
                        })
                    } else {
                        None
                    }
                }}
                </div>

                // Input area
                <div style="display: flex; flex-direction: column; gap: 8px;">
                // File attachments display
                {move || {
                    let files = vm.uploaded_files.get();
                    if !files.is_empty() {
                        Some(
                            view! {
                                <Flex style="gap: 8px; flex-wrap: wrap;">
                                    <For
                                        each=move || vm.uploaded_files.get()
                                        key=|f| f.id.clone()
                                        let:file
                                    >
                                        <div style="padding: 6px 12px; background: var(--colorNeutralBackground2); border: 1px solid var(--colorNeutralStroke2); border-radius: 6px; display: flex; align-items: center; gap: 8px;">
                                            <span style="font-size: 14px;">
                                                {icon("document")}
                                                " "
                                                {file.filename.clone()}
                                            </span>
                                            <button
                                                style="background: none; border: none; cursor: pointer; padding: 2px; color: var(--colorNeutralForeground3);"
                                                on:click={
                                                    let file_id = file.id.clone();
                                                    move |_| {
                                                        let mut files = vm.uploaded_files.get();
                                                        files.retain(|f| f.id != file_id);
                                                        vm.uploaded_files.set(files);
                                                    }
                                                }
                                            >
                                                {icon("close")}
                                            </button>
                                        </div>
                                    </For>
                                </Flex>
                            },
                        )
                    } else {
                        None
                    }
                }}

                <Flex style="gap: 8px; align-items: flex-end;">
                    <input
                        type="file"
                        accept=".txt,.md,.rs,.toml,.json,.sql,.js,.ts,.py,.go,.java,.c,.cpp,.h,.hpp,.cs,.rb,.php,.html,.css,.xml,.yaml,.yml"
                        style="display: none;"
                        id="file-input"
                        on:change={
                            let chat_id = chat_id.clone();
                            move |ev| {
                                use wasm_bindgen::JsCast;
                                let input: web_sys::HtmlInputElement = ev.target().unwrap().dyn_into().unwrap();
                                if let Some(files) = input.files() {
                                    if let Some(file) = files.get(0) {
                                        let chat_id = chat_id.clone();
                                        wasm_bindgen_futures::spawn_local(async move {
                                            match super::model::upload_file(&chat_id, file).await {
                                                Ok(file_info) => {
                                                    let mut uploaded = vm.uploaded_files.get();
                                                    uploaded.push(file_info);
                                                    vm.uploaded_files.set(uploaded);
                                                }
                                                Err(e) => {
                                                    vm.error.set(Some(format!("Ошибка загрузки файла: {}", e)));
                                                }
                                            }
                                        });
                                    }
                                }
                                // Clear input
                                input.set_value("");
                            }
                        }
                    />

                    <div style="flex: 1;">
                        <Textarea
                            value=vm.new_message
                            placeholder="Введите сообщение... (Ctrl+Enter для отправки)"
                            attr:style="width: 100%; min-height: 60px; max-height: 200px; resize: vertical;"
                            disabled=vm.is_sending
                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                if ev.key() == "Enter" && ev.ctrl_key() {
                                    ev.prevent_default();
                                    handle_send.run(());
                                }
                            }
                        />
                    </div>

                    <Button
                        appearance=ButtonAppearance::Secondary
                        disabled=vm.is_sending
                        on_click=move |_| {
                            if let Some(window) = web_sys::window() {
                                if let Some(document) = window.document() {
                                    if let Some(input) = document.get_element_by_id("file-input") {
                                        use wasm_bindgen::JsCast;
                                        if let Ok(input) = input.dyn_into::<web_sys::HtmlElement>() {
                                            input.click();
                                        }
                                    }
                                }
                            }
                        }
                    >
                        {icon("attach")}
                    </Button>

                    <Button
                        appearance=ButtonAppearance::Primary
                        disabled=vm.is_sending
                        on_click=move |_| handle_send.run(())
                    >
                        {icon("send")}
                        {move || if vm.is_sending.get() { " Отправка..." } else { " Отправить" }}
                    </Button>
                </Flex>
                </div>
            </div>
        </PageFrame>
    }
}
