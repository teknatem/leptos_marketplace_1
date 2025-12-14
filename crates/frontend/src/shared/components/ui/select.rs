use leptos::prelude::*;

/// Select component with label support
#[component]
pub fn Select(
    /// Label text (optional)
    #[prop(optional, into)]
    label: MaybeProp<String>,
    /// Current value
    #[prop(into)]
    value: Signal<String>,
    /// Change event handler
    #[prop(optional)]
    on_change: Option<Callback<String>>,
    /// Options: Vec of (value, label) tuples
    #[prop(into)]
    options: Signal<Vec<(String, String)>>,
    /// Disabled state
    #[prop(optional)]
    disabled: bool,
    /// Required attribute
    #[prop(optional)]
    required: bool,
    /// ID for the select element
    #[prop(optional, into)]
    id: MaybeProp<String>,
    /// Additional CSS classes
    #[prop(optional, into)]
    class: MaybeProp<String>,
) -> impl IntoView {
    let select_id = move || id.get().unwrap_or_default();
    let additional_class = move || class.get().unwrap_or_default();

    view! {
        <div class="form__group">
            {move || label.get().map(|l| view! {
                <label class="form__label" for=select_id>
                    {l}
                </label>
            })}
            <select
                id=select_id
                class=move || format!("form__select {}", additional_class())
                disabled=disabled
                required=required
                on:change=move |ev| {
                    if let Some(handler) = on_change {
                        handler.run(event_target_value(&ev));
                    }
                }
            >
                <For
                    each=move || options.get()
                    key=|(val, _)| val.clone()
                    children=move |(val, label)| {
                        let val_clone = val.clone();
                        let is_selected = move || value.get() == val_clone;
                        view! {
                            <option value=val selected=is_selected>
                                {label}
                            </option>
                        }
                    }
                />
            </select>
        </div>
    }
}
