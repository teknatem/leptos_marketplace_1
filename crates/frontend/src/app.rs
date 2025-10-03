use crate::layout::global_context::AppGlobalContext;
use crate::routes::routes::AppRoutes;
use leptos::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    // Provide the AppGlobalContext store to the whole app via context.
    provide_context(AppGlobalContext::new());

    view! {
        <AppRoutes />
    }
}
