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
//! ```

use leptos::prelude::*;
use thaw::Card;

/// Обёртка над Thaw [`Card`] с анимацией `card-appear` из `layout.css`.
///
/// # Props
/// - `delay_ms` — задержка анимации в мс (по умолчанию `0`). Используй для stagger-эффекта.
/// - `style`    — дополнительные inline-стили, которые добавляются к анимации.
/// - `children` — содержимое карточки (аналогично обычному `Card`).
#[component]
pub fn CardAnimated(
    /// Задержка анимации в миллисекундах (для stagger-эффекта).
    #[prop(optional)]
    delay_ms: u32,
    /// Дополнительные inline-стили (добавляются после стилей анимации).
    #[prop(optional, into)]
    style: String,
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

    view! {
        <Card attr:style=full_style>
            {children()}
        </Card>
    }
}
