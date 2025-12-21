use leptos::prelude::*;

/// DateInput component with native date picker
/// Browser automatically displays dates in locale format (dd.mm.yyyy for RU locale)
#[component]
pub fn DateInput(
    /// The date value in yyyy-mm-dd format
    #[prop(into)]
    value: Signal<String>,
    /// Callback when the date changes (receives yyyy-mm-dd format)
    on_change: impl Fn(String) + 'static,
    #[prop(optional)] style: Option<String>,
) -> impl IntoView {
    let default_style = "padding: 6px 8px; border: 1px solid #ced4da; border-radius: 4px; font-size: 0.875rem; background: #fff; width: 130px;";
    let final_style = style.unwrap_or_else(|| default_style.to_string());

    view! {
        <input
            type="date"
            prop:value=value
            on:input=move |ev| {
                on_change(event_target_value(&ev));
            }
            style=final_style
        />
    }
}
