use super::{get_dom_snapshot, tree_view::TreeView, DomNode};
use crate::shared::icons::icon;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[component]
pub fn DomValidatorPage() -> impl IntoView {
    // Создаем реактивный сигнал, который будет обновляться
    let (tree, set_tree) = signal::<Option<DomNode>>(get_dom_snapshot());

    // Обновляем данные при монтировании компонента
    Effect::new(move |_| {
        set_tree.set(get_dom_snapshot());
    });

    let export_to_json = move |_| {
        if let Some(tree_data) = tree.get() {
            if let Ok(json) = serde_json::to_string_pretty(&tree_data) {
                // Создаем blob и скачиваем файл
                if let Some(window) = web_sys::window() {
                    if let Some(document) = window.document() {
                        // Создаем элемент ссылки для скачивания
                        if let Ok(elem) = document.create_element("a") {
                            if let Ok(element) = elem.dyn_into::<web_sys::HtmlAnchorElement>() {
                                let blob_parts = js_sys::Array::new();
                                blob_parts.push(&wasm_bindgen::JsValue::from_str(&json));

                                if let Ok(blob) = web_sys::Blob::new_with_str_sequence(&blob_parts)
                                {
                                    if let Ok(url) =
                                        web_sys::Url::create_object_url_with_blob(&blob)
                                    {
                                        element.set_href(&url);
                                        element.set_download("dom_snapshot.json");
                                        let _ = element.click();
                                        let _ = web_sys::Url::revoke_object_url(&url);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    view! {
        <div class="page">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"DOM Inspector"</h1>
                </div>
                <div class="page__header-right">
                    <button
                        class="button button--secondary"
                        on:click=export_to_json
                        disabled=move || tree.get().is_none()
                    >
                        {icon("download")}
                        "Экспорт в JSON"
                    </button>
                </div>
            </div>

            <div class="dom-validator-content">
                {move || match tree.get() {
                    Some(node) => view! { <TreeView node=node /> }.into_any(),
                    None => view! {
                        <div class="dom-validator-placeholder">
                            <p>"Снимок DOM не найден"</p>
                        </div>
                    }.into_any()
                }}
            </div>
        </div>
    }
}
