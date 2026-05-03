//! ClosePageButton — кнопка «Закрыть» для правого верхнего угла страницы.
//!
//! Закрывает текущий активный таб.  Нажатие Esc делает то же самое —
//! оно обрабатывается глобально в `app::App` и не требует доп. кода здесь.
//!
//! Пример использования в `page__header-right`:
//!
//! ```rust
//! use crate::shared::components::close_page_button::ClosePageButton;
//!
//! view! {
//!     <div class="page__header">
//!         <div class="page__header-left">
//!             <h1 class="page__title">"Заголовок"</h1>
//!         </div>
//!         <div class="page__header-right">
//!             <ClosePageButton />
//!         </div>
//!     </div>
//! }
//! ```

use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use leptos::prelude::*;
use thaw::{Button, ButtonAppearance};

/// Кнопка «Закрыть», которая закрывает текущий активный таб.
///
/// Esc-клавиша обрабатывается глобально в `app::App`, поэтому
/// компонент намеренно не регистрирует собственный обработчик клавиш.
#[component]
pub fn ClosePageButton() -> impl IntoView {
    let tabs_store = use_context::<AppGlobalContext>().expect("AppGlobalContext missing");

    let on_close = move |_| {
        if let Some(key) = tabs_store.active.get_untracked() {
            tabs_store.close_tab(&key);
        }
    };

    view! {
        <Button appearance=ButtonAppearance::Secondary on_click=on_close>
            {icon("x")}
            " Закрыть"
        </Button>
    }
}
