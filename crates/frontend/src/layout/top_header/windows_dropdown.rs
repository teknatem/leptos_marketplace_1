//! WindowsDropdown component - manages open windows via dropdown menu.
//!
//! Provides a compact dropdown interface for switching between and managing
//! multiple open windows in the application.

use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use leptos::ev;
use leptos::prelude::*;

#[component]
pub fn WindowsDropdown() -> impl IntoView {
    let ctx =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    let is_open = RwSignal::new(false);

    // Get active window title
    let active_title = move || {
        let active_key = ctx.active.get();
        if let Some(key) = active_key {
            ctx.opened
                .get()
                .iter()
                .find(|tab| tab.key == key)
                .map(|tab| tab.title.clone())
                .unwrap_or_else(|| "Окна".to_string())
        } else {
            "Окна".to_string()
        }
    };

    // Get opened windows count
    let windows_count = move || ctx.opened.get().len();

    // Toggle dropdown
    let toggle_dropdown = move |_| {
        is_open.update(|open| *open = !*open);
    };

    // Switch to window
    let switch_window = move |key: String| {
        ctx.activate_tab(&key);
        is_open.set(false);
    };

    // Close window
    let close_window = move |key: String| {
        ctx.close_tab(&key);
    };

    view! {
        <div class="windows-dropdown">
            // Trigger button
            <button
                class="windows-dropdown__trigger"
                on:click=toggle_dropdown
                title="Управление окнами"
            >
                <span class="windows-dropdown__trigger-text">
                    {move || {
                        let count = windows_count();
                        if count > 0 {
                            format!("{} ({})", active_title(), count)
                        } else {
                            "Окна (0)".to_string()
                        }
                    }}
                </span>
                {move || if is_open.get() {
                    icon("chevron-up")
                } else {
                    icon("chevron-down")
                }}
            </button>

            // Dropdown menu
            <Show when=move || is_open.get()>
                <div class="windows-dropdown__menu">
                    {move || {
                        let tabs = ctx.opened.get();
                        if tabs.is_empty() {
                            view! {
                                <div class="windows-dropdown__empty">
                                    "Нет открытых окон"
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div>
                                    {tabs.into_iter().map(|tab| {
                                        let is_active = ctx.active.get().as_ref() == Some(&tab.key);
                                        let key_for_switch = tab.key.clone();
                                        let key_for_close = tab.key.clone();

                                        view! {
                                            <div
                                                class="windows-dropdown__item"
                                                class:windows-dropdown__item--active=is_active
                                                on:click=move |_| {
                                                    switch_window(key_for_switch.clone());
                                                }
                                            >
                                                <span class="windows-dropdown__item-title">
                                                    {tab.title.clone()}
                                                </span>
                                                <button
                                                    class="windows-dropdown__item-close"
                                                    on:click=move |ev: ev::MouseEvent| {
                                                        ev.stop_propagation();
                                                        close_window(key_for_close.clone());
                                                    }
                                                    title="Закрыть окно"
                                                >
                                                    {icon("x")}
                                                </button>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            }.into_any()
                        }
                    }}
                </div>
            </Show>
        </div>
    }
}
