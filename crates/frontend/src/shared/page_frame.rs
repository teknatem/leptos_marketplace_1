//! PageFrame — standard root wrapper for every page rendered inside a tab.
//!
//! Guarantees two metadata attributes on the root DOM element:
//!   - `id`                     — `"{entity}--{category}"`, e.g. `"a012_wb_sales--list"`
//!   - `data-page-category`  — one of the PAGE_CAT_* constants
//!
//! Usage:
//! ```rust
//! use crate::shared::page_frame::PageFrame;
//! use crate::shared::page_standard::PAGE_CAT_LIST;
//!
//! #[component]
//! pub fn MyList() -> impl IntoView {
//!     view! {
//!         <PageFrame page_id="a012_wb_sales--list" category=PAGE_CAT_LIST>
//!             <div class="page__header">...</div>
//!             <div class="page__content">...</div>
//!         </PageFrame>
//!     }
//! }
//! ```

use super::page_standard::*;
use leptos::prelude::*;

/// Root wrapper that sets standard metadata on every tab page.
///
/// Automatically adds the BEM modifier class based on category:
/// - `list`      → `page`
/// - `detail`    → `page page--detail`
/// - `dashboard` → `page page--dashboard`
/// - `usecase`   → `page`
/// - `system`    → `page`
/// - `custom`    → `page page--custom`
/// - `legacy`    → `page page--legacy`
#[component]
pub fn PageFrame(
    /// HTML id in format `{entity}--{category}`, e.g. `"a012_wb_sales--list"`.
    /// Used for DOM inspection and IDE navigation.
    page_id: &'static str,
    /// One of the PAGE_CAT_* constants from `page_standard`.
    category: &'static str,
    /// Additional CSS classes appended after the base class.
    #[prop(optional)]
    class: &'static str,
    children: Children,
) -> impl IntoView {
    let base_class = match category {
        PAGE_CAT_LIST => "page",
        PAGE_CAT_DETAIL => "page page--detail",
        PAGE_CAT_DASHBOARD => "page page--dashboard",
        PAGE_CAT_USECASE => "page",
        PAGE_CAT_SYSTEM => "page",
        PAGE_CAT_CUSTOM => "page page--custom",
        PAGE_CAT_LEGACY => "page page--legacy",
        _ => "page",
    };

    let full_class = if class.is_empty() {
        base_class.to_string()
    } else {
        format!("{base_class} {class}")
    };

    view! {
        <div
            id=page_id
            class=full_class
            data-page-category=category
        >
            {children()}
        </div>
    }
}
