use crate::layout::global_context::AppGlobalContext;
use crate::layout::ModalService;
use crate::routes::routes::AppRoutes;
use leptos::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    // Provide the AppGlobalContext store to the whole app via context.
    provide_context(AppGlobalContext::new());
    
    // Provide ModalService for centralized modal management
    provide_context(ModalService::new());

    view! {
        <AppRoutes />
    }
}
