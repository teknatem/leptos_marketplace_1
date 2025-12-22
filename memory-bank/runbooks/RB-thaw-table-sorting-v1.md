---
title: Runbook - Adding Client-Side Sorting to Thaw Tables
version: 1.0
date: 2025-12-21
tags: [runbook, thaw-ui, sorting, leptos]
applies_to: Leptos projects using Thaw UI tables
---

# Runbook: Adding Client-Side Sorting to Thaw Tables

## Purpose

Step-by-step procedure for adding sortable columns to Thaw UI `<Table>` components with icon-based activation.

## When to Use

- Need to sort Thaw Table data on the client side
- Want sort indicators visible in column headers
- Must avoid conflicts with column resizing

## Prerequisites

- Thaw UI table already implemented
- Data loaded into a signal
- `list_utils` module available with `Sortable` trait

## Procedure

### Step 1: Add Imports

**File**: Your table module (e.g., `list/mod.rs`)

```rust
use crate::shared::list_utils::{get_sort_class, get_sort_indicator, Sortable};
use std::cmp::Ordering;
```

### Step 2: Add Sorting State

**Location**: Inside component, after existing state declarations

```rust
// Existing state
let (items, set_items) = signal::<Vec<YourRow>>(Vec::new());

// Add these three signals
let (raw_items, set_raw_items) = signal::<Vec<YourRow>>(Vec::new());
let (sort_field, set_sort_field) = signal::<String>("default_field".to_string());
let (sort_ascending, set_sort_ascending) = signal(true);
```

**Notes**:

- `raw_items`: Holds unsorted data from server
- `items`: Holds sorted data for display
- `sort_field`: Name of currently sorted column
- `sort_ascending`: Sort direction (true = ascending)

### Step 3: Implement Sortable Trait

**Location**: After your row struct definition

```rust
impl Sortable for YourRow {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "name" => self.name.cmp(&other.name),
            "status" => self.status.cmp(&other.status),
            "created_at" => self.created_at.cmp(&other.created_at),
            _ => Ordering::Equal,
        }
    }
}
```

**Important**: Add all sortable fields from your struct.

### Step 4: Update Data Fetch

**Change**: Store fetched data in `raw_items` instead of `items`

```rust
// Before:
set_items.set(rows);

// After:
set_raw_items.set(rows);
```

### Step 5: Add Sorting Effect

**Location**: After fetch function, before event handlers

```rust
// Automatic sorting when data or parameters change
Effect::new(move |_| {
    let mut sorted = raw_items.get();
    let field = sort_field.get();
    let ascending = sort_ascending.get();

    sorted.sort_by(|a, b| {
        let cmp = a.compare_by_field(b, &field);
        if ascending { cmp } else { cmp.reverse() }
    });

    set_items.set(sorted);
});
```

**How it works**:

- Runs automatically when `raw_items`, `sort_field`, or `sort_ascending` change
- Sorts data and updates `items` signal
- UI updates automatically via reactivity

### Step 6: Add Sort Toggle Handler

**Location**: After other event handlers

```rust
let toggle_sort = move |field: &'static str| {
    if sort_field.get() == field {
        // Same field: toggle direction
        set_sort_ascending.update(|a| *a = !*a);
    } else {
        // New field: sort ascending
        set_sort_field.set(field.to_string());
        set_sort_ascending.set(true);
    }
};
```

### Step 7: Update Table Headers

**For each sortable column**, replace:

```rust
// Before:
<TableHeaderCell>"Column Name"</TableHeaderCell>

// After:
<TableHeaderCell resizable=true min_width=150.0>
    "Column Name"
    <span
        class={move || get_sort_class(&sort_field.get(), "field_name")}
        style="cursor: pointer; margin-left: 4px;"
        on:click=move |e| {
            e.stop_propagation();  // Important: prevents resize conflict
            toggle_sort("field_name");
        }
    >
        {move || get_sort_indicator("field_name", &sort_field.get(), sort_ascending.get())}
    </span>
</TableHeaderCell>
```

**Key points**:

- `"field_name"` must match field in `Sortable` implementation
- `e.stop_propagation()` prevents click from triggering resize
- Icon is separate `<span>` inside header cell

### Step 8: Add CSS Styles

**File**: `crates/frontend/static/themes/core/components.css`

```css
/* Inactive sort icons (gray) */
.sort-icon {
  display: inline-block;
  font-size: 14px;
  opacity: 0.3;
  color: var(--color-text-tertiary);
  transition: opacity 0.2s ease;
}

/* Active sort icon (bright) */
.sort-icon.active {
  opacity: 1;
  color: var(--color-success);
  font-weight: bold;
}
```

## Testing Checklist

After implementation, verify:

- [ ] Initial sort works (default field, ascending)
- [ ] Clicking icon sorts by that column (ascending)
- [ ] Clicking same icon again reverses sort (descending)
- [ ] Clicking different icon switches to new column (ascending)
- [ ] Visual indicators show correct state:
  - [ ] Active column has bright green ▲ or ▼
  - [ ] Inactive columns have gray ▲
- [ ] Column resizing still works (not blocked by sort icons)
- [ ] Sort persists during data refresh

## Common Issues

### Issue 1: Sort Icon Blocks Resize

**Symptom**: Cannot resize columns by dragging edge

**Cause**: Icon click handler doesn't stop propagation

**Fix**: Ensure `e.stop_propagation()` in icon's `on:click`

### Issue 2: Wrong Field Sorting

**Symptom**: Clicking icon sorts by wrong field

**Cause**: Field name mismatch between handler and `Sortable` trait

**Fix**: Verify field names match exactly in all three places:

1. `toggle_sort("field_name")`
2. `get_sort_indicator("field_name", ...)`
3. `Sortable::compare_by_field` match arm

### Issue 3: Icons Not Visible

**Symptom**: No sort indicators appear

**Cause**: CSS classes not applied or missing styles

**Fix**: Check browser DevTools, verify CSS file loaded

### Issue 4: Data Doesn't Sort

**Symptom**: Clicking icons has no effect

**Cause**: Using old `items` signal in table body

**Fix**: Ensure table body iterates over `items.get()`, not `raw_items.get()`

## Example Implementation

See: `crates/frontend/src/domain/a006_connection_mp/ui/list/mod.rs`

Key sections:

- Lines 1-8: Imports
- Lines 76-78: State signals
- Lines 23-56: Sortable implementation
- Lines 95-107: Sort Effect
- Lines 123-131: Toggle handler
- Lines 246-333: Updated headers

## Alternatives

### Alternative 1: Entire Cell Click

Instead of icon-only, make whole cell clickable:

```rust
<TableHeaderCell
    resizable=true
    min_width=150.0
    on:click=move |_| {
        if !was_just_resizing() {
            toggle_sort("field_name");
        }
    }
>
    "Column Name"
    <span class={move || get_sort_class(&sort_field.get(), "field_name")}>
        {move || get_sort_indicator("field_name", &sort_field.get(), sort_ascending.get())}
    </span>
</TableHeaderCell>
```

**Pros**: Larger click target  
**Cons**: Requires `was_just_resizing()` check, more complex

### Alternative 2: Server-Side Sorting

For large datasets (>1000 rows), implement server-side:

- Send sort parameters to API
- Server returns sorted data
- Requires backend changes

See: `a016_ym_returns` for server-side sorting example

## Version History

- **v1.0** (2025-12-21): Initial runbook based on a006 implementation
