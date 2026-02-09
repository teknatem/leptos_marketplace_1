//! Компонент сортируемой ячейки заголовка таблицы
//!
//! # Примеры
//!
//! ```rust
//! // Базовое использование
//! <SortableHeaderCell
//!     label="Сумма"
//!     sort_field="amount"
//!     current_sort_field=Signal::derive(move || state.with(|s| s.sort_field.clone()))
//!     sort_ascending=Signal::derive(move || state.with(|s| s.sort_ascending))
//!     on_sort=Callback::new(move |field| toggle_sort(field))
//! />
//!
//! // С правым выравниванием (для числовых колонок)
//! <SortableHeaderCell
//!     label="Цена"
//!     sort_field="price"
//!     align="right"
//!     ...
//! />
//! ```

use crate::shared::list_utils::{get_sort_class, get_sort_indicator};
use crate::shared::table_utils::{clear_resize_flag, was_just_resizing};
use leptos::prelude::*;
use thaw::*;

/// Компонент сортируемой ячейки заголовка таблицы
///
/// Автоматически:
/// - Добавляет индикатор сортировки (▲▼)
/// - Обрабатывает клики для изменения сортировки
/// - Предотвращает конфликты с resize колонок
/// - Поддерживает resizable колонки
#[component]
pub fn SortableHeaderCell(
    /// Текст заголовка
    #[prop(into)]
    label: String,
    
    /// Поле для сортировки
    #[prop(into)]
    sort_field: String,
    
    /// Текущее поле сортировки из state
    #[prop(into)]
    current_sort_field: Signal<String>,
    
    /// Направление сортировки из state
    #[prop(into)]
    sort_ascending: Signal<bool>,
    
    /// Callback при клике на заголовок
    on_sort: Callback<String>,
    
    /// Минимальная ширина колонки
    #[prop(optional, default = 100.0)]
    min_width: f64,
    
    /// Выравнивание заголовка (left/right)
    #[prop(optional, default = "left")]
    align: &'static str,
    
    /// Можно ли изменять размер колонки
    #[prop(optional, default = true)]
    resizable: bool,
) -> impl IntoView {
    let sort_field_for_click = sort_field.clone();
    let sort_field_for_indicator = sort_field.clone();
    let sort_field_for_class = sort_field.clone();
    
    let handle_click = move |_| {
        // Не вызываем сортировку если только что изменяли размер колонки
        if was_just_resizing() {
            clear_resize_flag();
            return;
        }
        on_sort.run(sort_field_for_click.clone());
    };
    
    let header_style = if align == "right" {
        "cursor: pointer; justify-content: flex-end; padding-right: 12px; max-width: calc(100% - 12px);"
    } else {
        "cursor: pointer; padding-right: 12px; max-width: calc(100% - 12px);"
    };
    
    view! {
        <TableHeaderCell 
            resizable=resizable 
            min_width=min_width 
            class="resizable"
        >
            <div 
                class="table__sortable-header" 
                style=header_style
                on:click=handle_click
            >
                {label}
                <span class=move || {
                    get_sort_class(&current_sort_field.get(), &sort_field_for_class)
                }>
                    {move || {
                        get_sort_indicator(
                            &current_sort_field.get(),
                            &sort_field_for_indicator,
                            sort_ascending.get()
                        )
                    }}
                </span>
            </div>
        </TableHeaderCell>
    }
}
