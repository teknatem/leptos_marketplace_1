//! Shared body-level popover helpers for inline help text.

use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;

static NEXT_INFO_POPOVER_ID: AtomicUsize = AtomicUsize::new(1);

thread_local! {
    static ACTIVE_INFO_POPOVER: RefCell<Option<InfoPopoverDom>> = const { RefCell::new(None) };
    /// Single <div> reused for nav-tooltip portal (created lazily, stays in DOM).
    static NAV_TOOLTIP_EL: RefCell<Option<web_sys::Element>> = const { RefCell::new(None) };
}

// ── Nav tooltip portal ────────────────────────────────────────────────────────

/// Ensure the tooltip `<div>` exists in `<body>` and return a reference to it.
fn ensure_nav_tooltip_el(document: &web_sys::Document) -> Option<web_sys::Element> {
    NAV_TOOLTIP_EL.with(|cell| {
        let mut borrow = cell.borrow_mut();
        if let Some(ref el) = *borrow {
            return Some(el.clone());
        }
        let Ok(el) = document.create_element("div") else {
            return None;
        };
        el.set_class_name("nav-tooltip-portal");
        let _ = el.set_attribute("role", "tooltip");
        let _ = el.set_attribute("aria-live", "off");
        if let Some(body) = document.body() {
            let _ = body.append_child(&el);
        }
        *borrow = Some(el.clone());
        Some(el)
    })
}

/// Show a lightweight body-level tooltip near the cursor (used for brief links).
pub fn show_nav_tooltip(text: &str, client_x: i32, client_y: i32) {
    let Some(window) = web_sys::window() else { return };
    let Some(document) = window.document() else { return };
    let Some(el) = ensure_nav_tooltip_el(&document) else { return };

    let vw = window.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(1024.0);
    let vh = window.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(768.0);
    let left = ((client_x as f64) + 12.0).min(vw - 320.0).max(8.0);
    let top  = ((client_y as f64) + 18.0).min(vh - 100.0).max(8.0);

    el.set_text_content(Some(text));
    let _ = el.set_attribute(
        "style",
        &format!("display:block; left:{left:.0}px; top:{top:.0}px;"),
    );
}

/// Hide the nav tooltip.
pub fn hide_nav_tooltip() {
    NAV_TOOLTIP_EL.with(|cell| {
        if let Some(ref el) = *cell.borrow() {
            let _ = el.set_attribute("style", "display:none;");
        }
    });
}

// ── Info popover (existing) ───────────────────────────────────────────────────


struct InfoPopoverDom {
    id: String,
    backdrop: web_sys::Element,
    panel: web_sys::Element,
    backdrop_listener: Closure<dyn FnMut(web_sys::MouseEvent)>,
    close_listener: Closure<dyn FnMut(web_sys::MouseEvent)>,
    keydown_listener: Closure<dyn FnMut(web_sys::KeyboardEvent)>,
    open_button: Option<web_sys::Element>,
    open_button_listener: Option<Closure<dyn FnMut(web_sys::MouseEvent)>>,
}

fn document() -> Option<web_sys::Document> {
    web_sys::window().and_then(|window| window.document())
}

fn remove_active_info_popover() {
    ACTIVE_INFO_POPOVER.with(|active| {
        let Some(popover) = active.borrow_mut().take() else {
            return;
        };

        let _ = popover.backdrop.remove_event_listener_with_callback(
            "click",
            popover.backdrop_listener.as_ref().unchecked_ref(),
        );
        let _ = popover.panel.remove_event_listener_with_callback(
            "click",
            popover.close_listener.as_ref().unchecked_ref(),
        );
        if let Some(document) = document() {
            let _ = document.remove_event_listener_with_callback(
                "keydown",
                popover.keydown_listener.as_ref().unchecked_ref(),
            );
        }
        if let (Some(open_btn), Some(listener)) = (
            popover.open_button.as_ref(),
            popover.open_button_listener.as_ref(),
        ) {
            let _ = open_btn
                .remove_event_listener_with_callback("click", listener.as_ref().unchecked_ref());
        }

        popover.panel.remove();
        popover.backdrop.remove();
    });
}

fn active_info_popover_id() -> Option<String> {
    ACTIVE_INFO_POPOVER.with(|active| active.borrow().as_ref().map(|popover| popover.id.clone()))
}

fn close_info_popover_if_id(id: &str) {
    let is_active = ACTIVE_INFO_POPOVER.with(|active| {
        active
            .borrow()
            .as_ref()
            .map(|popover| popover.id == id)
            .unwrap_or(false)
    });

    if is_active {
        remove_active_info_popover();
    }
}

fn append_text_div(
    document: &web_sys::Document,
    parent: &web_sys::Element,
    class_name: &str,
    text: &str,
) {
    if let Ok(element) = document.create_element("div") {
        element.set_class_name(class_name);
        element.set_text_content(Some(text));
        let _ = parent.append_child(&element);
    }
}

fn show_info_popover(
    id: String,
    title: &'static str,
    endpoint: &'static str,
    description: &'static str,
    client_x: i32,
    client_y: i32,
) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Some(body) = document.body() else {
        return;
    };

    remove_active_info_popover();

    let viewport_width = window
        .inner_width()
        .ok()
        .and_then(|value| value.as_f64())
        .unwrap_or(1024.0);
    let viewport_height = window
        .inner_height()
        .ok()
        .and_then(|value| value.as_f64())
        .unwrap_or(768.0);
    let left = (client_x as f64 + 8.0)
        .min((viewport_width - 380.0).max(8.0))
        .max(8.0);
    let top = (client_y as f64 + 8.0)
        .min((viewport_height - 190.0).max(8.0))
        .max(8.0);

    let Ok(backdrop) = document.create_element("button") else {
        return;
    };
    backdrop.set_class_name("info-popover-portal__backdrop");
    let _ = backdrop.set_attribute("type", "button");
    let _ = backdrop.set_attribute("aria-label", "Закрыть подсказку");

    let Ok(panel) = document.create_element("div") else {
        return;
    };
    panel.set_class_name("info-popover-portal__panel");
    let _ = panel.set_attribute("role", "tooltip");
    let _ = panel.set_attribute("data-popover-id", &id);
    let _ = panel.set_attribute("style", &format!("left: {left:.1}px; top: {top:.1}px;"));

    if let Ok(close_button) = document.create_element("button") {
        close_button.set_class_name("info-popover-portal__close");
        close_button.set_text_content(Some("x"));
        let _ = close_button.set_attribute("type", "button");
        let _ = close_button.set_attribute("aria-label", "Закрыть");
        let _ = panel.append_child(&close_button);
    }

    append_text_div(&document, &panel, "info-popover-portal__title", title);

    if let Ok(endpoint_row) = document.create_element("div") {
        endpoint_row.set_class_name("info-popover-portal__endpoint");
        let text = document.create_text_node("Endpoint: ");
        let _ = endpoint_row.append_child(&text);
        if let Ok(code) = document.create_element("code") {
            code.set_text_content(Some(endpoint));
            let _ = endpoint_row.append_child(&code);
        }
        let _ = panel.append_child(&endpoint_row);
    }

    append_text_div(
        &document,
        &panel,
        "info-popover-portal__description",
        description,
    );

    let backdrop_listener =
        Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(move |_event| {
            remove_active_info_popover();
        }));
    let _ = backdrop
        .add_event_listener_with_callback("click", backdrop_listener.as_ref().unchecked_ref());

    let close_listener = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(move |event| {
        let target = event
            .target()
            .and_then(|target| target.dyn_into::<web_sys::Element>().ok());
        if target
            .as_ref()
            .map(|target| target.class_list().contains("info-popover-portal__close"))
            .unwrap_or(false)
        {
            event.prevent_default();
            event.stop_propagation();
            remove_active_info_popover();
        }
    }));
    let _ =
        panel.add_event_listener_with_callback("click", close_listener.as_ref().unchecked_ref());

    let keydown_listener =
        Closure::<dyn FnMut(web_sys::KeyboardEvent)>::wrap(Box::new(move |event| {
            if event.key() == "Escape" {
                event.prevent_default();
                event.stop_propagation();
                remove_active_info_popover();
            }
        }));
    let _ = document
        .add_event_listener_with_callback("keydown", keydown_listener.as_ref().unchecked_ref());

    let _ = body.append_child(&backdrop);
    let _ = body.append_child(&panel);

    ACTIVE_INFO_POPOVER.with(|active| {
        *active.borrow_mut() = Some(InfoPopoverDom {
            id,
            backdrop,
            panel,
            backdrop_listener,
            close_listener,
            keydown_listener,
            open_button: None,
            open_button_listener: None,
        });
    });
}

fn show_indicator_popover(
    id: String,
    title: String,
    comment: Option<String>,
    on_open: impl Fn() + 'static,
    client_x: i32,
    client_y: i32,
) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Some(body) = document.body() else {
        return;
    };

    remove_active_info_popover();

    let viewport_width = window
        .inner_width()
        .ok()
        .and_then(|value| value.as_f64())
        .unwrap_or(1024.0);
    let viewport_height = window
        .inner_height()
        .ok()
        .and_then(|value| value.as_f64())
        .unwrap_or(768.0);
    let left = (client_x as f64 + 8.0)
        .min((viewport_width - 380.0).max(8.0))
        .max(8.0);
    let top = (client_y as f64 + 8.0)
        .min((viewport_height - 190.0).max(8.0))
        .max(8.0);

    let Ok(backdrop) = document.create_element("button") else {
        return;
    };
    backdrop.set_class_name("info-popover-portal__backdrop");
    let _ = backdrop.set_attribute("type", "button");
    let _ = backdrop.set_attribute("aria-label", "Закрыть подсказку");

    let Ok(panel) = document.create_element("div") else {
        return;
    };
    panel.set_class_name("info-popover-portal__panel");
    let _ = panel.set_attribute("role", "tooltip");
    let _ = panel.set_attribute("data-popover-id", &id);
    let _ = panel.set_attribute("style", &format!("left: {left:.1}px; top: {top:.1}px;"));

    if let Ok(close_button) = document.create_element("button") {
        close_button.set_class_name("info-popover-portal__close");
        close_button.set_text_content(Some("x"));
        let _ = close_button.set_attribute("type", "button");
        let _ = close_button.set_attribute("aria-label", "Закрыть");
        let _ = panel.append_child(&close_button);
    }

    append_text_div(&document, &panel, "info-popover-portal__title", &title);

    if let Some(ref comment_text) = comment {
        append_text_div(
            &document,
            &panel,
            "info-popover-portal__description",
            comment_text,
        );
    }

    let open_button = document.create_element("button").ok();
    if let Some(ref open_btn) = open_button {
        open_btn.set_class_name("info-popover-portal__open-link");
        open_btn.set_inner_html(concat!(
            "Открыть индикатор",
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" style="display:inline-block;vertical-align:-2px;margin-left:5px;flex-shrink:0"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>"#
        ));
        let _ = open_btn.set_attribute("type", "button");
        let _ = panel.append_child(open_btn);
    }

    let backdrop_listener =
        Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(move |_event| {
            remove_active_info_popover();
        }));
    let _ = backdrop
        .add_event_listener_with_callback("click", backdrop_listener.as_ref().unchecked_ref());

    let close_listener = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
        move |event: web_sys::MouseEvent| {
            let target = event
                .target()
                .and_then(|target| target.dyn_into::<web_sys::Element>().ok());
            if target
                .as_ref()
                .map(|target| target.class_list().contains("info-popover-portal__close"))
                .unwrap_or(false)
            {
                event.prevent_default();
                event.stop_propagation();
                remove_active_info_popover();
            }
        },
    ));
    let _ =
        panel.add_event_listener_with_callback("click", close_listener.as_ref().unchecked_ref());

    let keydown_listener =
        Closure::<dyn FnMut(web_sys::KeyboardEvent)>::wrap(Box::new(move |event| {
            if event.key() == "Escape" {
                event.prevent_default();
                event.stop_propagation();
                remove_active_info_popover();
            }
        }));
    let _ = document
        .add_event_listener_with_callback("keydown", keydown_listener.as_ref().unchecked_ref());

    let open_button_listener = open_button.as_ref().map(|open_btn| {
        let on_open_rc = Rc::new(on_open);
        let listener = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                event.stop_propagation();
                remove_active_info_popover();
                on_open_rc();
            },
        ));
        let _ =
            open_btn.add_event_listener_with_callback("click", listener.as_ref().unchecked_ref());
        listener
    });

    let _ = body.append_child(&backdrop);
    let _ = body.append_child(&panel);

    ACTIVE_INFO_POPOVER.with(|active| {
        *active.borrow_mut() = Some(InfoPopoverDom {
            id,
            backdrop,
            panel,
            backdrop_listener,
            close_listener,
            keydown_listener,
            open_button,
            open_button_listener,
        });
    });
}

#[component]
pub fn IndicatorInfoButton(
    title: String,
    comment: Option<String>,
    on_open: Callback<()>,
) -> impl IntoView {
    let popover_id = StoredValue::new(format!(
        "info-popover-{}",
        NEXT_INFO_POPOVER_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let title = StoredValue::new(title);
    let comment = StoredValue::new(comment);

    on_cleanup(move || {
        let id = popover_id.get_value();
        close_info_popover_if_id(&id);
    });

    view! {
        <button
            type="button"
            class="info-popover-label__trigger"
            title="Описание индикатора"
            aria-label="Описание индикатора"
            on:click=move |event: leptos::ev::MouseEvent| {
                event.prevent_default();
                event.stop_propagation();

                let id = popover_id.get_value();
                if active_info_popover_id().as_deref() == Some(id.as_str()) {
                    remove_active_info_popover();
                } else {
                    show_indicator_popover(
                        id,
                        title.get_value(),
                        comment.get_value(),
                        move || on_open.run(()),
                        event.client_x(),
                        event.client_y(),
                    );
                }
            }
        >
            "?"
        </button>
    }
}

#[component]
pub fn HelpPopoverLabel(
    label: &'static str,
    endpoint: &'static str,
    description: &'static str,
) -> impl IntoView {
    let popover_id = StoredValue::new(format!(
        "info-popover-{}",
        NEXT_INFO_POPOVER_ID.fetch_add(1, Ordering::Relaxed)
    ));

    on_cleanup(move || {
        let id = popover_id.get_value();
        close_info_popover_if_id(&id);
    });

    view! {
        <span class="info-popover-label">
            <span>{label}</span>
            <button
                type="button"
                class="info-popover-label__trigger"
                title="Описание показателя"
                aria-label="Описание показателя"
                on:click=move |event: leptos::ev::MouseEvent| {
                    event.prevent_default();
                    event.stop_propagation();

                    let id = popover_id.get_value();
                    if active_info_popover_id().as_deref() == Some(id.as_str()) {
                        remove_active_info_popover();
                    } else {
                        show_info_popover(
                            id,
                            label,
                            endpoint,
                            description,
                            event.client_x(),
                            event.client_y(),
                        );
                    }
                }
            >
                <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                    <circle cx="12" cy="12" r="10"/>
                    <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/>
                    <path d="M12 17h.01"/>
                </svg>
            </button>
        </span>
    }
}
