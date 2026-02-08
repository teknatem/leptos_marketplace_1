use leptos::prelude::*;

/// PageHeader component - reusable header for list pages (BEM)
///
/// Uses unified .page__header structure with BEM naming
#[component]
pub fn PageHeader(
    /// Page title (required)
    #[prop(into)]
    title: String,

    /// Optional subtitle
    #[prop(optional, into)]
    subtitle: MaybeProp<String>,

    /// Children content (pass empty fragment if not needed)
    children: Children,
) -> impl IntoView {
    view! {
        <div class="page__header">
            <div class="page__header-left">
                <h1 class="page__title">{title}</h1>
                {move || subtitle.get().map(|s| view! {
                    <div class="page__subtitle">{s}</div>
                })}
            </div>
            <div class="page__header-right">
                {children()}
            </div>
        </div>
    }
}
