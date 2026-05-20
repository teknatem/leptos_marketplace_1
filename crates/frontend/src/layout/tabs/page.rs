//! TabPage component - wrapper для отображения контента таба
//!
//! Отвечает за:
//! - Показ/скрытие контента в зависимости от активности таба
//! - Ленивую инициализацию: контент создаётся только при первой активации таба
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
///
/// Контент создаётся **лениво** — только при первой активации таба. Это позволяет
/// открывать приложение с конкретным табом в URL без инициализации всех остальных
/// открытых табов (их VM, сигналов, Effects и сетевых запросов).
#[component]
pub fn TabPage(tab: TabData, tabs_store: AppGlobalContext) -> impl IntoView {
    let tab_key = tab.key.clone();

    // Memo<bool> является Copy — можно использовать в нескольких замыканиях.
    let tab_key_for_active = tab_key.clone();
    let is_active =
        Memo::new(move |_| tabs_store.active.get().as_deref() == Some(tab_key_for_active.as_str()));

    log!(
        "🔨 TabPage CREATED for: '{}' (this should happen once per open)",
        tab_key
    );

    let tab_key_for_cleanup = tab_key.clone();
    on_cleanup(move || {
        log!("💥 TabPage DESTROYED for: '{}'", tab_key_for_cleanup);
    });

    // Ленивая инициализация: `true` с самого начала если таб уже активен при открытии
    // (например, восстановлен из ?active= в URL), иначе — ждём первой активации.
    let already_active = tabs_store
        .active
        .with_untracked(|a| a.as_deref() == Some(tab_key.as_str()));
    let initialized = RwSignal::new(already_active);

    // Как только таб впервые становится активным — взводим флаг (необратимо).
    Effect::new(move |_| {
        if is_active.get() && !initialized.get_untracked() {
            initialized.set(true);
        }
    });

    // Контент создаётся ровно один раз: когда `initialized` переходит в `true`.
    // После этого `initialized` больше не меняется, поэтому closure не перезапускается
    // и компонент не пересоздаётся при переключении между табами.
    let tab_key_for_content = tab_key.clone();
    let content = move || {
        if initialized.get() {
            render_tab_content(&tab_key_for_content, tabs_store)
        } else {
            ().into_any()
        }
    };

    view! {
        <div
            class="app-tabs__item"
            class:app-tabs__item--hidden=move || !is_active.get()
            data-tab-key=tab_key
        >
            {content}
        </div>
    }
}
