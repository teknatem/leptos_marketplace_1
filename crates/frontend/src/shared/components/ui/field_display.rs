//! FieldDisplay — лёгкий компонент для отображения значений readonly-полей.
//!
//! Замена `<Input value=RwSignal::new(text) attr:readonly=true />` для случаев,
//! когда поле только отображает данные и не требует реактивности.
//!
//! В отличие от Thaw `Input`, не создаёт ни одного Leptos-сигнала:
//! значение передаётся как обычная строка и рендерится в static HTML-атрибут.
//!
//! # Пример
//! ```rust
//! // Было (создаёт RwSignal):
//! <Input value=RwSignal::new(document_no.clone()) attr:readonly=true />
//!
//! // Стало (нет аллокаций в reactive runtime):
//! <FieldDisplay value=document_no />
//! ```
use leptos::prelude::*;

/// Read-only поле отображения значения.
///
/// Принимает статическую строку — без Leptos-сигналов, без reactive подписок.
/// Используй вместо `<Input attr:readonly=true>` везде, где поле не редактируется.
#[component]
pub fn FieldDisplay(
    /// Отображаемое значение
    #[prop(into)]
    value: String,
) -> impl IntoView {
    view! {
        <input
            type="text"
            readonly
            class="form__input"
            value=value
        />
    }
}

/// Read-only поле с реактивным значением (для случаев, когда значение меняется).
///
/// Используй когда источник данных реактивен (Signal/Memo), но поле не редактируется.
#[component]
pub fn FieldDisplayReactive(
    /// Реактивное значение
    #[prop(into)]
    value: Signal<String>,
) -> impl IntoView {
    view! {
        <input
            type="text"
            readonly
            class="form__input"
            prop:value=move || value.get()
        />
    }
}

/// Read-only многострочное поле отображения значения.
///
/// Используй для длинного текста (комментарии, описания, payload-фрагменты), который
/// нужно показать с переносами строк. Высота подбирается по количеству строк значения
/// в диапазоне [`min_rows`, `max_rows`]; пользователь может растянуть поле по вертикали.
#[component]
pub fn FieldDisplayMultiline(
    /// Отображаемое значение
    #[prop(into)]
    value: String,
    /// Минимальная высота в строках (по умолчанию 3).
    #[prop(optional)]
    min_rows: Option<u32>,
    /// Максимальная высота в строках без скролла (по умолчанию 12).
    #[prop(optional)]
    max_rows: Option<u32>,
) -> impl IntoView {
    let min_rows = min_rows.unwrap_or(3);
    let max_rows = max_rows.unwrap_or(12);
    let content_lines = value.lines().count().max(1) as u32;
    let rows = content_lines.clamp(min_rows, max_rows);

    view! {
        <textarea
            readonly
            class="form__textarea"
            rows=rows
        >
            {value}
        </textarea>
    }
}
