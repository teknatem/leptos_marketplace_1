//! `MoreActionsMenu` — «Ещё» dropdown для заголовков страниц.
//!
//! Использует `position: fixed` + `NodeRef` для позиционирования относительно
//! viewport, что позволяет ему работать внутри `overflow: hidden` контейнеров
//! (например, `.page__header` с `overflow: hidden; position: sticky`).
//!
//! Тот же паттерн, что применяется в `CardAnimated` для nav-меню.
//!
//! # Использование
//!
//! ```rust
//! use crate::shared::components::more_actions_menu::{MoreActionsMenu, use_more_actions_close};
//!
//! <MoreActionsMenu>
//!     <button class="theme-dropdown__item" on:click=move |_| {
//!         use_more_actions_close();   // закрыть меню
//!         do_something();
//!     }>"Действие"</button>
//! </MoreActionsMenu>
//! ```
//!
//! Либо без `use_more_actions_close` — меню закроется по клику вне его (backdrop).

use crate::shared::icons::icon;
use leptos::context::provide_context;
use leptos::prelude::*;
use thaw::{Button, ButtonAppearance, ButtonSize};

/// Ключ контекста для сигнала открытия/закрытия меню «Ещё».
#[derive(Clone, Copy)]
pub struct MoreActionsClose(pub RwSignal<bool>);

/// Закрыть меню «Ещё» из дочернего элемента (если компонент найден в контексте).
pub fn use_more_actions_close() {
    if let Some(MoreActionsClose(open)) = use_context::<MoreActionsClose>() {
        open.set(false);
    }
}

/// Кнопка «Ещё ▾» с выпадающим меню, корректно отображаемым поверх
/// `overflow: hidden` родителей через `position: fixed`.
#[component]
pub fn MoreActionsMenu(children: Children) -> impl IntoView {
    let open = RwSignal::new(false);
    let trigger_ref = NodeRef::<leptos::html::Div>::new();
    let pos: RwSignal<(f64, f64)> = RwSignal::new((0.0, 0.0));

    provide_context(MoreActionsClose(open));

    // Render children once eagerly — the panel stays in the DOM and is shown/hidden via CSS.
    // This avoids the FnOnce vs Fn issue: Children is FnOnce and cannot be called from
    // a reactive Fn closure inside <Show>.
    let panel = children();

    view! {
        <div node_ref=trigger_ref style="position: relative;">
            <Button
                appearance=ButtonAppearance::Subtle
                size=ButtonSize::Medium
                on_click=move |_| {
                    if !open.get_untracked() {
                        if let Some(el) = trigger_ref.get() {
                            let rect = el.get_bounding_client_rect();
                            let vw = web_sys::window()
                                .and_then(|w| w.inner_width().ok())
                                .and_then(|v| v.as_f64())
                                .unwrap_or(1024.0);
                            pos.set((rect.bottom() + 4.0, vw - rect.right()));
                        }
                    }
                    open.update(|v| *v = !*v);
                }
            >
                <span class="page-action-button__content">
                    <span class="page-action-button__text">"Ещё"</span>
                    <span class="page-action-button__icon">{icon("chevron-down")}</span>
                </span>
            </Button>

            // Backdrop: только когда открыто
            <Show when=move || open.get()>
                <button
                    type="button"
                    style="position: fixed; inset: 0; z-index: 10000; background: transparent; border: none; cursor: default;"
                    aria-label="Закрыть меню"
                    on:click=move |_| open.set(false)
                />
            </Show>

            // Панель: всегда в DOM, скрыта через display:none когда закрыта.
            // position: fixed — не обрезается overflow: hidden родителя.
            <div
                style=move || {
                    if open.get() {
                        let (top, right) = pos.get();
                        format!(
                            "position: fixed; top: {top:.1}px; right: {right:.1}px; z-index: 10001; \
                             min-width: 210px; background: var(--color-surface); \
                             border: 1px solid var(--color-border); border-radius: var(--radius-md); \
                             box-shadow: 0 4px 24px rgba(0,0,0,.18); \
                             padding: var(--spacing-xs) 0; \
                             display: flex; flex-direction: column; gap: 2px; \
                             animation: fadeIn 0.15s ease;"
                        )
                    } else {
                        "display: none;".to_string()
                    }
                }
                on:click=move |ev| ev.stop_propagation()
            >
                {panel}
            </div>
        </div>
    }
}
