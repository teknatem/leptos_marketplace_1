use crate::layout::global_context::AppGlobalContext;
use crate::routes::routes::AppRoutes;
use crate::shared::picker_aggregate::ModalService;
use crate::system::auth::context::AuthProvider;
use leptos::prelude::*;
use thaw::{ConfigProvider, Theme};

/// Context for Thaw UI theme management
#[derive(Clone, Copy)]
pub struct ThawThemeContext(pub RwSignal<Theme>);

#[component]
pub fn App() -> impl IntoView {
    // Provide the AppGlobalContext store to the whole app via context.
    provide_context(AppGlobalContext::new());
    // Provide ModalService for picker components
    provide_context(ModalService::new());

    // Thaw UI theme - start with dark theme
    let theme = RwSignal::new(Theme::dark());

    // Provide Thaw theme context for ThemeSelect to update
    provide_context(ThawThemeContext(theme));

    view! {
        <ConfigProvider theme>
            <AuthProvider>
                <AppRoutes />
            </AuthProvider>
        </ConfigProvider>
    }
}
