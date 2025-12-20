use crate::shared::icons::icon;
use leptos::ev;
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;

#[component]
pub fn Modal(
    /// Title of the modal
    title: String,
    /// Callback when modal should close
    on_close: Callback<()>,
    /// Optional action buttons (Save, Cancel, etc.) to display in header
    #[prop(optional)]
    action_buttons: Option<ChildrenFn>,
    /// Modal content
    children: Children,
) -> impl IntoView {
    // Handle Escape key
    Effect::new(move |_| {
        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            if let Some(keyboard_event) = event.dyn_ref::<KeyboardEvent>() {
                if keyboard_event.key() == "Escape" {
                    on_close.run(());
                }
            }
        }) as Box<dyn FnMut(_)>);

        if let Some(window) = web_sys::window() {
            let _ = window
                .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());
            closure.forget();
        }
    });

    // Handle overlay click
    let handle_overlay_click = move |_| {
        on_close.run(());
    };

    // Prevent click propagation from modal content
    let stop_propagation = move |ev: ev::MouseEvent| {
        ev.stop_propagation();
    };

    // Handle close button click
    let handle_close = move |_| {
        on_close.run(());
    };

    view! {
        <div class="modal-overlay" on:click=handle_overlay_click>
            <div class="modal" on:click=stop_propagation>
                <div class="modal-header">
                    <h2 class="modal-title">{title}</h2>
                    <div class="modal-header-actions">
                        {move || action_buttons.as_ref().map(|buttons| buttons())}
                        <button class="button button--icon modal__close" on:click=handle_close>
                            {icon("x")}
                        </button>
                    </div>
                </div>
                <div class="modal-body">
                    {children()}
                </div>
            </div>
        </div>
    }
}
