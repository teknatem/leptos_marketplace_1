use leptos::prelude::*;

/// Radio button component
#[component]
pub fn Radio(
    /// Label text
    #[prop(into)]
    label: Signal<String>,
    /// Radio value
    #[prop(into)]
    value: String,
    /// Current selected value
    #[prop(into)]
    checked_value: Signal<String>,
    /// Change event handler
    #[prop(optional)]
    on_change: Option<Callback<String>>,
    /// Name attribute (for grouping)
    #[prop(into)]
    name: String,
    /// Disabled state
    #[prop(optional)]
    disabled: bool,
    /// ID for the radio element
    #[prop(optional, into)]
    id: MaybeProp<String>,
) -> impl IntoView {
    let value_for_id = value.clone();
    let value_for_check = value.clone();
    let value_for_change = value.clone();

    let radio_id = id
        .get()
        .unwrap_or_else(|| format!("radio-{}", value_for_id));
    let is_checked = move || checked_value.get() == value_for_check;
    let wrapper_class = move || {
        if disabled {
            "form__radio-wrapper form__radio-wrapper--disabled"
        } else {
            "form__radio-wrapper"
        }
    };

    view! {
        <div class=wrapper_class>
            <input
                id=radio_id.clone()
                type="radio"
                class="form__radio"
                name=name.clone()
                value=value
                checked=is_checked
                disabled=disabled
                on:change=move |_| {
                    if let Some(handler) = on_change {
                        handler.run(value_for_change.clone());
                    }
                }
            />
            <label class="form__radio-label" for=radio_id>
                {label}
            </label>
        </div>
    }
}

/// Radio group component
#[component]
pub fn RadioGroup(
    /// Label for the group
    #[prop(optional, into)]
    label: MaybeProp<String>,
    /// Current selected value
    #[prop(into)]
    value: Signal<String>,
    /// Change event handler
    #[prop(optional)]
    on_change: Option<Callback<String>>,
    /// Name attribute (for grouping)
    #[prop(into)]
    name: String,
    /// Options: Vec of (value, label) tuples
    #[prop(into)]
    options: Signal<Vec<(String, String)>>,
    /// Disabled state
    #[prop(optional)]
    disabled: bool,
) -> impl IntoView {
    view! {
        <div class="form__group">
            {move || label.get().map(|l| view! {
                <label class="form__label">{l}</label>
            })}
            <div class="form__radio-group">
                <For
                    each=move || options.get()
                    key=|(val, _)| val.clone()
                    children=move |(val, lbl)| {
                        let on_change_inner = move |new_val: String| {
                            if let Some(handler) = on_change {
                                handler.run(new_val);
                            }
                        };
                        view! {
                            <Radio
                                label=lbl
                                value=val
                                checked_value=value
                                on_change=Callback::new(on_change_inner)
                                name=name.clone()
                                disabled=disabled
                            />
                        }
                    }
                />
            </div>
        </div>
    }
}
