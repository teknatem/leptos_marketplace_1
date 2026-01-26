use leptos::prelude::*;

/// Button component with variants (primary, secondary, ghost) and sizes (sm, md)
#[component]
pub fn Button(
    /// Button variant: "primary" (default), "secondary", or "ghost"
    #[prop(optional, into)]
    variant: MaybeProp<String>,
    /// Button size: "md" (default) or "sm"
    #[prop(optional, into)]
    size: MaybeProp<String>,
    /// Additional CSS classes
    #[prop(optional, into)]
    class: MaybeProp<String>,
    /// Button type attribute
    #[prop(optional, into)]
    button_type: MaybeProp<String>,
    /// Disabled state (reactive)
    #[prop(optional, into)]
    disabled: MaybeProp<bool>,
    /// Click event handler
    #[prop(optional)]
    on_click: Option<Callback<leptos::ev::MouseEvent>>,
    /// Button children (content)
    children: Children,
) -> impl IntoView {
    let variant_class = move || match variant.get().as_deref().unwrap_or("primary") {
        "secondary" => "button--secondary",
        "ghost" => "button--ghost",
        _ => "button--primary",
    };

    let size_class = move || {
        if size.get().as_deref() == Some("sm") {
            "button--smallall"
        } else {
            ""
        }
    };

    let additional_class = move || class.get().unwrap_or_default();
    let btn_type = move || button_type.get().unwrap_or_else(|| "button".to_string());

    view! {
        <button
            type=btn_type
            class=move || format!("button {} {} {}", variant_class(), size_class(), additional_class())
            disabled=move || disabled.get().unwrap_or(false)
            on:click=move |ev| {
                if let Some(handler) = on_click {
                    handler.run(ev);
                }
            }
        >
            {children()}
        </button>
    }
}
