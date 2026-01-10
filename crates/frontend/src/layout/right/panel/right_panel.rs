//! Right panel component - правая боковая панель
//!
//! Пока пустая заглушка. В будущем может использоваться для:
//! - Контекстной информации
//! - Быстрых действий
//! - Уведомлений

use leptos::prelude::*;

#[component]
pub fn RightPanel() -> impl IntoView {
    view! {
        <div class="right-panel">
            // Placeholder - панель скрыта по умолчанию через AppGlobalContext.right_open
        </div>
    }
}
