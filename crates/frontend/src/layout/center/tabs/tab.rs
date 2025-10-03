use crate::layout::global_context::{AppGlobalContext, Tab as TabData};
use leptos::ev;
use leptos::prelude::*;

#[component]
pub fn Tab(tab: TabData) -> impl IntoView {
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");

    let tab_for_active = tab.clone();
    let is_active =
        Memo::new(move |_| tabs_store.active.get().as_deref() == Some(&tab_for_active.key));

    let tab_for_click = tab.clone();
    let on_click = move |_| tabs_store.activate_tab(&tab_for_click.key);

    let tab_for_close = tab.clone();
    let on_close = move |ev: ev::MouseEvent| {
        ev.stop_propagation();
        tabs_store.close_tab(&tab_for_close.key);
    };

    view! {
        <div class="tab" class:active=is_active on:click=on_click>
            <span>{tab.title}</span>
            {if tab.dirty { " (*)" } else { "" }}
            {if tab.pinned { " (P)" } else { "" }}
            <button class="tab-close" on:click=on_close>"×"</button>
        </div>
    }
}
