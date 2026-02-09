use crate::layout::global_context::AppGlobalContext;
use leptos::prelude::*;

#[component]
pub fn Center(children: Children) -> impl IntoView {
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");

    view! {
        <div data-zone="center" class="app-tabs" style="flex: 1; overflow: auto;">
            {children()}
        </div>
    }
}
