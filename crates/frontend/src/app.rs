use crate::app_shell::AppShell;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::modal_stack::{ModalHost, ModalStackService};
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
    // Provide centralized modal stack for CRUD-details modals
    provide_context(ModalStackService::new());

    // Thaw UI theme - start with dark theme
    let theme = RwSignal::new(Theme::dark());

    // Provide Thaw theme context for ThemeSelect to update
    provide_context(ThawThemeContext(theme));

    view! {
        <ConfigProvider theme>
            <AuthProvider>
                <AppShell />
                <ModalHost />
            </AuthProvider>
        </ConfigProvider>
    }
}
