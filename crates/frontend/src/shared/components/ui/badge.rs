use leptos::prelude::*;

/// Badge component with different variants
#[component]
pub fn Badge(
    /// Badge variant: "primary", "success", "warning", "error", "neutral" (default)
    #[prop(optional, into)]
    variant: MaybeProp<String>,
    /// Badge content
    children: Children,
    /// Additional CSS classes
    #[prop(optional, into)]
    class: MaybeProp<String>,
) -> impl IntoView {
    let variant_class = move || match variant.get().as_deref().unwrap_or("neutral") {
        "primary" => "badge--primary",
        "success" => "badge--success",
        "warning" => "badge--warning",
        "error" => "badge--error",
        _ => "badge--neutral",
    };

    let additional_class = move || class.get().unwrap_or_default();

    view! {
        <span class=move || format!("badge {} {}", variant_class(), additional_class())>
            {children()}
        </span>
    }
}

/// Status badge component for posted/not-posted states
#[component]
pub fn StatusBadge(
    /// Status: "posted" or "not-posted"
    #[prop(into)]
    status: Signal<String>,
    /// Badge content
    children: Children,
) -> impl IntoView {
    let status_class = move || match status.get().as_str() {
        "posted" => "badge badge--status badge--status-posted",
        _ => "badge badge--status badge--status-not-posted",
    };

    view! {
        <span class=status_class>
            {children()}
        </span>
    }
}
