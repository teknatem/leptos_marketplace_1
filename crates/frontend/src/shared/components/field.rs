use leptos::prelude::*;

#[component]
pub fn FieldGroup(
    #[prop(optional, into)]
    class: MaybeProp<String>,
    children: Children,
) -> impl IntoView {
    let class_name = move || match class.get() {
        Some(extra) if !extra.is_empty() => format!("field-group {}", extra),
        _ => "field-group".to_string(),
    };

    view! {
        <div class=class_name>
            {children()}
        </div>
    }
}

#[component]
pub fn Field(
    #[prop(optional, into)]
    class: MaybeProp<String>,
    children: Children,
) -> impl IntoView {
    let class_name = move || match class.get() {
        Some(extra) if !extra.is_empty() => format!("field {}", extra),
        _ => "field".to_string(),
    };

    view! {
        <div class=class_name>
            {children()}
        </div>
    }
}

#[component]
pub fn FieldContent(
    #[prop(optional, into)]
    class: MaybeProp<String>,
    children: Children,
) -> impl IntoView {
    let class_name = move || match class.get() {
        Some(extra) if !extra.is_empty() => format!("field__content {}", extra),
        _ => "field__content".to_string(),
    };

    view! {
        <div class=class_name>
            {children()}
        </div>
    }
}

#[component]
pub fn FieldLabel(
    #[prop(optional, into)]
    class: MaybeProp<String>,
    #[prop(optional, into)]
    r#for: MaybeProp<String>,
    children: Children,
) -> impl IntoView {
    let class_name = move || match class.get() {
        Some(extra) if !extra.is_empty() => format!("field__label {}", extra),
        _ => "field__label".to_string(),
    };
    let html_for = move || r#for.get().unwrap_or_default();

    view! {
        <label class=class_name for=html_for>
            {children()}
        </label>
    }
}

#[component]
pub fn FieldDescription(
    #[prop(optional, into)]
    class: MaybeProp<String>,
    children: Children,
) -> impl IntoView {
    let class_name = move || match class.get() {
        Some(extra) if !extra.is_empty() => format!("field__description {}", extra),
        _ => "field__description".to_string(),
    };

    view! {
        <div class=class_name>
            {children()}
        </div>
    }
}
