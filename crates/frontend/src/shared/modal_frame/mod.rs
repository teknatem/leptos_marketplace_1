use leptos::ev;
use leptos::prelude::*;
use gloo_timers::future::TimeoutFuture;
use wasm_bindgen_futures::spawn_local;

/// Modal frame container (overlay + positioned surface).
///
/// Important: this component intentionally DOES NOT render a header or action buttons.
/// CRUD-details screens render their own compact header so they look identical in a modal and in a tab.
#[component]
pub fn ModalFrame(
    /// Called when the modal should close (overlay click, close by host, etc.).
    on_close: Callback<()>,
    /// Close when clicking on the overlay (default: true).
    #[prop(optional)]
    close_on_overlay: Option<bool>,
    /// z-index for overlay stacking (default: 1000).
    #[prop(optional)]
    z_index: Option<i32>,
    /// Extra class for the modal surface (`div.modal`).
    #[prop(optional)]
    modal_class: Option<String>,
    /// Extra style for the modal surface (`div.modal`).
    #[prop(optional)]
    modal_style: Option<String>,
    /// Extra style for overlay (`div.modal-overlay`).
    #[prop(optional)]
    overlay_style: Option<String>,
    children: Children,
) -> impl IntoView {
    let close_on_overlay = close_on_overlay.unwrap_or(true);
    let z_index = z_index.unwrap_or(1000);
    let overlay_mouse_down = RwSignal::new(false);

    let is_direct_overlay_event = |ev: &ev::MouseEvent| -> bool {
        match (ev.target(), ev.current_target()) {
            (Some(t), Some(ct)) => t == ct,
            _ => false,
        }
    };

    // We only close if both press and release happened on the overlay itself.
    // This prevents closing when user selects text inside the modal and releases the mouse outside.
    let handle_overlay_mouse_down = {
        let is_direct_overlay_event = is_direct_overlay_event;
        move |ev: ev::MouseEvent| {
            overlay_mouse_down.set(is_direct_overlay_event(&ev));
        }
    };

    let handle_overlay_click = {
        let is_direct_overlay_event = is_direct_overlay_event;
        move |ev: ev::MouseEvent| {
            let should_close =
                close_on_overlay && overlay_mouse_down.get() && is_direct_overlay_event(&ev);
            overlay_mouse_down.set(false);
            if should_close {
                // Defer close to next tick: avoids Leptos event delegation calling a dropped handler
                // when the overlay is removed synchronously during its own click dispatch.
                let on_close = on_close;
                spawn_local(async move {
                    TimeoutFuture::new(0).await;
                    on_close.run(());
                });
            }
        }
    };

    let stop_propagation = move |ev: ev::MouseEvent| {
        ev.stop_propagation();
    };

    let overlay_style_full = move || {
        let extra = overlay_style.clone().unwrap_or_default();
        if extra.is_empty() {
            format!("z-index: {z_index};")
        } else {
            format!("z-index: {z_index}; {extra}")
        }
    };

    let modal_style_full = move || {
        let extra = modal_style.clone().unwrap_or_default();
        if extra.is_empty() {
            "position: relative;".to_string()
        } else {
            format!("position: relative; {extra}")
        }
    };

    view! {
        <div
            class="modal-overlay"
            style=overlay_style_full
            on:mousedown=handle_overlay_mouse_down
            on:click=handle_overlay_click
        >
            <div
                class=move || {
                    if let Some(cls) = modal_class.clone() {
                        format!("modal {cls}")
                    } else {
                        "modal".to_string()
                    }
                }
                style=modal_style_full
                on:click=stop_propagation
            >
                {children()}
            </div>
        </div>
    }
}


