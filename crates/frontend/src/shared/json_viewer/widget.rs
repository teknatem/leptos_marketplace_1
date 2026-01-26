use crate::shared::icons::icon;
use leptos::prelude::*;

/// Подсветка JSON синтаксиса с цветами для разных типов данных
fn highlight_json_html(json: &str) -> String {
    let mut result = String::with_capacity(json.len() * 2);
    let mut chars = json.chars().peekable();
    let mut in_string = false;
    let mut in_key = false;
    let mut escape_next = false;
    let mut current_token = String::new();
    
    while let Some(ch) = chars.next() {
        if escape_next {
            current_token.push(ch);
            escape_next = false;
            continue;
        }
        
        match ch {
            '\\' if in_string => {
                escape_next = true;
                current_token.push(ch);
            }
            '"' => {
                if in_string {
                    // Закрываем строку
                    current_token.push('"');
                    let class = if in_key { "json-key" } else { "json-string" };
                    result.push_str(&format!("<span class=\"{}\">{}</span>", class, html_escape(&current_token)));
                    current_token.clear();
                    in_string = false;
                    in_key = false;
                } else {
                    // Открываем строку
                    in_string = true;
                    current_token.push('"');
                    // Проверяем, это ключ или значение
                    let mut temp_chars = chars.clone();
                    while let Some(ch) = temp_chars.next() {
                        if ch == '"' {
                            // Пропускаем содержимое строки до закрывающей кавычки
                            break;
                        }
                    }
                    // Смотрим что после закрывающей кавычки (пропуская пробелы)
                    while let Some(ch) = temp_chars.next() {
                        if !ch.is_whitespace() {
                            if ch == ':' {
                                in_key = true;
                            }
                            break;
                        }
                    }
                }
            }
            ':' | ',' | '[' | ']' | '{' | '}' if !in_string => {
                result.push_str(&format!("<span class=\"json-punctuation\">{}</span>", ch));
            }
            c if !in_string && !c.is_whitespace() => {
                // Собираем числа, true, false, null
                current_token.push(c);
                if chars.peek().map(|&next| next.is_whitespace() || ",:]}".contains(next)).unwrap_or(true) {
                    let class = if current_token == "true" || current_token == "false" {
                        "json-boolean"
                    } else if current_token == "null" {
                        "json-null"
                    } else if current_token.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-' || c == 'e' || c == 'E' || c == '+') {
                        "json-number"
                    } else {
                        ""
                    };
                    
                    if !class.is_empty() {
                        result.push_str(&format!("<span class=\"{}\">{}</span>", class, html_escape(&current_token)));
                    } else {
                        result.push_str(&html_escape(&current_token));
                    }
                    current_token.clear();
                }
            }
            c if in_string => {
                current_token.push(c);
            }
            c => {
                result.push(c);
            }
        }
    }
    
    result
}

/// Экранирование HTML символов для безопасного отображения
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

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
    let json_content_for_stats = json_content.clone();
    
    // Применяем подсветку синтаксиса
    let highlighted_html = highlight_json_html(&json_content);

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
        <div class="json-viewer">
            // Заголовок и кнопки
            <div class="modal-header modal-header--compact">
                <h3 class="modal-title">
                    {title.unwrap_or_else(|| "JSON Данные".to_string())}
                </h3>
                <div class="modal-header-actions">
                    <button
                        class="button button--secondary"
                        on:click=handle_copy
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
                        class="button button--success"
                        on:click=handle_download
                        title="Скачать как файл"
                    >
                        {icon("download")}
                        {"Скачать"}
                    </button>
                </div>
            </div>

            // Область просмотра JSON
            <div class="json-viewer__body">
                <pre class="json-viewer__content" inner_html=highlighted_html>
                </pre>
            </div>

            // Статистика
            <div class="json-viewer__footer">
                {"Размер: "}
                <strong>{format!("{} символов", json_content_for_stats.len())}</strong>
                {" | "}
                {"Строк: "}
                <strong>{json_content_for_stats.lines().count()}</strong>
            </div>
        </div>
    }
}
