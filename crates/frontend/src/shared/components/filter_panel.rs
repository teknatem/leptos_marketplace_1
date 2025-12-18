use crate::shared::icons::icon;
use leptos::prelude::*;

/// FilterPanel component - collapsible filter panel with pagination
///
/// Mimics the structure from bolt-mpi-ui-redesign ReturnsYandex.tsx filter-panel
#[component]
pub fn FilterPanel(
    /// Whether the filter panel is expanded
    #[prop(into)]
    is_expanded: RwSignal<bool>,

    /// Number of active filters (for badge display)
    #[prop(into)]
    active_filters_count: Signal<usize>,

    /// Pagination controls (children/slot)
    #[prop(into)]
    pagination_controls: ChildrenFn,

    /// Filter content (form fields)
    #[prop(into)]
    filter_content: ChildrenFn,

    /// Filter tags (active filter chips) - optional
    #[prop(optional, into)]
    filter_tags: Option<ChildrenFn>,
) -> impl IntoView {
    let toggle_expanded = move |_| {
        is_expanded.update(|e| *e = !*e);
    };

    view! {
        <div class="filter-panel">
            <div class="filter-panel-header">
                <div
                    class="filter-panel-header__left"
                    on:click=toggle_expanded
                >
                    <svg
                        width="16"
                        height="16"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        class=move || {
                            if is_expanded.get() {
                                "filter-panel__chevron filter-panel__chevron--expanded"
                            } else {
                                "filter-panel__chevron"
                            }
                        }
                    >
                        <polyline points="6 9 12 15 18 9"></polyline>
                    </svg>
                    {icon("filter")}
                    <span class="filter-panel__title">"Фильтры"</span>
                    {move || {
                        let count = active_filters_count.get();
                        if count > 0 {
                            view! {
                                <span class="badge badge--primary">{count}</span>
                            }.into_any()
                        } else {
                            view! { <></> }.into_any()
                        }
                    }}
                </div>
                <div class="filter-panel-header__center">
                    {pagination_controls()}
                </div>
            </div>

            <div class=move || {
                if is_expanded.get() {
                    "filter-panel__collapsible filter-panel__collapsible--expanded"
                } else {
                    "filter-panel__collapsible filter-panel__collapsible--collapsed"
                }
            }>
                <div class="filter-panel-content">
                    {filter_content()}
                    {filter_tags.as_ref().map(|tags| view! {
                        <div>{tags()}</div>
                    })}
                </div>
            </div>
        </div>
    }
}

/// FilterTag component - individual filter tag/chip
#[component]
pub fn FilterTag(
    /// Tag label
    #[prop(into)]
    label: String,

    /// Callback when remove is clicked
    on_remove: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="filter-tag">
            <span>{label}</span>
            <svg
                width="12"
                height="12"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="filter-tag__remove"
                on:click=move |e| {
                    e.stop_propagation();
                    on_remove.run(());
                }
            >
                <line x1="18" y1="6" x2="6" y2="18"></line>
                <line x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
        </div>
    }
}
