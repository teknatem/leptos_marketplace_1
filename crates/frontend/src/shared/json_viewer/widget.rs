use crate::shared::icons::icon;
use leptos::prelude::*;

#[component]
pub fn JsonViewer(
    /// JSON строка для отображения
    json_content: String,
    /// Заголовок
    #[prop(optional)]
    title: Option<String>,
) -> impl IntoView {
    let (copied, set_copied) = signal(false);

    let json_content_for_copy = json_content.clone();
    let json_content_for_download = json_content.clone();
    let json_content_for_display = json_content.clone();
    let json_content_for_stats = json_content.clone();

    // Копирование в буфер обмена
    let handle_copy = move |_| {
        let window = web_sys::window().expect("no window");
        let clipboard = window.navigator().clipboard();
        let content = json_content_for_copy.clone();
        let _ = wasm_bindgen_futures::spawn_local(async move {
            let promise = clipboard.write_text(&content);
            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
        });
        set_copied.set(true);

        // Сбросить через 2 секунды
        leptos::task::spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(2000).await;
            set_copied.set(false);
        });
    };

    // Скачать JSON файл
    let handle_download = move |_| {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                // Создаем Blob
                let blob_parts = js_sys::Array::new();
                blob_parts.push(&wasm_bindgen::JsValue::from_str(&json_content_for_download));

                let blob_property_bag = web_sys::BlobPropertyBag::new();
                blob_property_bag.set_type("application/json");

                if let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(
                    &blob_parts,
                    &blob_property_bag,
                ) {
                    // Создаем URL для blob
                    if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                        // Создаем ссылку для скачивания
                        if let Ok(a) = document.create_element("a") {
                            use wasm_bindgen::JsCast;
                            if let Ok(link) = a.dyn_into::<web_sys::HtmlAnchorElement>() {
                                link.set_href(&url);
                                link.set_download("import_data.json");
                                let _ = link.click();

                                // Освобождаем URL
                                web_sys::Url::revoke_object_url(&url).ok();
                            }
                        }
                    }
                }
            }
        }
    };

    view! {
        <div style="display: flex; flex-direction: column; height: 100%; overflow: hidden;">
            // Заголовок и кнопки
            <div style="padding: 15px; border-bottom: 1px solid #ddd; background: #f9f9f9; display: flex; justify-content: space-between; align-items: center; flex-shrink: 0;">
                <h3 style="margin: 0; font-size: 18px; color: #333;">
                    {title.unwrap_or_else(|| "JSON Данные".to_string())}
                </h3>
                <div style="display: flex; gap: 10px;">
                    <button
                        class="btn btn-secondary"
                        on:click=handle_copy
                        style="padding: 8px 16px; font-size: 14px;"
                        title="Копировать в буфер обмена"
                    >
                        {move || if copied.get() {
                            view! {
                                <>
                                    {icon("check")}
                                    {"Скопировано!"}
                                </>
                            }.into_any()
                        } else {
                            view! {
                                <>
                                    {icon("copy")}
                                    {"Копировать"}
                                </>
                            }.into_any()
                        }}
                    </button>
                    <button
                        class="btn"
                        style="background: #4CAF50; color: white; padding: 8px 16px; font-size: 14px;"
                        on:click=handle_download
                        title="Скачать как файл"
                    >
                        {icon("download")}
                        {"Скачать"}
                    </button>
                </div>
            </div>

            // Область просмотра JSON
            <div style="flex: 1; overflow: auto; padding: 20px; background: #f5f5f5;">
                <pre style="margin: 0; padding: 20px; background: white; border: 1px solid #ddd; border-radius: 4px; font-family: 'Courier New', monospace; font-size: 13px; line-height: 1.6; color: #333; overflow-x: auto;">
                    {json_content_for_display}
                </pre>
            </div>

            // Статистика
            <div style="padding: 15px; border-top: 1px solid #ddd; background: #f9f9f9; font-size: 14px; color: #666; flex-shrink: 0;">
                {"Размер: "}
                <strong>{format!("{} символов", json_content_for_stats.len())}</strong>
                {" | "}
                {"Строк: "}
                <strong>{json_content_for_stats.lines().count()}</strong>
            </div>
        </div>
    }
}
