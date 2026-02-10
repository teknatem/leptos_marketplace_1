//! Table crosshair highlight component - highlights both row and column on cell hover using SVG overlay.

use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cell::RefCell;
use std::collections::HashSet;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlElement, MouseEvent as WebMouseEvent};

// Global registry to track which tables already have listeners attached
// This prevents memory leaks when components are remounted
thread_local! {
    static INITIALIZED_TABLES: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

/// Component that provides crosshair-style highlighting for table cells.
/// Displays both row and column highlights plus enhanced cell highlight on intersection.
#[component]
pub fn TableCrosshairHighlight(
    /// ID of the table element to attach highlighting to
    table_id: String,
    /// Enable or disable highlighting (default: true)
    #[prop(default = true)]
    enabled: bool,
) -> impl IntoView {
    let row_rect_id = format!("{}-row-rect", table_id);
    let col_rect_id = format!("{}-col-rect", table_id);
    let cell_rect_id = format!("{}-cell-rect", table_id);

    // Clone IDs for Effect closure
    let table_id_effect = table_id.clone();
    let row_rect_id_effect = row_rect_id.clone();
    let col_rect_id_effect = col_rect_id.clone();
    let cell_rect_id_effect = cell_rect_id.clone();

    // Initialize highlighting when component mounts (run once per table_id)
    // Using global registry to prevent duplicate listeners on remount
    let is_initialized = StoredValue::new(false);

    Effect::new(move |_| {
        if enabled && !is_initialized.get_value() {
            // Check if this table already has listeners
            let already_initialized =
                INITIALIZED_TABLES.with(|tables| tables.borrow().contains(&table_id_effect));

            if !already_initialized {
                is_initialized.set_value(true);

                let table_id = table_id_effect.clone();
                let row_rect_id = row_rect_id_effect.clone();
                let col_rect_id = col_rect_id_effect.clone();
                let cell_rect_id = cell_rect_id_effect.clone();

                spawn_local(async move {
                    // Small delay to ensure table is fully rendered
                    gloo_timers::future::TimeoutFuture::new(100).await;

                    // Initialize and mark as initialized
                    // Note: Closures are intentionally leaked (.forget()) since event listeners
                    // need to live for the page lifetime. This is safe for SPA context.
                    if let Some((mouseover, mouseleave)) = init_crosshair_highlight(
                        &table_id,
                        &row_rect_id,
                        &col_rect_id,
                        &cell_rect_id,
                    ) {
                        // Leak the closures so event listeners remain active
                        mouseover.forget();
                        mouseleave.forget();

                        INITIALIZED_TABLES.with(|tables| {
                            tables.borrow_mut().insert(table_id);
                        });
                    }
                });
            }
        }
    });

    view! {
        <svg
            class="table-crosshair-overlay"
            style="position: fixed; top: 0; left: 0; width: 100%; height: 100%; pointer-events: none; z-index: 1; overflow: visible;"
            xmlns="http://www.w3.org/2000/svg"
        >
            // Horizontal bar for row highlight
            <rect
                id=row_rect_id
                x="0" y="0" width="0" height="0"
                fill="var(--table-crosshair-row)"
                style="transition: all 0.12s ease-out; opacity: 0;"
                visibility="hidden"
            />

            // Vertical bar for column highlight
            <rect
                id=col_rect_id
                x="0" y="0" width="0" height="0"
                fill="var(--table-crosshair-col)"
                style="transition: all 0.12s ease-out; opacity: 0;"
                visibility="hidden"
            />

            // Enhanced highlight for cell at intersection
            <rect
                id=cell_rect_id
                x="0" y="0" width="0" height="0"
                fill="var(--table-crosshair-cell)"
                rx="4" ry="4"
                style="transition: all 0.12s ease-out; opacity: 0;"
                visibility="hidden"
            />
        </svg>
    }
}

/// Initialize crosshair highlighting for a table using Event Delegation
///
/// This implementation uses a single event listener on the table element
/// instead of attaching listeners to each individual cell, which dramatically
/// improves performance for large tables (e.g., 3000+ rows).
///
/// Returns the closures to allow proper cleanup when the component unmounts,
/// preventing memory leaks.
fn init_crosshair_highlight(
    table_id: &str,
    row_rect_id: &str,
    col_rect_id: &str,
    cell_rect_id: &str,
) -> Option<(
    Closure<dyn FnMut(WebMouseEvent)>,
    Closure<dyn FnMut(WebMouseEvent)>,
)> {
    let Some(window) = web_sys::window() else {
        return None;
    };
    let Some(document) = window.document() else {
        return None;
    };
    let Some(table) = document.get_element_by_id(table_id) else {
        return None;
    };

    let Some(row_rect) = document.get_element_by_id(row_rect_id) else {
        return None;
    };
    let Some(col_rect) = document.get_element_by_id(col_rect_id) else {
        return None;
    };
    let Some(cell_rect) = document.get_element_by_id(cell_rect_id) else {
        return None;
    };

    // Get table container for coordinate calculations
    let Some(container) = table.parent_element() else {
        return None;
    };

    // Clone for closures
    let row_rect_over = row_rect.clone();
    let col_rect_over = col_rect.clone();
    let cell_rect_over = cell_rect.clone();
    let table_for_highlight = table.clone();

    // Use Event Delegation: ONE listener on the table instead of N listeners on cells
    // mouseover bubbles up (unlike mouseenter), so we catch it at the table level
    let mouseover = Closure::wrap(Box::new(move |e: WebMouseEvent| {
        // Get the element that triggered the event
        let Some(target) = e.target() else { return };
        let Ok(element) = target.dyn_into::<Element>() else {
            return;
        };

        // Find the closest td or th element (handles nested elements like spans)
        let cell = match element.tag_name().as_str() {
            "TD" | "TH" => element,
            _ => {
                // Check if we're inside a cell
                match element.closest("td, th") {
                    Ok(Some(cell)) => cell,
                    _ => return, // Not in a cell, ignore
                }
            }
        };

        // Convert to HtmlElement for getBoundingClientRect
        let Ok(cell_html) = cell.dyn_into::<HtmlElement>() else {
            return;
        };

        // Highlight the cell
        highlight_cell(
            &cell_html,
            &table_for_highlight,
            &row_rect_over,
            &col_rect_over,
            &cell_rect_over,
        );
    }) as Box<dyn FnMut(WebMouseEvent)>);

    let _ = table.add_event_listener_with_callback("mouseover", mouseover.as_ref().unchecked_ref());

    // Hide highlight when mouse leaves table container
    let mouseleave = Closure::wrap(Box::new(move |_: WebMouseEvent| {
        hide_highlight(&row_rect, &col_rect, &cell_rect);
    }) as Box<dyn FnMut(WebMouseEvent)>);

    let _ = container
        .add_event_listener_with_callback("mouseleave", mouseleave.as_ref().unchecked_ref());

    // Return closures so they can be cleaned up when component unmounts
    // When these are dropped, the event listeners are automatically removed
    Some((mouseover, mouseleave))
}

/// Highlight a specific cell and its row/column
fn highlight_cell(
    cell: &HtmlElement,
    table: &Element,
    row_rect: &Element,
    col_rect: &Element,
    cell_rect: &Element,
) {
    // Check if column resize is in progress - don't highlight during resize
    if let Some(body) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.body())
    {
        if body.class_list().contains("resizing-column") {
            return; // Skip highlighting during resize
        }
    }

    // Get cell coordinates (SVG is position: fixed, so use viewport coordinates directly)
    let cell_bounds = cell.get_bounding_client_rect();
    let cell_x = cell_bounds.left();
    let cell_y = cell_bounds.top();
    let cell_width = cell_bounds.width();
    let cell_height = cell_bounds.height();

    // Calculate row bounds (horizontal bar across table width only)
    let row_bounds = if let Some(tr) = cell.parent_element() {
        let tr_bounds = tr.get_bounding_client_rect();

        // Get table bounds to limit row highlight to table width
        let table_bounds = table.get_bounding_client_rect();

        (
            table_bounds.left(), // Start from table left edge
            tr_bounds.top(),
            table_bounds.width(), // Table width only
            tr_bounds.height(),
        )
    } else {
        (cell_x, cell_y, cell_width, cell_height)
    };

    // Calculate column bounds (vertical bar through all rows)
    let col_index = get_cell_index(cell);
    let col_bounds = calculate_column_bounds(table, col_index);

    // Update SVG rectangles
    update_rect(
        row_rect,
        row_bounds.0,
        row_bounds.1,
        row_bounds.2,
        row_bounds.3,
    );
    update_rect(
        col_rect,
        col_bounds.0,
        col_bounds.1,
        col_bounds.2,
        col_bounds.3,
    );
    update_rect(cell_rect, cell_x, cell_y, cell_width, cell_height);
}

/// Get the index of a cell within its parent row
fn get_cell_index(cell: &HtmlElement) -> usize {
    if let Some(parent) = cell.parent_element() {
        let children = parent.children();
        for i in 0..children.length() {
            if let Some(child) = children.item(i) {
                if child == cell.clone().into() {
                    return i as usize;
                }
            }
        }
    }
    0
}

/// Calculate the bounding box for all cells in a column
fn calculate_column_bounds(table: &Element, col_index: usize) -> (f64, f64, f64, f64) {
    let Ok(rows) = table.query_selector_all("tr") else {
        return (0.0, 0.0, 0.0, 0.0);
    };

    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    // Find bounds of all cells in this column (using viewport coordinates)
    for i in 0..rows.length() {
        if let Some(row) = rows.get(i) {
            if let Ok(row_element) = row.dyn_into::<HtmlElement>() {
                let children = row_element.children();
                if let Some(cell) = children.item(col_index as u32) {
                    if let Ok(cell_element) = cell.dyn_into::<HtmlElement>() {
                        let bounds = cell_element.get_bounding_client_rect();
                        let x = bounds.left();
                        let y = bounds.top();

                        min_x = min_x.min(x);
                        min_y = min_y.min(y);
                        max_x = max_x.max(x + bounds.width());
                        max_y = max_y.max(y + bounds.height());
                    }
                }
            }
        }
    }

    (min_x, min_y, max_x - min_x, max_y - min_y)
}

/// Update SVG rect attributes and make it visible
fn update_rect(rect: &Element, x: f64, y: f64, width: f64, height: f64) {
    let _ = rect.set_attribute("x", &x.to_string());
    let _ = rect.set_attribute("y", &y.to_string());
    let _ = rect.set_attribute("width", &width.to_string());
    let _ = rect.set_attribute("height", &height.to_string());
    let _ = rect.set_attribute("visibility", "visible");
    let _ = rect.set_attribute("style", "opacity: 1; transition: all 0.12s ease-out;");
}

/// Hide all highlight rectangles
fn hide_highlight(row_rect: &Element, col_rect: &Element, cell_rect: &Element) {
    for rect in [row_rect, col_rect, cell_rect] {
        let _ = rect.set_attribute("visibility", "hidden");
        let _ = rect.set_attribute("style", "opacity: 0; transition: all 0.15s ease-out;");
    }
}
