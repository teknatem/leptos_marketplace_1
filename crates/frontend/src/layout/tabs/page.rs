//! TabPage component - wrapper для отображения контента таба
//!
//! Отвечает за:
//! - Показ/скрытие контента в зависимости от активности таба
//! - Логирование создания/уничтожения для отладки
//! - Вызов registry для получения контента по ключу

use super::registry::render_tab_content;
use crate::layout::global_context::{AppGlobalContext, Tab as TabData};
use leptos::logging::log;
use leptos::prelude::*;

/// Компонент-обёртка для отдельного таба.
///
/// Рендерит контент таба через `registry::render_tab_content` и управляет
/// видимостью через CSS class `hidden` в зависимости от того, активен ли таб.
#[component]
pub fn TabPage(tab: TabData, tabs_store: AppGlobalContext) -> impl IntoView {
    let tab_key = tab.key.clone();
    let tab_key_for_active_check = tab_key.clone();

    // Check if this tab is active - this closure will be reactive
    let is_active = move || {
        let current_active = tabs_store.active.get();
        current_active.as_ref() == Some(&tab_key_for_active_check)
    };

    log!(
        "🔨 TabPage CREATED for: '{}' (this should happen once per open)",
        tab_key
    );

    // Log when component is destroyed
    let tab_key_for_cleanup = tab_key.clone();
    on_cleanup(move || {
        log!("💥 TabPage DESTROYED for: '{}'", tab_key_for_cleanup);
    });

    // Render content using the registry
    let tab_key_for_content = tab_key.clone();
    let content = render_tab_content(&tab_key_for_content, tabs_store);

    view! {
        <div
            class="tabs__item"
            class:tabs__item--hidden=move || !is_active()
            data-tab-key=tab_key
        >
            {content}
        </div>
    }
}
