use crate::layout::global_context::AppGlobalContext;
use leptos::prelude::*;

#[component]
pub fn Header() -> impl IntoView {
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");

    view! {
        <header data-zone="header" class="header">
            <div class="header-left">
                <span class="header-title">"Integrator"</span>
            </div>
            <div class="header-actions">
                <button class="icon-btn" aria-label="Toggle Left" on:click=move |_| tabs_store.toggle_left()>{"◀"}</button>
                <button class="icon-btn" aria-label="Toggle Right" on:click=move |_| tabs_store.toggle_right()>{"▶"}</button>
            </div>
        </header>
    }
}
