//! CardAnimated — обёртка над Thaw Card с анимацией появления.
//!
//! Полная замена `<Card attr:style="...">` → `<CardAnimated style="..." delay_ms=N>`.
//! Анимация определена в `layout.css` (`@keyframes card-appear`).
//!
//! # Пример
//! ```rust
//! // Без задержки
//! <CardAnimated>
//!     <p>"Контент"</p>
//! </CardAnimated>
//!
//! // С каскадной задержкой для stagger-эффекта
//! <CardAnimated delay_ms=0>   // карточка 1
//! <CardAnimated delay_ms=80>  // карточка 2
//! <CardAnimated delay_ms=160> // карточка 3
//!
//! // С дополнительными inline-стилями (нестандартная ширина и т.д.)
//! <CardAnimated style="max-width: 400px;" delay_ms=0>
//!
//! // С nav id для поиска в IDE и DOM inspector
//! <CardAnimated nav_id="a024_bi_indicator_details_preview_fields">
//!     <p>"Контент"</p>
//! </CardAnimated>
//! ```

use crate::shared::clipboard::copy_to_clipboard_with_callback;
use crate::shared::icons::icon;
use leptos::prelude::*;
use thaw::Card;

/// Обёртка над Thaw [`Card`] с анимацией `card-appear` из `layout.css`.
///
/// # Props
/// - `delay_ms` — задержка анимации в мс (по умолчанию `0`). Используй для stagger-эффекта.
/// - `style`    — дополнительные inline-стили, которые добавляются к анимации.
/// - `nav_id`   — стабильный id карточки для поиска в IDE и DOM inspector.
/// - `children` — содержимое карточки (аналогично обычному `Card`).
#[component]
pub fn CardAnimated(
    /// Задержка анимации в миллисекундах (для stagger-эффекта).
    #[prop(optional)]
    delay_ms: u32,
    /// Дополнительные inline-стили (добавляются после стилей анимации).
    #[prop(optional, into)]
    style: String,
    /// Стабильный id карточки для поиска в IDE и DOM inspector.
    #[prop(optional, into)]
    nav_id: String,
    children: Children,
) -> impl IntoView {
    let full_style = if style.is_empty() {
        format!("animation: card-appear 0.28s ease-out {}ms both;", delay_ms)
    } else {
        format!(
            "animation: card-appear 0.28s ease-out {}ms both; {}",
            delay_ms, style
        )
    };
    let menu_open = RwSignal::new(false);
    let copied = RwSignal::new(false);
    let nav_id = nav_id.trim().to_string();
    let has_nav_id = !nav_id.is_empty();
    let nav_id_attr = nav_id.clone();
    let nav_id_dom = nav_id.clone();

    let content_class = if has_nav_id {
        "card-animated__content card-animated__content--with-nav"
    } else {
        "card-animated__content"
    };

    view! {
        <div
            class="card-animated"
            attr:data-nav-id=nav_id_attr
            id=nav_id_dom
        >
            <Card attr:style=full_style>
                {if has_nav_id {
                    let nav_id_value = StoredValue::new(nav_id.clone());
                    view! {
                        <div class="card-animated__nav">
                            <button
                                type="button"
                                class="card-animated__nav-trigger"
                                title="Info"
                                aria-label="Info"
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    copied.set(false);
                                    menu_open.update(|open| *open = !*open);
                                }
                            >
                                <span class="card-animated__nav-icon">
                                    {icon("settings")}
                                </span>
                            </button>

                            <Show when=move || menu_open.get()>
                                <>
                                    <button
                                        type="button"
                                        class="card-animated__nav-backdrop"
                                        aria-label="Закрыть Info"
                                        on:click=move |_| {
                                            menu_open.set(false);
                                            copied.set(false);
                                        }
                                    ></button>
                                    <div
                                        class="card-animated__nav-menu"
                                        on:click=move |ev| ev.stop_propagation()
                                    >
                                        <div class="card-animated__nav-header">
                                            <div class="card-animated__nav-title">"Info"</div>
                                            <button
                                                type="button"
                                                class="card-animated__nav-close"
                                                title="Закрыть"
                                                aria-label="Закрыть"
                                                on:click=move |_| {
                                                    menu_open.set(false);
                                                    copied.set(false);
                                                }
                                            >
                                                {icon("x")}
                                            </button>
                                        </div>
                                        <div class="card-animated__nav-row">
                                            <span class="card-animated__nav-key">"id"</span>
                                            <code class="card-animated__nav-code">{move || nav_id_value.get_value()}</code>
                                            <button
                                                type="button"
                                                class="card-animated__nav-copy"
                                                title="Копировать id"
                                                aria-label="Копировать id"
                                                on:click=move |_| {
                                                    let on_success = move || copied.set(true);
                                                    let nav_id_copy = nav_id_value.get_value();
                                                    copy_to_clipboard_with_callback(&nav_id_copy, on_success);
                                                }
                                            >
                                                {move || {
                                                    if copied.get() {
                                                        icon("check")
                                                    } else {
                                                        icon("copy")
                                                    }
                                                }}
                                            </button>
                                        </div>
                                    </div>
                                </>
                            </Show>
                        </div>
                    }.into_any()
                } else {
                    ().into_any()
                }}
                <div class=content_class>
                    {children()}
                </div>
            </Card>
        </div>
    }
}
