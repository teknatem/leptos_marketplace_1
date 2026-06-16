use js_sys::Function;
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = loadPluginCodeEditor)]
    fn load_plugin_code_editor() -> js_sys::Promise;

    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = PluginCodeEditor, js_name = create)]
    fn create_editor(
        parent: &web_sys::Element,
        language: &str,
        value: &str,
        on_change: &Function,
    ) -> JsValue;

    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = PluginCodeEditor, js_name = setValue)]
    fn set_editor_value(editor: &JsValue, value: &str);

    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = PluginCodeEditor, js_name = destroy)]
    fn destroy_editor(editor: &JsValue);
}

struct EditorHandle {
    editor: JsValue,
    _on_change: Closure<dyn FnMut(String)>,
}

#[component]
pub fn CodeEditor(
    language: &'static str,
    value: RwSignal<String>,
    #[prop(optional, into)] class: String,
) -> impl IntoView {
    let node_ref = NodeRef::<html::Div>::new();
    let handle = StoredValue::new_local(None::<EditorHandle>);

    {
        node_ref.on_load(move |element| {
            spawn_local(async move {
                if let Err(error) = JsFuture::from(load_plugin_code_editor()).await {
                    web_sys::console::error_2(
                        &JsValue::from_str("Failed to load CodeMirror"),
                        &error,
                    );
                    return;
                }
                if handle.is_disposed() {
                    return;
                }

                let callback = Closure::wrap(Box::new(move |next: String| {
                    value.set(next);
                }) as Box<dyn FnMut(String)>);
                let editor = create_editor(
                    element.unchecked_ref::<web_sys::Element>(),
                    language,
                    &value.get_untracked(),
                    callback.as_ref().unchecked_ref(),
                );
                handle.set_value(Some(EditorHandle {
                    editor,
                    _on_change: callback,
                }));
            });
        });
    }

    {
        Effect::new(move |_| {
            let next = value.get();
            handle.with_value(|handle| {
                if let Some(handle) = handle {
                    set_editor_value(&handle.editor, &next);
                }
            });
        });
    }

    on_cleanup(move || {
        let mut removed = None;
        handle.update_value(|current| {
            removed = current.take();
        });
        if let Some(handle) = removed {
            destroy_editor(&handle.editor);
        }
    });

    view! {
        <div node_ref=node_ref class=format!("plugin-code-editor {class}")></div>
    }
}
