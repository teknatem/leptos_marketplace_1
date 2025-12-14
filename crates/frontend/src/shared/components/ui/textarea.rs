use leptos::prelude::*;

/// Textarea component with label support
#[component]
pub fn Textarea(
    /// Label text (optional)
    #[prop(optional, into)]
    label: MaybeProp<String>,
    /// Textarea value
    #[prop(into)]
    value: Signal<String>,
    /// Input event handler
    #[prop(optional)]
    on_input: Option<Callback<String>>,
    /// Placeholder text
    #[prop(optional, into)]
    placeholder: MaybeProp<String>,
    /// Disabled state
    #[prop(optional)]
    disabled: bool,
    /// Required attribute
    #[prop(optional)]
    required: bool,
    /// Rows attribute
    #[prop(optional)]
    rows: Option<u32>,
    /// ID for the textarea element
    #[prop(optional, into)]
    id: MaybeProp<String>,
    /// Additional CSS classes
    #[prop(optional, into)]
    class: MaybeProp<String>,
) -> impl IntoView {
    let textarea_id = move || id.get().unwrap_or_default();
    let textarea_placeholder = move || placeholder.get().unwrap_or_default();
    let additional_class = move || class.get().unwrap_or_default();
    let textarea_rows = rows.unwrap_or(3);

    view! {
        <div class="form__group">
            {move || label.get().map(|l| view! {
                <label class="form__label" for=textarea_id>
                    {l}
                </label>
            })}
            <textarea
                id=textarea_id
                class=move || format!("form__textarea {}", additional_class())
                placeholder=textarea_placeholder
                disabled=disabled
                required=required
                rows=textarea_rows
                on:input=move |ev| {
                    if let Some(handler) = on_input {
                        handler.run(event_target_value(&ev));
                    }
                }
            >
                {move || value.get()}
            </textarea>
        </div>
    }
}
