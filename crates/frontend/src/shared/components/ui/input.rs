use leptos::prelude::*;

/// Input component with label support
#[component]
pub fn Input(
    /// Label text (optional)
    #[prop(optional, into)]
    label: MaybeProp<String>,
    /// Input value
    #[prop(into)]
    value: Signal<String>,
    /// Input event handler
    #[prop(optional)]
    on_input: Option<Callback<String>>,
    /// Placeholder text
    #[prop(optional, into)]
    placeholder: MaybeProp<String>,
    /// Input type: "text" (default), "password", "email", etc.
    #[prop(optional, into)]
    input_type: MaybeProp<String>,
    /// Disabled state
    #[prop(optional)]
    disabled: bool,
    /// Required attribute
    #[prop(optional)]
    required: bool,
    /// ID for the input element
    #[prop(optional, into)]
    id: MaybeProp<String>,
    /// Autocomplete attribute
    #[prop(optional, into)]
    autocomplete: MaybeProp<String>,
    /// Additional CSS classes
    #[prop(optional, into)]
    class: MaybeProp<String>,
) -> impl IntoView {
    let input_id = move || id.get().unwrap_or_default();
    let input_placeholder = move || placeholder.get().unwrap_or_default();
    let input_t = move || input_type.get().unwrap_or_else(|| "text".to_string());
    let input_autocomplete = move || autocomplete.get().unwrap_or_default();
    let additional_class = move || class.get().unwrap_or_default();

    view! {
        <div class="form__group">
            {move || label.get().map(|l| view! {
                <label class="form__label" for=input_id>
                    {l}
                </label>
            })}
            <input
                id=input_id
                class=move || format!("form__input {}", additional_class())
                type=input_t
                value=move || value.get()
                placeholder=input_placeholder
                disabled=disabled
                required=required
                autocomplete=input_autocomplete
                on:input=move |ev| {
                    if let Some(handler) = on_input {
                        handler.run(event_target_value(&ev));
                    }
                }
            />
        </div>
    }
}
