//! Компонент чекбокса в ячейке таблицы для выбора отдельной строки
//!
//! # Примеры
//!
//! ```rust
//! <TableCellCheckbox
//!     item_id=row.id.clone()
//!     selected=selected
//!     on_change=Callback::new(move |(id, checked)| {
//!         toggle_select(id, checked);
//!     })
//! />
//! ```

use leptos::prelude::*;
use std::collections::HashSet;
use thaw::*;

/// Компонент чекбокса в ячейке таблицы
///
/// Автоматически:
/// - Отображает состояние выбора на основе selected set
/// - Останавливает propagation клика (чтобы не вызывать клик на строке)
/// - Вызывает callback при изменении состояния
#[component]
pub fn TableCellCheckbox(
    /// ID текущего элемента
    #[prop(into)]
    item_id: String,
    
    /// Выбранные ID
    #[prop(into)]
    selected: Signal<HashSet<String>>,
    
    /// Callback при изменении (item_id, checked)
    on_change: Callback<(String, bool)>,
) -> impl IntoView {
    let item_id_for_checked = item_id.clone();
    let item_id_for_change = item_id.clone();
    
    view! {
        <TableCell class="fixed-checkbox-column" on:click=|e| e.stop_propagation()>
            <input
                type="checkbox"
                class="table__checkbox"
                prop:checked=move || selected.get().contains(&item_id_for_checked)
                on:change=move |ev| {
                    let checked = event_target_checked(&ev);
                    on_change.run((item_id_for_change.clone(), checked));
                }
            />
        </TableCell>
    }
}
