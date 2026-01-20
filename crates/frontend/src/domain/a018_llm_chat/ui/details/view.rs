//! LLM Chat Details - View Component

use super::artifact_card::ArtifactCard;
use super::model::{fetch_chat, fetch_messages, send_message};
use super::view_model::LlmChatDetailsVm;
use crate::shared::icons::icon;
use contracts::domain::common::AggregateId;
use leptos::prelude::*;
use thaw::*;

#[component]
#[allow(non_snake_case)]
pub fn LlmChatDetails(id: String, on_close: Callback<()>) -> impl IntoView {
    let vm = LlmChatDetailsVm::new();
    let chat_id = id.clone();

    // Load chat and messages
    Effect::new({
        let chat_id = chat_id.clone();
        move |_| {
            let chat_id = chat_id.clone();
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
                    }
                    Err(e) => vm.error.set(Some(e)),
                }
            });
        }
    });

    // Send message handler - using Callback to avoid move issues
    let handle_send = Callback::new({
        let chat_id = chat_id.clone();
        move |_| {
            let content = vm.new_message.get();
            if content.trim().is_empty() {
                return;
            }

            vm.is_sending.set(true);
            vm.new_message.set(String::new());

            let chat_id = chat_id.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match send_message(&chat_id, &content).await {
                    Ok(_) => {
                        // Reload messages
                        match fetch_messages(&chat_id).await {
                            Ok(msgs) => {
                                vm.messages.set(msgs);
                                vm.error.set(None);
                            }
                            Err(e) => vm.error.set(Some(e)),
                        }
                        vm.is_sending.set(false);
                    }
                    Err(e) => {
                        vm.error.set(Some(e));
                        vm.is_sending.set(false);
                    }
                }
            });
        }
    });

    view! {
        <div style="height: 100%; display: flex; flex-direction: column; padding: 20px;">
            // Header - 1 строка
            <Flex
                justify=FlexJustify::SpaceBetween
                align=FlexAlign::Center
                style="margin-bottom: 16px; padding-bottom: 12px; border-bottom: 1px solid var(--colorNeutralStroke2);"
            >
                <Flex align=FlexAlign::Center style="gap: 16px;">
                    <h2 style="font-size: 18px; font-weight: bold;">
                        {move || {
                            vm.chat
                                .get()
                                .map(|c| c.base.description.clone())
                                .unwrap_or_else(|| "Загрузка...".to_string())
                        }}
                    </h2>
                    <span style="color: var(--colorNeutralForeground3); font-size: 14px;">
                        {move || {
                            vm.chat
                                .get()
                                .map(|c| format!("Агент: {}", c.agent_id.as_string()))
                                .unwrap_or_default()
                        }}
                    </span>
                    <span style="color: var(--colorNeutralForeground3); font-size: 14px;">
                        {move || {
                            vm.chat
                                .get()
                                .map(|c| format!("Модель: {}", c.model_name))
                                .unwrap_or_default()
                        }}
                    </span>
                    <span style="color: var(--colorNeutralForeground3); font-size: 14px;">
                        {move || format!("Сообщений: {}", vm.messages.get().len())}
                    </span>
                </Flex>
                <Button
                    appearance=ButtonAppearance::Secondary
                    on_click=move |_| on_close.run(())
                >
                    {icon("close")}
                    " Закрыть"
                </Button>
            </Flex>

            // Error display
            {move || {
                vm.error
                    .get()
                    .map(|e| {
                        view! {
                            <div style="padding: 12px; margin-bottom: 16px; background: var(--color-error-50); border: 1px solid var(--color-error-100); border-radius: 8px;">
                                <span style="color: var(--color-error);">{e}</span>
                            </div>
                        }
                    })
            }}

            // Messages area
            <div
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
                        let artifact_id = msg.artifact_id.as_ref().map(|id| id.as_string());
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
                                    artifact_id
                                        .clone()
                                        .map(|id| view! { <ArtifactCard artifact_id=id /> })
                                }}
                            </div>
                        }
                    }}
                </For>
            </div>

            // Input area
            <Flex style="gap: 8px; align-items: flex-end;">
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
                    appearance=ButtonAppearance::Primary
                    disabled=vm.is_sending
                    on_click=move |_| handle_send.run(())
                >
                    {icon("send")}
                    {move || if vm.is_sending.get() { " Отправка..." } else { " Отправить" }}
                </Button>
            </Flex>
        </div>
    }
}
