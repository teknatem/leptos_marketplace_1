use crate::shared::icons::icon;
use leptos::prelude::*;

/// PaginationControls component - reusable pagination controls
///
/// Mimics the structure from bolt-mpi-ui-redesign ReturnsYandex.tsx pagination
#[component]
pub fn PaginationControls(
    /// Current page (0-indexed)
    #[prop(into)]
    current_page: Signal<usize>,

    /// Total number of pages
    #[prop(into)]
    total_pages: Signal<usize>,

    /// Total count of items
    #[prop(into)]
    total_count: Signal<usize>,

    /// Current page size
    #[prop(into)]
    page_size: Signal<usize>,

    /// Callback when page changes
    on_page_change: Callback<usize>,

    /// Callback when page size changes
    on_page_size_change: Callback<usize>,

    /// Available page size options (optional, defaults to [50, 100, 200, 500])
    #[prop(optional)]
    page_size_options: Option<Vec<usize>>,
) -> impl IntoView {
    let page_size_opts = page_size_options.unwrap_or_else(|| vec![50, 100, 200, 500]);

    view! {
        <div class="pagination-controls">
            <button
                class="pagination-btn"
                on:click=move |_| on_page_change.run(0)
                disabled=move || current_page.get() == 0
                title="Первая страница"
            >
                {icon("chevrons-left")}
            </button>
            <button
                class="pagination-btn"
                on:click=move |_| {
                    let page = current_page.get();
                    if page > 0 {
                        on_page_change.run(page - 1);
                    }
                }
                disabled=move || current_page.get() == 0
                title="Предыдущая страница"
            >
                {icon("chevron-left")}
            </button>
            <span class="pagination-info">
                {move || {
                    let page = current_page.get();
                    let total = total_pages.get().max(1);
                    let count = total_count.get();
                    format!("{} / {} ({})", page + 1, total, count)
                }}
            </span>
            <button
                class="pagination-btn"
                on:click=move |_| {
                    let page = current_page.get();
                    let total = total_pages.get();
                    if page + 1 < total {
                        on_page_change.run(page + 1);
                    }
                }
                disabled=move || {
                    let page = current_page.get();
                    let total = total_pages.get();
                    page + 1 >= total
                }
                title="Следующая страница"
            >
                {icon("chevron-right")}
            </button>
            <button
                class="pagination-btn"
                on:click=move |_| {
                    let total = total_pages.get();
                    if total > 0 {
                        on_page_change.run(total - 1);
                    }
                }
                disabled=move || {
                    let page = current_page.get();
                    let total = total_pages.get();
                    page + 1 >= total
                }
                title="Последняя страница"
            >
                {icon("chevrons-right")}
            </button>
            <select
                class="page-size-select"
                on:change=move |ev| {
                    let val = event_target_value(&ev).parse().unwrap_or(100);
                    on_page_size_change.run(val);
                }
                prop:value=move || page_size.get().to_string()
            >
                {page_size_opts.iter().map(|&size| {
                    view! {
                        <option value={size.to_string()} selected=move || page_size.get() == size>
                            {size.to_string()}
                        </option>
                    }
                }).collect_view()}
            </select>
        </div>
    }
}
