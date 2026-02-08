//! WindowsList component - displays open windows in the right panel.
//!
//! Shows a persistent list of all open tabs/windows with ability to switch
//! between them and close them.

use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use leptos::ev;
use leptos::prelude::*;

#[component]
pub fn WindowsList() -> impl IntoView {
    let ctx =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    // Switch to window
    let switch_window = move |key: String| {
        ctx.activate_tab(&key);
    };

    // Close window
    let close_window = move |key: String| {
        ctx.close_tab(&key);
    };

    view! {
        <div class="windows-list">
            <div class="windows-list__header">
                <h3>"Открытые окна"</h3>
                <span class="windows-list__count">
                    {move || {
                        let count = ctx.opened.get().len();
                        format!("({})", count)
                    }}
                </span>
            </div>

            <div class="windows-list__items">
                {move || {
                    let tabs = ctx.opened.get();
                    if tabs.is_empty() {
                        view! {
                            <div class="windows-list__empty">
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
                                            class="windows-list__item"
                                            class:windows-list__item--active=is_active
                                            on:click=move |_| {
                                                switch_window(key_for_switch.clone());
                                            }
                                        >
                                            <span class="windows-list__item-title">
                                                {tab.title.clone()}
                                            </span>
                                            <button
                                                class="windows-list__item-close"
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
        </div>
    }
}
