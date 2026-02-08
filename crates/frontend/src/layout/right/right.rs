use crate::layout::global_context::AppGlobalContext;
use leptos::prelude::*;
use leptos::prelude::window_event_listener;

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

    // Обработчик начала resize
    let on_resize_start = move |ev: leptos::ev::MouseEvent| {
        if !is_open() {
            return;
        }
        is_resizing.set(true);
        start_x.set(ev.client_x() as f64);
        start_width.set(width.get_untracked());
        ev.prevent_default();
    };

    // Глобальный обработчик mousemove на window
    let _ = window_event_listener(leptos::ev::mousemove, move |ev: leptos::ev::MouseEvent| {
        if !is_resizing.get_untracked() {
            return;
        }
        
        // Получаем размер окна
        let window = web_sys::window().expect("window");
        let window_width = window.inner_width().unwrap().as_f64().unwrap();
        
        // Расчет доступной ширины
        let max_available = window_width - 400.0 - 260.0;
        let max_width = max_available.min(window_width * 0.5);
        
        let dx = start_x.get_untracked() - ev.client_x() as f64;
        let new_width = (start_width.get_untracked() + dx)
            .max(30.0)
            .min(max_width);
        
        width.set(new_width);
    });

    // Глобальный обработчик mouseup на window
    let _ = window_event_listener(leptos::ev::mouseup, move |_ev: leptos::ev::MouseEvent| {
        if is_resizing.get_untracked() {
            is_resizing.set(false);
        }
    });

    // Effect для управления cursor и user-select
    Effect::new(move |_| {
        let is_resizing_value = is_resizing.get();
        
        if let Some(body) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.body())
        {
            if is_resizing_value {
                let _ = body.style().set_property("cursor", "col-resize");
                let _ = body.style().set_property("user-select", "none");
            } else {
                let _ = body.style().set_property("cursor", "");
                let _ = body.style().set_property("user-select", "");
            }
        }
    });

    view! {
        <div
            data-zone="right"
            class="right-panel"
            class:right-panel--hidden=move || !is_open()
            class:right-panel--resizing=move || is_resizing.get()
            style:width=move || if is_open() { format!("{}px", width.get()) } else { "0px".to_string() }
        >
            <div class="right-panel__resizer" on:mousedown=on_resize_start></div>
            {children()}
        </div>
    }
}
