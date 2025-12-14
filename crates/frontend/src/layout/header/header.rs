use crate::layout::global_context::AppGlobalContext;
use crate::layout::header::mega_menu::MegaMenuBar;
use leptos::prelude::*;

#[component]
pub fn Header() -> impl IntoView {
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");

    view! {
        <header data-zone="header" class="header">
            <div class="header__content">
                <span class="header__title">"Integrator"</span>
                <MegaMenuBar />
            </div>
            <div class="header__actions">
                <button class="button button--ghost" aria-label="Toggle Left" on:click=move |_| tabs_store.toggle_left()>{"◀"}</button>
                <button class="button button--ghost" aria-label="Toggle Right" on:click=move |_| tabs_store.toggle_right()>{"▶"}</button>
            </div>
        </header>
    }
}
