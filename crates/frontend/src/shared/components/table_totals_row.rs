use leptos::prelude::*;

/// Компонент строки итогов для таблиц
///
/// Использует Leptos `children` для гибкого содержимого.
/// Рендерит <tr> со стилями строки итогов.
///
/// # BEM классы
/// - `.table__totals-row` - базовый класс строки итогов
///
/// # Пример использования
/// ```rust
/// // В <thead> после заголовков:
/// <TableTotalsRow>
///     <td class="table__cell--checkbox"></td>
///     <td>{format!("Записей: {}", totals.total_records)}</td>
///     <td class="table__cell--right">{format_number(totals.sum_amount)}</td>
/// </TableTotalsRow>
/// ```
///
/// # Как отключить итоги
/// ```rust
/// // Вариант 1: условный рендеринг
/// {move || {
///     if let Some(totals) = state.get().server_totals {
///         view! { <TableTotalsRow>...</TableTotalsRow> }.into_any()
///     } else {
///         view! { <></> }.into_any()
///     }
/// }}
///
/// // Вариант 2: if false для временного отключения
/// {move || {
///     if false {  // <- просто поменять на false
///         view! { <TableTotalsRow>...</TableTotalsRow> }.into_any()
///     } else {
///         view! { <></> }.into_any()
///     }
/// }}
/// ```
#[component]
pub fn TableTotalsRow(
    /// Содержимое строки (td элементы)
    children: Children,
    /// Дополнительные CSS классы
    #[prop(optional)]
    class: &'static str,
) -> impl IntoView {
    let row_class = if class.is_empty() {
        "table__totals-row".to_string()
    } else {
        format!("table__totals-row {}", class)
    };

    view! {
        <tr class={row_class}>
            {children()}
        </tr>
    }
}
