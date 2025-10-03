use crate::layout::global_context::AppGlobalContext;
use leptos::prelude::*;

#[component]
pub fn Right(children: Children) -> impl IntoView {
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");
    let is_open = move || tabs_store.right_open.get();

    // Width state (px); start with the CSS default 260px
    let width = RwSignal::new(260.0f64);
    let is_resizing = RwSignal::new(false);
    let start_x = RwSignal::new(0.0f64);
    let start_width = RwSignal::new(260.0f64);

    // Handlers
    let on_resize_start = move |ev: leptos::ev::MouseEvent| {
        if !is_open() {
            return;
        }
        is_resizing.set(true);
        start_x.set(ev.client_x() as f64);
        start_width.set(width.get_untracked());
        ev.prevent_default();
    };

    let on_resize_move = move |ev: leptos::ev::MouseEvent| {
        if !is_resizing.get_untracked() {
            return;
        }
        let dx = start_x.get_untracked() - ev.client_x() as f64; // dragging left increases width
        let new_width = (start_width.get_untracked() + dx).max(30.0);
        width.set(new_width);
        ev.prevent_default();
    };

    let on_resize_end = move |ev: leptos::ev::MouseEvent| {
        if is_resizing.get_untracked() {
            is_resizing.set(false);
            ev.prevent_default();
        }
    };

    view! {
        <div
            data-zone="right"
            class="right"
            class:hidden=move || !is_open()
            class:resizing=move || is_resizing.get()
            // width reacts to state; also enforce min/max via CSS (30px..50vw)
            style:width=move || if is_open() { format!("{}px", width.get()) } else { "0px".to_string() }
            style:min-width=move || if is_open() { "30px".to_string() } else { "0".to_string() }
            //style:min-width="30px"
            style:max-width="50vw"
        >
            // Drag handle at the left edge of the right panel
            <div class="right-resizer" on:mousedown=on_resize_start></div>
            {children()}
            // Fullscreen overlay to capture mouse while resizing
            <Show when=move || is_resizing.get() fallback=|| ()>
                <div class="resize-overlay" on:mousemove=on_resize_move on:mouseup=on_resize_end></div>
            </Show>
        </div>
    }
}
