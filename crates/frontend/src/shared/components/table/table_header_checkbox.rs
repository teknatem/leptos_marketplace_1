//! Компонент чекбокса в заголовке таблицы для выбора всех строк
//!
//! # Примеры
//!
//! ```rust
//! <TableHeaderCheckbox
//!     items=items
//!     selected=selected
//!     get_id=Callback::new(|row: MyRow| row.id.clone())
//!     on_change=Callback::new(move |check_all: bool| {
//!         if check_all {
//!             // Выбрать все
//!         } else {
//!             // Снять все
//!         }
//!     })
//! />
//! ```

use leptos::prelude::*;
use leptos::prelude::event_target_checked;
use std::collections::HashSet;
use thaw::*;
use wasm_bindgen::JsCast;

/// Компонент чекбокса в заголовке таблицы
///
/// Автоматически:
/// - Показывает три состояния: unchecked, checked, indeterminate
/// - При клике переключает между "выбрать все" и "снять все"
/// - Вычисляет состояние на основе items и selected
#[component]
pub fn TableHeaderCheckbox<T>(
    /// Все items в таблице
    #[prop(into)]
    items: Signal<Vec<T>>,
    
    /// Выбранные ID
    #[prop(into)]
    selected: Signal<HashSet<String>>,
    
    /// Функция для получения ID из item
    get_id: Callback<T, String>,
    
    /// Callback при изменении (true = выбрать все, false = снять все)
    on_change: Callback<bool>,
) -> impl IntoView
where
    T: Clone + Send + Sync + 'static,
{
    // Вычисление состояния чекбокса
    let checkbox_state = Signal::derive(move || {
        let current_items = items.get();
        let sel = selected.get();
        
        if current_items.is_empty() {
            return CheckboxState::Unchecked;
        }
        
        let selected_count = current_items
            .iter()
            .filter(|&item| {
                let id = get_id.run(item.clone());
                sel.contains(&id)
            })
            .count();
        
        if selected_count == 0 {
            CheckboxState::Unchecked
        } else if selected_count == current_items.len() {
            CheckboxState::Checked
        } else {
            CheckboxState::Indeterminate
        }
    });
    
    // Создаем NodeRef для доступа к DOM элементу
    let checkbox_ref = NodeRef::<leptos::html::Input>::new();
    
    // Effect для установки indeterminate состояния
    Effect::new(move |_| {
        if let Some(input) = checkbox_ref.get() {
            let state = checkbox_state.get();
            
            // Устанавливаем indeterminate через web_sys
            if let Some(input_el) = input.dyn_ref::<web_sys::HtmlInputElement>() {
                let is_indeterminate = matches!(state, CheckboxState::Indeterminate);
                input_el.set_indeterminate(is_indeterminate);
            }
        }
    });
    
    view! {
        <TableHeaderCell resizable=false class="fixed-checkbox-column">
            <input
                node_ref=checkbox_ref
                type="checkbox"
                class="table__checkbox"
                prop:checked=move || matches!(checkbox_state.get(), CheckboxState::Checked)
                on:change=move |ev| {
                    // Получаем checked из DOM элемента, а не из checkbox_state
                    // чтобы избежать конфликта заимствований
                    let checked = event_target_checked(&ev);
                    on_change.run(checked);
                }
            />
        </TableHeaderCell>
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CheckboxState {
    Unchecked,
    Checked,
    Indeterminate,
}
