use leptos::prelude::*;

#[component]
pub fn Center(children: Children) -> impl IntoView {
    view! {
        <div data-zone="center" class="app-tabs" style="flex: 1; overflow: auto;">
            {children()}
        </div>
    }
}
