use leptos::prelude::*;

/// Checkbox component
#[component]
pub fn Checkbox(
    /// Label text
    #[prop(into)]
    label: Signal<String>,
    /// Checked state
    #[prop(into)]
    checked: Signal<bool>,
    /// Change event handler
    #[prop(optional)]
    on_change: Option<Callback<bool>>,
    /// Disabled state
    #[prop(optional)]
    disabled: bool,
    /// ID for the checkbox element
    #[prop(optional, into)]
    id: MaybeProp<String>,
    /// Additional CSS classes for wrapper
    #[prop(optional, into)]
    class: MaybeProp<String>,
) -> impl IntoView {
    let checkbox_id = move || id.get().unwrap_or_default();
    let additional_class = move || class.get().unwrap_or_default();
    let wrapper_class = move || {
        if disabled {
            format!(
                "form__checkbox-wrapper form__checkbox-wrapper--disabled {}",
                additional_class()
            )
        } else {
            format!("form__checkbox-wrapper {}", additional_class())
        }
    };

    view! {
        <div class=wrapper_class>
            <input
                id=checkbox_id
                type="checkbox"
                class="form__checkbox"
                checked=move || checked.get()
                disabled=disabled
                on:change=move |ev| {
                    if let Some(handler) = on_change {
                        handler.run(event_target_checked(&ev));
                    }
                }
            />
            <label class="form__checkbox-label" for=checkbox_id>
                {label}
            </label>
        </div>
    }
}
