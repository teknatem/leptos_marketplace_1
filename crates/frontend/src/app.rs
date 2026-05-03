use crate::app_shell::AppShell;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::modal_stack::{KeydownGuard, ModalHost, ModalStackService};
use crate::system::auth::context::AuthProvider;
use js_sys::Function;
use leptos::prelude::*;
use thaw::{ConfigProvider, Theme};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;

/// Context for Thaw UI theme management
#[derive(Clone, Copy)]
pub struct ThawThemeContext(pub RwSignal<Theme>);

#[component]
pub fn App() -> impl IntoView {
    // Provide the AppGlobalContext store to the whole app via context.
    provide_context(AppGlobalContext::new());
    // Provide centralized modal stack for CRUD-details modals
    provide_context(ModalStackService::new());

    let tabs_store = use_context::<AppGlobalContext>().unwrap();
    let modal_svc = use_context::<ModalStackService>().unwrap();

    // Global ESC handler: close the active tab when Escape is pressed,
    // unless a modal is open (modal handles its own ESC) or the tab is dirty.
    let _esc_guard = StoredValue::new_local(web_sys::window().map(|window| {
        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            if let Some(ke) = event.dyn_ref::<KeyboardEvent>() {
                if ke.key() == "Escape" && !modal_svc.is_open() {
                    if let Some(active_key) = tabs_store.active.get_untracked() {
                        let is_dirty = tabs_store.opened.with_untracked(|tabs| {
                            tabs.iter()
                                .find(|t| t.key == active_key)
                                .map(|t| t.dirty)
                                .unwrap_or(false)
                        });
                        if !is_dirty {
                            tabs_store.close_tab(&active_key);
                        }
                    }
                }
            }
        }) as Box<dyn FnMut(_)>);

        let js_fn = closure.as_ref().unchecked_ref::<Function>().clone();
        let _ = window.add_event_listener_with_callback("keydown", &js_fn);

        KeydownGuard {
            window,
            js_fn,
            _closure: closure,
        }
    }));

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
