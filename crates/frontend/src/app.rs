use crate::layout::global_context::AppGlobalContext;
use crate::routes::routes::AppRoutes;
use crate::shared::picker_aggregate::ModalService;
use crate::system::auth::context::AuthProvider;
use leptos::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    // Provide the AppGlobalContext store to the whole app via context.
    provide_context(AppGlobalContext::new());
    // Provide ModalService for picker components
    provide_context(ModalService::new());

    view! {
        <AuthProvider>
            <AppRoutes />
        </AuthProvider>
    }
}
