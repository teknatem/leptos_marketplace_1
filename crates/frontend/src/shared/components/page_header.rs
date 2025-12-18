use leptos::prelude::*;

/// PageHeader component - reusable header for list pages
///
/// Mimics the structure from bolt-mpi-ui-redesign Header.tsx
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
        <div class="page-header">
            <div class="page-header__content">
                <div class="page-header__text">
                    <h1 class="page-header__title">{title}</h1>
                    {move || subtitle.get().map(|s| view! {
                        <div class="page-header__subtitle">{s}</div>
                    })}
                </div>
            </div>
            <div class="page-header__actions">
                {children()}
            </div>
        </div>
    }
}
