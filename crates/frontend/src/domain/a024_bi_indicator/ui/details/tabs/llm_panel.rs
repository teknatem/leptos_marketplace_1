//! LLM generation panel — collapsible sidebar for AI-assisted view creation

use super::super::view_model::BiIndicatorDetailsVm;
use crate::shared::icons::icon;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn LlmPanel(vm: BiIndicatorDetailsVm) -> impl IntoView {
    let is_open = vm.llm_panel_open;

    view! {
        <div
            class="bi-llm-panel"
            class:bi-llm-panel--open=move || is_open.get()
        >
            <button
                class="bi-llm-panel__toggle"
                on:click={
                    let vm = vm.clone();
                    move |_| vm.llm_panel_open.update(|v| *v = !*v)
                }
                title="LLM генерация"
            >
                {icon("sparkles")}
                <span class="bi-llm-panel__toggle-label">" AI"</span>
            </button>

            {move || {
                if is_open.get() {
                    Some(view! { <LlmPanelContent vm=vm.clone() /> })
                } else {
                    None
                }
            }}
        </div>
    }
}

#[component]
fn LlmPanelContent(vm: BiIndicatorDetailsVm) -> impl IntoView {
    let is_generating = vm.llm_generating;
    let history = vm.llm_history;

    let handle_generate = {
        let vm = vm.clone();
        move |_| vm.generate_view()
    };

    let handle_keydown = {
        let vm = vm.clone();
        move |ev: leptos::ev::KeyboardEvent| {
            if ev.key() == "Enter" && ev.ctrl_key() {
                ev.prevent_default();
                vm.generate_view();
            }
        }
    };

    view! {
        <div class="bi-llm-panel__content">
            <h4 class="bi-llm-panel__title">
                {icon("sparkles")} " LLM Генерация"
            </h4>

            <div class="bi-llm-panel__input-area">
                <Textarea
                    value=vm.llm_prompt
                    placeholder="Опишите желаемый вид индикатора...\nНапример: Большое число по центру, дельта зелёным/красным снизу"
                    attr:rows=4
                    attr:style="font-size: 13px; width: 100%;"
                    on:keydown=handle_keydown
                />
                <div class="bi-llm-panel__actions">
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=handle_generate
                        disabled=Signal::derive(move || is_generating.get())
                    >
                        {move || if is_generating.get() {
                            view! { <span>"Генерация..."</span> }.into_any()
                        } else {
                            view! { <span>{icon("sparkles")} " Сгенерировать"</span> }.into_any()
                        }}
                    </Button>
                    <span class="bi-llm-panel__hint">"Ctrl+Enter"</span>
                </div>
            </div>

            {move || vm.llm_error.get().map(|e| view! {
                <div class="bi-llm-panel__error">
                    {e}
                </div>
            })}

            <div class="bi-llm-panel__history">
                {move || {
                    let items = history.get();
                    if items.is_empty() {
                        view! {
                            <p class="bi-llm-panel__empty">
                                "Результатов генерации пока нет"
                            </p>
                        }.into_any()
                    } else {
                        view! {
                            <div class="bi-llm-panel__history-list">
                                {items.iter().rev().enumerate().map(|(idx, entry)| {
                                    let entry_clone = entry.clone();
                                    let vm_apply = vm.clone();
                                    view! {
                                        <div class="bi-llm-panel__history-item">
                                            <div class="bi-llm-panel__history-prompt">
                                                <strong>"#" {items.len() - idx}</strong>
                                                ": "
                                                {entry.prompt.clone()}
                                            </div>
                                            <div class="bi-llm-panel__history-explanation">
                                                {entry.explanation.clone()}
                                            </div>
                                            <Button
                                                appearance=ButtonAppearance::Secondary
                                                size=ButtonSize::Small
                                                on_click=move |_| {
                                                    vm_apply.apply_generation(&entry_clone);
                                                }
                                            >
                                                {icon("check")} " Применить"
                                            </Button>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        }.into_any()
                    }
                }}
            </div>
        </div>
    }
}
