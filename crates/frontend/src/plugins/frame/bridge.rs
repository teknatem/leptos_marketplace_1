use js_sys::{Function, Object, Reflect};
use serde::Serialize;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::HtmlIFrameElement;

pub(super) struct MessageListenerGuard {
    window: web_sys::Window,
    js_fn: Function,
    _handler: Closure<dyn FnMut(web_sys::MessageEvent)>,
}

impl MessageListenerGuard {
    pub(super) fn new(
        window: web_sys::Window,
        handler: Closure<dyn FnMut(web_sys::MessageEvent)>,
    ) -> Self {
        let js_fn = handler.as_ref().unchecked_ref::<Function>().clone();
        let _ = window.add_event_listener_with_callback("message", &js_fn);
        Self {
            window,
            js_fn,
            _handler: handler,
        }
    }
}

impl Drop for MessageListenerGuard {
    fn drop(&mut self) {
        let _ = self
            .window
            .remove_event_listener_with_callback("message", &self.js_fn);
    }
}

pub(super) fn post_json(iframe: &HtmlIFrameElement, value: serde_json::Value) {
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    let Ok(js_value) = value.serialize(&serializer) else {
        return;
    };
    if let Some(window) = iframe.content_window() {
        let _ = window.post_message(&js_value, "*");
    }
}

pub(super) fn string_property(data: &JsValue, name: &str) -> Option<String> {
    Reflect::get(data, &JsValue::from_str(name))
        .ok()
        .and_then(|value| value.as_string())
}

pub(super) fn event_source_matches_iframe(
    event: &web_sys::MessageEvent,
    iframe: &HtmlIFrameElement,
) -> bool {
    let Some(source) = event.source() else {
        return false;
    };
    let Some(expected) = iframe.content_window() else {
        return false;
    };
    Object::is(source.as_ref(), expected.as_ref())
}
