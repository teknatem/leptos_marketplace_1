//! Page category constants for tab page standardization.
//!
//! Every page rendered inside a tab must declare:
//!   - HTML `id` in the format `{entity}--{category}` (e.g. `"a012_wb_sales--list"`)
//!   - `data-page-category` with one of the constants below
//!
//! The `--` separator makes the entity name searchable: copy the id from
//! the browser DOM Inspector, paste into IDE search, and you land in the
//! `domain/a012_wb_sales/` directory.
//!
//! After future refactoring, the `ui/{category}/` directory structure will
//! mirror the category value.

/// List of records — table with filters/pagination.
pub const PAGE_CAT_LIST: &str = "list";

/// Detail / edit form for a single record.
pub const PAGE_CAT_DETAIL: &str = "detail";

/// Analytical dashboard / chart view.
pub const PAGE_CAT_DASHBOARD: &str = "dashboard";

/// Use-case wizard / action page (imports, matching, etc.).
pub const PAGE_CAT_USECASE: &str = "usecase";

/// System administration page.
pub const PAGE_CAT_SYSTEM: &str = "system";

/// Intentionally custom design — free-form, exempt from structural checks.
pub const PAGE_CAT_CUSTOM: &str = "custom";

/// Non-standard legacy page queued for refactoring.
/// Metadata is present but inner structure may deviate from the standard.
/// Tracked by DomValidator as a refactoring backlog counter.
pub const PAGE_CAT_LEGACY: &str = "legacy";

/// Categories where standard structure (`page__header` + `page__content`) is required.
pub const STANDARD_CATEGORIES: &[&str] = &[
    PAGE_CAT_LIST,
    PAGE_CAT_DETAIL,
    PAGE_CAT_DASHBOARD,
    PAGE_CAT_USECASE,
    PAGE_CAT_SYSTEM,
];

/// Validate that a page id matches the `{entity}--{category}` format.
pub fn is_valid_page_id(id: &str) -> bool {
    let parts: Vec<&str> = id.splitn(2, "--").collect();
    parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty()
}

/// All known category values.
pub const ALL_CATEGORIES: &[&str] = &[
    PAGE_CAT_LIST,
    PAGE_CAT_DETAIL,
    PAGE_CAT_DASHBOARD,
    PAGE_CAT_USECASE,
    PAGE_CAT_SYSTEM,
    PAGE_CAT_CUSTOM,
    PAGE_CAT_LEGACY,
];

/// Return true if the category value is recognised.
pub fn is_known_category(cat: &str) -> bool {
    ALL_CATEGORIES.contains(&cat)
}
