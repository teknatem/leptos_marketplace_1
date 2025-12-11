use crate::layout::global_context::AppGlobalContext;
use leptos::prelude::*;

#[component]
pub fn Left(children: Children) -> impl IntoView {
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");
    let is_open = move || tabs_store.left_open.get();

    view! {
        <div data-zone="left" class="left" class:hidden=move || !is_open()>
            {children()}
        </div>
    }
}
