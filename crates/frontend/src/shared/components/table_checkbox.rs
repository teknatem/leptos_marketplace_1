use leptos::prelude::*;

/// Компонент чекбокса для таблицы с единым BEM-стилем
///
/// Рендерит <td> с чекбоксом внутри.
/// Клик на чекбокс не вызывает клик на строку (stop_propagation).
///
/// # BEM классы
/// - `.table__cell--checkbox` - td обёртка
/// - `.table__checkbox` - input элемент
///
/// # Пример использования
/// ```rust
/// <TableCheckbox
///     checked=Signal::derive(move || selected.get().contains(&id))
///     on_change=Callback::new(move |checked| toggle_select(id.clone(), checked))
/// />
/// ```
#[component]
pub fn TableCheckbox(
    /// Сигнал состояния чекбокса
    checked: Signal<bool>,
    /// Callback вызывается при изменении состояния
    on_change: Callback<bool>,
    /// Отключить чекбокс
    #[prop(optional)]
    disabled: bool,
) -> impl IntoView {
    view! {
        <td
            class="table__cell table__cell--checkbox"
            on:click=|e| e.stop_propagation()
        >
            <input
                type="checkbox"
                class="table__checkbox"
                prop:checked=checked
                prop:disabled=disabled
                on:change=move |ev| {
                    let checked = event_target_checked(&ev);
                    on_change.run(checked);
                }
            />
        </td>
    }
}
