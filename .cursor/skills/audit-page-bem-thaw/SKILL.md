---
name: audit-page-bem-thaw
description: Audit and refactor a frontend page to BEM + Thaw UI standards. Interactive mode with user confirmation for each fix.
---

# Page Audit & Refactoring Skill

This skill audits frontend pages for compliance with BEM + Thaw UI standards and applies fixes interactively with user confirmation.

## When to Use This Skill

**Trigger scenarios:**

- User says "audit page [name]" or "audit [page_name]"
- User wants to refactor page to standards
- User asks "does [page] follow standards?"
- User requests page structure compliance check

**Examples:**

- "audit page a016_ym_returns"
- "audit a008_marketplace_sales"
- "refactor a005_marketplace to BEM"

## Severity Levels

Issues are classified by impact on functionality and maintainability:

- **Critical**: Layout broken, page doesn't render correctly
  - Wrong container class (`content` instead of `page`)
  - Missing required structure elements
  - **Action**: Must fix immediately

- **High**: BEM structure violations, inconsistent with project standards
  - Wrong header structure (`page-header` instead of `page__header`)
  - Filter panel structure violations
  - Missing required imports
  - **Action**: Should fix before merge

- **Medium**: Style inconsistencies, suboptimal patterns
  - Native buttons instead of Thaw Button (should migrate to Thaw)
  - Native tables instead of Thaw Table (should migrate to Thaw)
  - Wrong component usage (native HTML instead of Thaw)
  - Missing custom component usage
  - **Action**: Fix when refactoring

- **Low**: Stylistic issues, minor improvements
  - Inline styles (when CSS class would work)
  - Unnecessary nesting
  - Missing state.rs for complex pages
  - **Action**: Nice to have

## Audit Process

### Step 0: Pre-flight Check

Before starting the audit, perform these checks:

1. **Verify file exists:**

   ```
   Use Read tool to check the page file exists
   ```

2. **Check for existing linter errors:**

   ```
   Use ReadLints tool to see baseline errors
   Document count before changes
   ```

3. **Determine page type:**
   - List view (standard table)
   - Tree view (hierarchical data)
   - Details/Form view
   - Custom layout

4. **Report pre-flight status:**
   ```markdown
   ## Pre-flight Check

   - File exists: ✅/❌
   - Baseline linter errors: [count]
   - Page type: [list/tree/details/custom]
   - Ready for audit: ✅/❌
   ```

### Step 1: Analysis Phase

Read the target page file and run through this checklist:

#### 1. Container Structure

- [ ] Uses `class="page"` as root container (NOT "content")
- [ ] No inline styles on root div (like `style="padding: 20px;"`)

#### 2. Imports & Dependencies

- [ ] Has `use thaw::*;` if using Thaw components
- [ ] Imports custom components from `crate::shared::components::`:
  - `Badge` for status indicators
  - `DateRangePicker` for date filters
  - `PaginationControls` for table pagination
  - `Button` for enhanced buttons
- [ ] Imports layout context: `use crate::layout::global_context::AppGlobalContext;`
- [ ] Imports icons: `use crate::shared::icons::icon;`

#### 3. Header Structure

- [ ] Uses `page__header` (NOT "page-header" - check the underscores!)
- [ ] Has proper children: `page__header-left` and `page__header-right`
- [ ] Title uses `<h1 class="page__title">` (NOT "page-header\_\_title")
- [ ] No unnecessary nesting (icon/text wrappers)

#### 4. Table Structure (List View)

- [ ] **REQUIRED**: Tables use Thaw `Table`, `TableHeader`, `TableBody`, `TableRow`, `TableCell`
- [ ] Thaw Table imported: `use thaw::*;`
- [ ] NO native `<table>` elements (should be migrated to Thaw Table)
- [ ] If native table found with BEM classes - flag as Medium severity (needs migration)

#### 5. Tree Structure (Tree View)

- [ ] Uses `<div class="tree-container">` as wrapper
- [ ] Tree table uses `table__data tree-table` classes
- [ ] Expand/collapse icons use consistent pattern
- [ ] Indentation uses CSS `padding-left` based on depth
- [ ] Node rows have `data-depth` attribute

#### 6. Filter Panel (if present)

- [ ] Uses `filter-panel` root class
- [ ] Has `filter-panel-header` with BEM children: `filter-panel-header__left`, `filter-panel-header__center`, `filter-panel-header__right`
- [ ] Has `filter-panel-content` for filter controls
- [ ] Uses `<Show>` for collapsible behavior
- [ ] Filter controls use Thaw components or custom DateRangePicker

#### 7. Component Usage

- [ ] **REQUIRED**: Buttons use Thaw `Button` component with `ButtonAppearance`
- [ ] NO native `<button>` elements (should be migrated to Thaw Button)
- [ ] If native button found with `.button` class - flag as Medium severity (needs migration)
- [ ] Inputs use Thaw `Input`, `Select`, `DatePicker` components
- [ ] Layout uses Thaw `Flex`, `Space` components (optional)
- [ ] Badges use custom `Badge` component
- [ ] Minimal inline styles (only when absolutely necessary)

#### 8. State Management (Complex Pages)

- [ ] Pages with multiple filters/sorting have separate `state.rs` module
- [ ] State uses `RwSignal` or proper reactive primitives
- [ ] Loading/error states are handled
- [ ] Selected items use `HashSet<String>` pattern

#### 9. CSS Class Validation

- [ ] All classes exist in `layout.css` or `components.css`
- [ ] No custom classes without documentation
- [ ] Modifiers follow BEM convention (`--modifier`)

### Step 2: Report Phase

Present findings in this format:

````markdown
## Audit Report: [page_name]

**File:** `crates/frontend/src/domain/[path]/mod.rs`

### Summary

- Issues Found: [count]
- Critical: [count] | High: [count] | Medium: [count] | Low: [count]
- Estimated Effort: [Simple/Moderate/Complex]

### Issues by Severity

#### Issue 1: [Title] (Critical/High/Medium/Low)

**Problem:** [Clear description of the issue]
**Location:** Line [X] or Lines [X-Y]
**Severity:** [Critical/High/Medium/Low]
**Impact:** [What this breaks or makes inconsistent]
**Fix:** [What needs to be changed]

Current code:

```rust
[problematic code snippet]
```
````

Fixed code:

```rust
[corrected code snippet]
```

[Repeat for each issue, grouped by severity: Critical first, then High, Medium, Low]

````

**Important:** Group issues by severity in the report, with Critical issues listed first.

### Step 3: Interactive Refactoring

**Batch Operations:**

Before starting individual fixes, offer a preview mode:

```markdown
Found [N] issues. How would you like to proceed?

Options:
1. **Preview All** - Show all proposed changes before applying
2. **Apply All** - Apply all fixes automatically (recommended for low-risk changes)
3. **Interactive** - Review and confirm each fix individually
4. **Apply All Similar** - Apply all fixes of the same type (e.g., all container class fixes)
````

Use AskQuestion tool with these options.

**Interactive Mode (default):**

For each issue found:

1. **Show the problem:**

   ```rust
   // Current code (problematic)
   <div class="page-header">
       ...
   </div>
   ```

2. **Show the fix:**

   ```rust
   // Fixed code (compliant)
   <div class="page__header">
       <div class="page__header-left">
           <h1 class="page__title">...</h1>
       </div>
       <div class="page__header-right">
           ...
       </div>
   </div>
   ```

3. **Ask for confirmation using AskQuestion tool:**

   ```
   Apply fix for Issue [N]: [Title] (Severity: [level])?

   Options:
   - Yes - apply this fix
   - No - skip this fix
   - Apply All Remaining - apply all remaining fixes without asking
   - Apply All [Severity] - apply all remaining fixes of this severity level
   - Apply All Similar - apply all fixes of this type (e.g., all "container class" fixes)
   - Cancel - stop audit
   ```

4. **Track batch state:**
   - If user selects "Apply All Similar", identify the issue type and apply all matching fixes
   - If user selects "Apply All [Severity]", apply all fixes of that severity without asking
   - Keep count of applied vs skipped fixes

5. **Apply if confirmed** using StrReplace tool

### Step 3.5: CSS Class Validation

After applying fixes (or during analysis), validate all CSS classes:

1. **Extract all CSS classes from the page:**

   ```
   Search for class=" and class: patterns
   Build list of all classes used
   ```

2. **Check against standard stylesheets:**

   ```
   Read crates/frontend/static/themes/core/layout.css
   Read crates/frontend/static/themes/core/components.css
   Build list of available classes
   ```

3. **Validate each class:**
   - ✅ Exists in layout.css or components.css
   - ⚠️ Custom class (not in standards) - document why it's needed
   - ❌ Undefined class (typo or missing definition)

4. **Report validation results:**

   ```markdown
   ### CSS Class Validation

   **Standard classes found:** [count]
   **Custom classes:** [count]

   - `custom-class-name` - Used in [location] - [reason/impact]

   **Undefined classes (potential issues):** [count]

   - `typo-clas` - Line [X] - Should be `typo-class`?
   ```

### Step 3.6: Thaw Table Migration Guide

When auditing pages with native HTML tables, check if they should be migrated to Thaw Table components. Flag as **High** severity if native table is found.

#### When to Migrate to Thaw Table

**Always migrate if:**

- Page needs sortable columns
- Page needs column resizing
- Page needs row selection (checkboxes)
- Page has complex interactions
- Page is a primary list view

**Keep native table only if:**

- Simple read-only data display
- Tree structure (use BEM tree-table)
- Custom layout requirements

#### Migration Steps

**Step 1: Update imports**

```rust
use thaw::*;
use crate::shared::components::table::{
    TableCellCheckbox, TableHeaderCheckbox,
    TableCrosshairHighlight, TableCellMoney
};
use crate::shared::table_utils::init_column_resize;
```

**Step 2: Add table constants**

```rust
const TABLE_ID: &str = "your-table-id";
const COLUMN_WIDTHS_KEY: &str = "your_page_column_widths";
```

**Step 3: Replace table structure**

```rust
// Before (native)
<table class="table__data">
    <thead><tr><th>Column</th></tr></thead>
    <tbody><tr><td>Data</td></tr></tbody>
</table>

// After (Thaw)
<div class="table-wrapper">
    <TableCrosshairHighlight table_id=TABLE_ID.to_string() />

    <Table attr:id=TABLE_ID attr:style="width: 100%; min-width: 900px;">
        <TableHeader>
            <TableRow>
                <TableHeaderCell resizable=false min_width=120.0 class="resizable">
                    {"Column"}
                </TableHeaderCell>
            </TableRow>
        </TableHeader>
        <TableBody>
            {move || items.get().into_iter().map(|item| {
                view! {
                    <TableRow>
                        <TableCell>
                            <TableCellLayout>{item.data}</TableCellLayout>
                        </TableCell>
                    </TableRow>
                }
            }).collect_view()}
        </TableBody>
    </Table>
</div>
```

**Step 4: Add checkboxes for multi-select**

```rust
// Header checkbox
<TableHeaderCheckbox
    items=items_signal
    selected=selected_signal
    get_id=Callback::new(|row: YourDto| row.id.clone())
    on_change=Callback::new(toggle_all)
/>

// Row checkbox
<TableCellCheckbox
    item_id=item.id.clone()
    selected=selected_signal
    on_change=Callback::new(move |(id, checked)| toggle_select(id, checked))
/>
```

**Step 5: Add column resize initialization**

```rust
let resize_initialized = leptos::prelude::StoredValue::new(false);
Effect::new(move |_| {
    if !resize_initialized.get_value() {
        resize_initialized.set_value(true);
        spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(100).await;
            init_column_resize(TABLE_ID, COLUMN_WIDTHS_KEY);
        });
    }
});
```

**Step 6: Use For loop instead of map**

```rust
// Thaw Table prefers For over map for better performance
<For
    each=move || items.get()
    key=|item| item.id.clone()
    children=move |item| {
        view! { <TableRow>...</TableRow> }
    }
/>
```

**Step 7: Replace UiButton with Thaw Button**

```rust
// Before
<UiButton variant="primary".to_string()>

// After
<Button appearance=ButtonAppearance::Primary>
```

### Step 4: Verification Phase

After all approved changes:

1. **Run linter check:**

   ```
   Use ReadLints tool on modified files
   Compare with baseline from Pre-flight Check
   ```

2. **Verify no new linter errors introduced:**
   - If new errors appear, review changes
   - Offer to fix linter errors or rollback

3. **Report completion:**

   ```markdown
   ## Audit Complete: [page_name]

   - Fixes Applied: [count]
   - Fixes Skipped: [count]
   - Linter Errors: [count or "None"]

   ### Next Steps

   - Test page in browser at `http://localhost:3000`
   - Verify layout appears correctly
   - Check responsive behavior
   ```

## Detailed Audit Rules

### Container Rules

**Rule:** Root must be `<div class="page">`

**Check:**

```rust
// Find the root element in the view! macro
// It should be: <div class="page">
```

**Common violations:**

- `<div class="content">` - old pattern
- `<div style="padding: 20px;">` - inline styles
- `<div class="page page--wide">` - unnecessary modifier

### Header Rules

**Rule:** Must use BEM `page__header` structure

**Check:**

```rust
// Must have this exact structure:
<div class="page__header">
    <div class="page__header-left">
        <h1 class="page__title">...</h1>
    </div>
    <div class="page__header-right">
        // actions
    </div>
</div>
```

**Common violations:**

- `page-header` instead of `page__header` (single hyphen vs double underscore)
- `page-header__content` → `page-header__icon` → `page-header__text` → `page-header__title` (over-nesting)
- `page-header__actions` instead of `page__header-right`

### Table Rules

**Rule:** ALWAYS use Thaw Table components (REQUIRED)

**Standard pattern (all tables):**

```rust
use thaw::*;

<Table>
    <TableHeader>
        <TableRow>
            <TableHeaderCell resizable=false class="fixed-checkbox-column">
                <input type="checkbox" class="table__checkbox" ... />
            </TableHeaderCell>
            <TableHeaderCell resizable=true min_width=150.0>
                "Column Name"
            </TableHeaderCell>
        </TableRow>
    </TableHeader>
    <TableBody>
        {move || items.get().into_iter().map(|row| {
            view! {
                <TableRow>
                    <TableCell class="fixed-checkbox-column">
                        <input type="checkbox" class="table__checkbox" ... />
                    </TableCell>
                    <TableCell>
                        <TableCellLayout>
                            {row.data}
                        </TableCellLayout>
                    </TableCell>
                </TableRow>
            }
        }).collect_view()}
    </TableBody>
</Table>
```

**Common violations (Medium severity - needs migration):**

- Native `<table>` with BEM classes (should migrate to Thaw Table)
- `<div class="table">` wrapper pattern (outdated, use Thaw Table)
- Missing `use thaw::*;` import when table exists

### Tree View Rules

**Rule:** Tree views need special handling for hierarchical data

**Check:**

```rust
// Must have this structure:
<div class="tree-container">
    <table class="table__data tree-table">
        <thead class="table__head">
            <tr>
                <th class="table__header-cell">...</th>
            </tr>
        </thead>
        <tbody>
            <tr class="table__row tree-row" data-depth="0">
                <td class="table__cell tree-cell">
                    <button class="tree-toggle">{icon("chevron-right")}</button>
                    {content}
                </td>
            </tr>
        </tbody>
    </table>
</div>
```

**Common violations:**

- Missing `tree-container` wrapper
- Not using `data-depth` attribute for hierarchy
- Inconsistent expand/collapse icons
- Manual padding instead of CSS-based indentation

### Filter Panel Rules

**Rule:** Collapsible filter panels with consistent structure

**Check:**

```rust
<div class="filter-panel">
    <div class="filter-panel-header">
        <div class="filter-panel-header__left">
            <button class="filter-panel-toggle" on:click=toggle>
                {icon("filter")}
                "Фильтры"
                {icon(if expanded { "chevron-up" } else { "chevron-down" })}
            </button>
        </div>
        <div class="filter-panel-header__right">
            <button class="filter-panel-clear" on:click=clear>
                {icon("x")} "Сбросить"
            </button>
        </div>
    </div>
    <Show when=move || expanded>
        <div class="filter-panel-content">
            // Thaw components or custom DateRangePicker
        </div>
    </Show>
</div>
```

**Common violations:**

- Wrong class names (`filters-panel`, `filter-container`)
- Missing collapsible behavior
- Not using `<Show>` component
- Inconsistent button classes

### Button Rules

**Rule:** ALWAYS use Thaw Button components (REQUIRED)

**Standard pattern:**

```rust
use thaw::*;

<Button
    appearance=ButtonAppearance::Primary
    on_click=move |_| handle_action()
>
    {icon("plus")}
    " Button Text"
</Button>

<Button
    appearance=ButtonAppearance::Secondary
    on_click=move |_| handle_action()
    disabled=Signal::derive(move || some_condition())
>
    {icon("refresh")}
    " Secondary Action"
</Button>
```

**Available appearances:**

- `ButtonAppearance::Primary` - Main action
- `ButtonAppearance::Secondary` - Secondary actions
- `ButtonAppearance::Outline` - Subtle actions
- `ButtonAppearance::Subtle` - Very subtle actions
- `ButtonAppearance::Transparent` - Transparent buttons

**Common violations (Medium severity - needs migration):**

- Native `<button class="button button--primary">` (should migrate to Thaw Button)
- Native `<button class="button button--secondary">` (should migrate to Thaw Button)
- Missing `use thaw::*;` import when buttons exist

### Import Rules

**Rule:** Proper imports for components and utilities

**Check:**

```rust
// Thaw components (REQUIRED for buttons and tables)
use thaw::*;

// Custom components
use crate::shared::components::date_range_picker::DateRangePicker;
use crate::shared::components::ui::badge::Badge as UiBadge;
use crate::shared::modal_stack::ModalStackService;

// Icons
use crate::shared::icons::icon;
```

**Common violations:**

- Missing `use thaw::*;` when using Thaw components (Critical if buttons/tables exist)
- Not importing custom components
- Using fully qualified paths instead of imports

## Reference Standards

**Primary documentation:** `memory-bank/architecture/frontend-page-standards.md`

**Related documentation:**

- `memory-bank/runbooks/RB-page-refactoring-to-bem-thaw-v1.md` - Step-by-step refactoring guide
- `memory-bank/templates/page-templates.md` - Ready-to-use templates
- `systemPatterns.md` - Architecture patterns

## Examples

### Example 1: Simple Audit (No Issues)

```
User: audit page a002_organization

AI:
## Pre-flight Check
- File exists: ✅
- Baseline linter errors: 0
- Page type: list
- Ready for audit: ✅

Analyzing crates/frontend/src/domain/a002_organization/ui/list/mod.rs...

## Audit Report: a002_organization

**File:** `crates/frontend/src/domain/a002_organization/ui/list/mod.rs`

### Summary
- Issues Found: 0
- Critical: 0 | High: 0 | Medium: 0 | Low: 0
- Status: ✅ COMPLIANT

This page follows all BEM + Thaw UI standards:
✅ Uses `class="page"` container
✅ Proper `page__header` BEM structure
✅ Table uses BEM classes
✅ Standard button classes
✅ Proper imports

No changes needed!
```

### Example 2: Critical Issue - Wrong Container

````
User: audit page a008_marketplace_sales

AI:
## Pre-flight Check
- File exists: ✅
- Baseline linter errors: 0
- Page type: list
- Ready for audit: ✅

Analyzing crates/frontend/src/domain/a008_marketplace_sales/ui/list/mod.rs...

## Audit Report: a008_marketplace_sales

**File:** `crates/frontend/src/domain/a008_marketplace_sales/ui/list/mod.rs`

### Summary
- Issues Found: 3
- Critical: 1 | High: 1 | Medium: 1 | Low: 0
- Estimated Effort: Moderate

### Issues by Severity

#### Issue 1: Wrong Container Class (Critical)
**Problem:** Uses `class="content"` instead of `class="page"`
**Location:** Line 123
**Severity:** Critical
**Impact:** Page doesn't expand to full width, layout broken

Current code:
```rust
<div class="content">
````

Fixed code:

```rust
<div class="page">
```

#### Issue 2: Header Structure Violation (High)

**Problem:** Uses `page-header` instead of `page__header` (BEM violation)
**Location:** Lines 125-135
**Severity:** High
**Impact:** Inconsistent with project standards, harder to maintain

Current code:

```rust
<div class="page-header">
    <div class="page-header__content">
        <h1 class="page-header__title">{"Sales"}</h1>
    </div>
</div>
```

Fixed code:

```rust
<div class="page__header">
    <div class="page__header-left">
        <h1 class="page__title">{"Sales"}</h1>
    </div>
    <div class="page__header-right">
        // Actions
    </div>
</div>
```

#### Issue 3: Missing Table Classes (Medium)

**Problem:** Plain `<table>` without BEM classes
**Location:** Lines 145-160
**Severity:** Medium
**Impact:** Inconsistent styling, harder to maintain

---

Found 3 issues. How would you like to proceed?
[AI uses AskQuestion tool with options: Preview All, Apply All, Interactive, Apply All Similar]

```

### Example 3: Tree View Page

```

User: audit page a003_counterparty

AI:

## Pre-flight Check

- File exists: ✅
- Baseline linter errors: 0
- Page type: tree
- Ready for audit: ✅

Analyzing crates/frontend/src/domain/a003_counterparty/ui/tree/widget.rs...

## Audit Report: a003_counterparty

**File:** `crates/frontend/src/domain/a003_counterparty/ui/tree/widget.rs`

### Summary

- Issues Found: 2
- Critical: 0 | High: 0 | Medium: 2 | Low: 0
- Estimated Effort: Simple

### Issues by Severity

#### Issue 1: Missing Tree Container (Medium)

**Problem:** Tree table not wrapped in `tree-container` div
**Location:** Line 89
**Severity:** Medium
**Impact:** Tree styling may not apply correctly

Current code:

```rust
<table class="table__data tree-table">
```

Fixed code:

```rust
<div class="tree-container">
    <table class="table__data tree-table">
```

#### Issue 2: Missing data-depth Attribute (Medium)

**Problem:** Tree rows don't have `data-depth` attribute for proper indentation
**Location:** Lines 95-110
**Severity:** Medium
**Impact:** Manual indentation instead of CSS-based hierarchy

```

### Example 4: Filter Panel Issues

```

User: audit page a012_wb_sales

AI:

## Pre-flight Check

- File exists: ✅
- Baseline linter errors: 0
- Page type: list
- Ready for audit: ✅

Analyzing crates/frontend/src/domain/a012_wb_sales/ui/list/mod.rs...

## Audit Report: a012_wb_sales

**File:** `crates/frontend/src/domain/a012_wb_sales/ui/list/mod.rs`

### Summary

- Issues Found: 2
- Critical: 0 | High: 1 | Medium: 1 | Low: 0
- Estimated Effort: Simple

### Issues by Severity

#### Issue 1: Wrong Filter Panel Class (High)

**Problem:** Uses `filters-panel` instead of `filter-panel`
**Location:** Line 145
**Severity:** High
**Impact:** CSS styles won't apply, inconsistent with standards

Current code:

```rust
<div class="filters-panel">
```

Fixed code:

```rust
<div class="filter-panel">
```

#### Issue 2: Missing Custom DateRangePicker Import (Medium)

**Problem:** Uses native inputs instead of custom DateRangePicker component
**Location:** Lines 160-165
**Severity:** Medium
**Impact:** Inconsistent UX, missing standard date picker features

Current code:

```rust
<input type="date" />
```

Fixed code:

```rust
use crate::shared::components::date_range_picker::DateRangePicker;

// In view:
<DateRangePicker
    date_from=date_from
    date_to=date_to
    on_date_from_change=move |val| set_date_from.set(val)
    on_date_to_change=move |val| set_date_to.set(val)
/>
```

### CSS Class Validation

**Standard classes found:** 15
**Custom classes:** 0
**Undefined classes:** 1

- `filters-panel` - Line 145 - Should be `filter-panel`

```

### Example 5: Migration from Native to Thaw Components

```

User: audit page a002_organization

AI:

## Pre-flight Check

- File exists: ✅
- Baseline linter errors: 0
- Page type: list
- Ready for audit: ✅

Analyzing crates/frontend/src/domain/a002_organization/ui/list/mod.rs...

## Audit Report: a002_organization

**File:** `crates/frontend/src/domain/a002_organization/ui/list/mod.rs`

### Summary

- Issues Found: 2
- Critical: 0 | High: 0 | Medium: 2 | Low: 0
- Estimated Effort: Moderate (requires component migration)

### Issues by Severity

#### Issue 1: Native Buttons Instead of Thaw (Medium)

**Problem:** Uses native `<button>` elements instead of Thaw `Button` component
**Location:** Lines 180-191
**Severity:** Medium
**Impact:** Not using Thaw UI system, inconsistent with modern component approach

Current code:

```rust
<button class="button button--primary" on:click=move |_| handle_create_new()>
    {icon("plus")}
    {"Новая организация"}
</button>
<button class="button button--secondary" on:click=move |_| fetch()>
    {icon("refresh")}
    {"Обновить"}
</button>
```

Fixed code:

```rust
use thaw::*;

<Button
    appearance=ButtonAppearance::Primary
    on_click=move |_| handle_create_new()
>
    {icon("plus")}
    " Новая организация"
</Button>
<Button
    appearance=ButtonAppearance::Secondary
    on_click=move |_| fetch()
>
    {icon("refresh")}
    " Обновить"
</Button>
```

#### Issue 2: Native Table Instead of Thaw (Medium)

**Problem:** Uses native `<table>` with BEM classes instead of Thaw `Table` component
**Location:** Lines 202-261
**Severity:** Medium
**Impact:** Not using Thaw UI table features (resizable columns, built-in styling)

Current code:

```rust
<div class="table">
    <table class="table__data table--striped">
        <thead class="table__head">
            <tr>
                <th class="table__header-cell">{"Код"}</th>
            </tr>
        </thead>
        <tbody>
            <tr class="table__row">
                <td class="table__cell">{row.code}</td>
            </tr>
        </tbody>
    </table>
</div>
```

Fixed code:

```rust
use thaw::*;

<Table>
    <TableHeader>
        <TableRow>
            <TableHeaderCell resizable=true min_width=100.0>
                "Код"
            </TableHeaderCell>
        </TableRow>
    </TableHeader>
    <TableBody>
        {move || items.get().into_iter().map(|row| {
            view! {
                <TableRow>
                    <TableCell>
                        <TableCellLayout>{row.code}</TableCellLayout>
                    </TableCell>
                </TableRow>
            }
        }).collect_view()}
    </TableBody>
</Table>
```

---

Would you like to proceed with migration?
[User selects: Apply All]

AI: Migrating buttons and table to Thaw components...
✅ Migrated 3 buttons to Thaw Button
✅ Migrated table to Thaw Table
✅ Added `use thaw::*;` import

Migration complete!

```

### Example 6: Batch Operations Workflow

```

User: audit page a005_marketplace

AI: [After analysis showing 8 issues: 2 Critical, 3 High, 2 Medium, 1 Low]

Found 8 issues. How would you like to proceed?

[User selects: "Apply All Similar"]

AI: Which type of issues would you like to apply?

- Container class (2 occurrences)
- Header structure (3 occurrences)
- Table classes (2 occurrences)
- Imports (1 occurrence)

[User selects: "Container class"]

AI: Applying all "container class" fixes...
✅ Fixed Line 45: content → page
✅ Fixed Line 234: content → page

Applied 2 fixes. 6 issues remaining.

Continue with next issue type?
[Continues with interactive mode for remaining issues]

```

### Example 7: Native Table Migration Required

```

User: audit page a015_wb_orders

AI:

## Pre-flight Check

- File exists: ✅
- Baseline linter errors: 0
- Page type: list
- Ready for audit: ✅

Analyzing crates/frontend/src/domain/a015_wb_orders/ui/list/mod.rs...

## Audit Report: a015_wb_orders

**File:** `crates/frontend/src/domain/a015_wb_orders/ui/list/mod.rs`

### Summary

- Issues Found: 8
- Critical: 0 | High: 2 | Medium: 6 | Low: 0
- Estimated Effort: Complex (requires Thaw Table migration)

### Issues by Severity

#### Issue 1: Native Table Instead of Thaw Table (High)

**Problem:** Uses native `<table>` elements instead of Thaw Table component system
**Location:** Lines 654-762
**Severity:** High
**Impact:** Missing Thaw Table features (resize, crosshair, styling consistency)

Current code:

```rust
<table class="table__data table--striped">
    <thead class="table__head">
        <tr>
            <th class="table__header-cell">...
```

Fixed code:

```rust
<Table attr:id=TABLE_ID attr:style="width: 100%; min-width: 1200px;">
    <TableHeader>
        <TableRow>
            <TableHeaderCell resizable=false min_width=120.0 class="resizable">...
```

Reference: See a012_wb_sales (lines 1141-1342) for complete Thaw Table example

#### Issue 2: Missing Checkbox Components (High)

**Problem:** No TableHeaderCheckbox/TableCellCheckbox for multi-select
**Location:** Table structure
**Severity:** High
**Impact:** Cannot select multiple items for batch operations

Fix: Add checkbox components:

```rust
<TableHeaderCheckbox
    items=items_signal
    selected=selected_signal
    get_id=Callback::new(|row: WbOrdersDto| row.id.clone())
    on_change=Callback::new(toggle_all)
/>

<TableCellCheckbox
    item_id=item.id.clone()
    selected=selected_signal
    on_change=Callback::new(toggle_select)
/>
```

#### Issue 3: Missing TableCellLayout (Medium)

**Problem:** Cell content not wrapped in TableCellLayout
**Location:** All table cells
**Severity:** Medium
**Impact:** Missing Thaw styling and truncation features

Fix: Wrap cell contents:

```rust
<TableCell>
    <TableCellLayout truncate=true>
        {order.document_no}
    </TableCellLayout>
</TableCell>
```

#### Issue 4: Missing Column Resize (Medium)

**Problem:** No init_column_resize() call
**Location:** Component initialization
**Severity:** Medium
**Impact:** Users cannot resize columns

Fix: Add resize initialization:

```rust
const TABLE_ID: &str = "a015-wb-orders-table";
const COLUMN_WIDTHS_KEY: &str = "a015_wb_orders_column_widths";

Effect::new(move |_| {
    spawn_local(async move {
        gloo_timers::future::TimeoutFuture::new(100).await;
        init_column_resize(TABLE_ID, COLUMN_WIDTHS_KEY);
    });
});
```

#### Issue 5: Missing TableCrosshairHighlight (Medium)

**Problem:** No crosshair highlight component
**Location:** Table wrapper
**Severity:** Medium
**Impact:** Missing visual cell highlight feature

Fix: Add before Table:

```rust
<div class="table-wrapper">
    <TableCrosshairHighlight table_id=TABLE_ID.to_string() />
    <Table ...>
```

#### Issue 6: UiButton Instead of Thaw Button (Medium)

**Problem:** Uses custom UiButton instead of Thaw Button
**Location:** Lines 490-520
**Severity:** Medium
**Impact:** Inconsistent with Thaw component system

Fix: Replace with Thaw Button:

```rust
<Button appearance=ButtonAppearance::Primary on:click=...>
    {"Create New"}
</Button>
```

#### Issue 7: Missing Table Components Import (Medium)

**Problem:** Missing imports for Thaw Table components
**Location:** Top of file
**Severity:** Medium
**Impact:** Cannot use Thaw Table features

Fix: Add imports:

```rust
use crate::shared::components::table::{
    TableCellCheckbox, TableHeaderCheckbox,
    TableCrosshairHighlight, TableCellMoney
};
use crate::shared::table_utils::init_column_resize;
```

#### Issue 8: Numbers Not Using TableCellMoney (Medium)

**Problem:** Monetary values displayed as plain text
**Location:** Lines 710-718 (spp, total_price columns)
**Severity:** Medium
**Impact:** Inconsistent number formatting

Fix: Use TableCellMoney:

```rust
<TableCellMoney
    value=order.spp
    show_currency=false
    color_by_sign=false
/>
```

### Recommended Approach

This requires a complete table migration. I recommend:

1. First migrate table structure (Issues 1, 7)
2. Add checkboxes for selection (Issue 2)
3. Add TableCellLayout to all cells (Issue 3)
4. Add column resize and crosshair (Issues 4, 5)
5. Replace buttons and format numbers (Issues 6, 8)

Would you like to proceed with the migration?
[Interactive mode with user confirmation for each step]

````

## List View Refactoring Checklist

This comprehensive checklist is based on successful refactorings of a015_wb_orders and a013_ym_order to match a012_wb_sales standards.

### Pre-Refactoring Analysis

**Check reference implementations:**
- ✅ a012_wb_sales (primary reference for list views)
- ✅ a015_wb_orders (refactored list with server-side pagination)
- ✅ a013_ym_order (refactored list with Thaw UI)

**Identify scope:**
- [ ] List-only refactor (UI + backend list API)
- [ ] Full refactor (includes details page)
- [ ] API response format: simple items/total OR full-paginated (items/total/page/page_size/total_pages)

### Backend Checklist

#### 1. Repository Layer (`repository.rs`)
- [ ] **List query struct** has all filter fields (date ranges, organization, search, status)
- [ ] **List row struct** includes all display fields + `organization_id` for enrichment
- [ ] **list_sql()** function:
  - [ ] Returns `(items: Vec<Row>, total: usize)`
  - [ ] Supports server-side sorting (sort_by, sort_desc parameters)
  - [ ] Supports server-side filtering (search, status, date range)
  - [ ] Implements pagination (limit, offset)
  - [ ] Includes `organization_id` in SELECT for enrichment
  - [ ] Uses parameterized queries (SQL injection safe)

#### 2. Handler Layer (`handlers/*.rs`)
- [ ] **Handler function**:
  - [ ] Accepts `Query<ListQueryParams>`
  - [ ] Returns `Json<PaginatedResponse>` with items/total/page/page_size/total_pages
  - [ ] Calculates `page = offset / page_size`
  - [ ] Calculates `total_pages = (total + page_size - 1) / page_size`
  - [ ] Loads organizations via `service::list_all()` for enrichment
  - [ ] Creates `HashMap<String, String>` for organization lookup
  - [ ] Maps repository rows to DTOs with `organization_name` enrichment
- [ ] **Query parameters**:
  - [ ] date_from, date_to (optional)
  - [ ] organization_id (optional)
  - [ ] search field (document_no, article, etc.)
  - [ ] status/filter field (optional)
  - [ ] sort_by, sort_desc (for sorting)
  - [ ] limit, offset (for pagination)

#### 3. Routes (`routes.rs`)
- [ ] Unified endpoint (e.g., both `/api/a015/wb-orders` and `/api/a015/wb-orders/list` use same handler)
- [ ] Legacy routes removed or redirected to new handler

#### 4. Contracts Layer (`contracts/src/domain/*/aggregate.rs`)
- [ ] **ListDto** includes:
  - [ ] All display fields
  - [ ] `organization_name: Option<String>` for enriched display
  - [ ] `is_posted: bool`, `is_error: bool` (if applicable)
  - [ ] `#[serde(default)]` on optional fields

### Frontend State Checklist (`state.rs`)

- [ ] **State struct** includes:
  - [ ] `orders: Vec<OrderDto>` (or equivalent)
  - [ ] `date_from: String`, `date_to: String`
  - [ ] `search_field: String` (order_no, article, etc.)
  - [ ] `filter_status: String` (or equivalent filter)
  - [ ] `selected_organization_id: Option<String>`
  - [ ] `sort_field: String`, `sort_ascending: bool`
  - [ ] `selected_ids: HashSet<String>` ⚠️ **HashSet not Vec!**
  - [ ] `page: usize`, `page_size: usize`
  - [ ] `total_count: usize`, `total_pages: usize`
  - [ ] `is_loaded: bool`

- [ ] **Default implementation**:
  - [ ] `selected_ids: HashSet::new()` ⚠️ **Not Vec::new()**
  - [ ] Reasonable defaults for sort_field, page_size (e.g., 50)
  - [ ] Empty strings for date/search fields

### Frontend UI Checklist (`mod.rs`)

#### 1. Imports
```rust
- [ ] use std::collections::HashSet; // For selection
- [ ] use thaw::*; // For all Thaw components
- [ ] use crate::shared::components::date_range_picker::DateRangePicker;
- [ ] use crate::shared::components::pagination_controls::PaginationControls;
- [ ] use crate::shared::components::table::{
        TableCellCheckbox, TableCellMoney,
        TableCrosshairHighlight, TableHeaderCheckbox,
    };
- [ ] use crate::shared::components::ui::badge::Badge as UiBadge;
- [ ] use crate::shared::icons::icon;
- [ ] use crate::shared::list_utils::{get_sort_class, get_sort_indicator, Sortable};
- [ ] use crate::shared::table_utils::init_column_resize;
````

**Remove old imports:**

- [ ] ❌ Remove `DateInput`, `MonthSelector` (replaced by DateRangePicker)
- [ ] ❌ Remove `format_number`, `format_number_int` (use TableCellMoney or keep minimal)
- [ ] ❌ Remove `AppGlobalContext` if not used

#### 2. Constants

```rust
- [ ] const TABLE_ID: &str = "your-table-id";
- [ ] const COLUMN_WIDTHS_KEY: &str = "your_page_column_widths";
```

#### 3. Component Structure

- [ ] Root container: `<div class="page">`
- [ ] Header: `<div class="page__header">` with `page__header-left` and `page__header-right`
- [ ] Filter panel: `<div class="filter-panel">` with proper BEM structure

#### 4. State Management & Signals

```rust
- [ ] state = create_state() // From state.rs
- [ ] (loading, set_loading) = signal(false)
- [ ] (error, set_error) = signal::<Option<String>>(None)
- [ ] (posting_in_progress, set_posting_in_progress) = signal(false)
- [ ] (save_notification, set_save_notification) = signal::<Option<String>>(None)
- [ ] (organizations, set_organizations) = signal::<Vec<Organization>>(Vec::new())
```

**Selection Management (for Thaw components):**

```rust
- [ ] selected = RwSignal::new(state.with_untracked(|s| s.selected_ids.clone()))
- [ ] toggle_selection = move |id: String, checked: bool| { ... }
- [ ] toggle_all = move |check_all: bool| { ... }
- [ ] items_signal = Signal::derive(move || state.get().orders)
- [ ] selected_signal = Signal::derive(move || selected.get())
```

**Filter Controls (RwSignals for Thaw components):**

```rust
- [ ] selected_org_id = RwSignal::new(...)
- [ ] Effect::new(move |_| { /* on change, update state and reload */ })
- [ ] search_order_no = RwSignal::new(...)
- [ ] Effect::new(move |_| { /* on change, reset page to 0 and reload */ })
- [ ] filter_status = RwSignal::new(...)
- [ ] Effect::new(move |_| { /* on change, reset page to 0 and reload */ })
```

**Column Resize Initialization:**

```rust
- [ ] Effect::new(move |_| { init_column_resize(TABLE_ID, COLUMN_WIDTHS_KEY); });
```

#### 5. Filter Panel Structure

```html
<div class="filter-panel">
  <div class="filter-panel-header">
    <div class="filter-panel-header__left">
      - [ ] Collapsible toggle with chevron icon - [ ] Active filters badge
    </div>
    <div class="filter-panel-header__center">
      - [ ] <PaginationControls ... /> component
    </div>
    <div class="filter-panel-header__right">
      - [ ] Batch action buttons (Post/Unpost)
    </div>
  </div>
  <Show when="move" || is_filter_expanded.get()>
    <div class="filter-panel-content">
      - [ ] Single-row filter layout using Thaw Flex - [ ] DateRangePicker
      component - [ ] Organization Select - [ ] Search Input - [ ] Status/Filter
      Select - [ ] "Обновить" Button (ButtonAppearance::Primary)
    </div>
  </Show>
</div>
```

#### 6. Table Structure (Thaw Components)

```html
<div class="table-wrapper">
    - [ ] <TableCrosshairHighlight table_id=TABLE_ID.to_string() />

    <Table attr:id=TABLE_ID attr:style="width: 100%; min-width: [value]px;">
        <TableHeader>
            <TableRow>
                - [ ] <TableHeaderCheckbox .../> for multi-select

                For each sortable column:
                <TableHeaderCell resizable=false min_width=[value] class="resizable">
                    <div class="table__sortable-header"
                         style="cursor: pointer;"
                         on:click=move |_| toggle_sort("field_name")>
                        "Column Name"
                        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "field_name"))>
                            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "field_name", state.with(|s| s.sort_ascending))}
                        </span>
                    </div>
                </TableHeaderCell>
            </TableRow>
        </TableHeader>

        <TableBody>
            <For
                each=move || state.get().orders
                key=|item| item.id.clone()
                children=move |order| {
                    view! {
                        <TableRow>
                            - [ ] <TableCellCheckbox .../> for row selection

                            For document number (clickable):
                            <TableCell>
                                <TableCellLayout truncate=true>
                                    <a href={...} class="table__link"
                                       style="color: #0f6cbd; text-decoration: underline;">
                                        {order.document_no}
                                    </a>
                                </TableCellLayout>
                            </TableCell>

                            For monetary values:
                            <TableCellMoney
                                value=order.amount
                                show_currency=false
                                color_by_sign=false
                            />

                            For regular text:
                            <TableCell>
                                <TableCellLayout truncate=true>
                                    {order.field}
                                </TableCellLayout>
                            </TableCell>
                        </TableRow>
                    }
                }
            />
        </TableBody>
    </Table>
</div>
```

#### 7. Sortable Headers Checklist

**Critical:** Each sortable column must have:

- [ ] `<div class="table__sortable-header">` wrapper
- [ ] `style="cursor: pointer;"` for visual feedback
- [ ] `on:click=move |_| toggle_sort("field_name")`
- [ ] `<span class=move || state.with(|s| get_sort_class(&s.sort_field, "field_name"))>`
- [ ] Inside span: `{move || get_sort_indicator(...)}` for triangle display

**Why this is important:**

- `get_sort_class()` returns `"sort-icon active"` for active sort, making it green
- Without CSS class, the sort indicator stays gray (bug reported in a015_wb_orders)

#### 8. Data Loading (`load_orders`)

- [ ] URL construction includes all parameters:
  - [ ] page, page_size (from state)
  - [ ] sort_field, sort_ascending (map to sort_by, sort_desc)
  - [ ] search_order_no, filter_status
  - [ ] date_from, date_to
  - [ ] organization_id
- [ ] Deserialize into `PaginatedResponse`
- [ ] Update state with: `items, total, page, page_size, total_pages`
- [ ] Update `selected` signal to match `state.selected_ids`
- [ ] Error handling with set_error

#### 9. Pagination Controls

- [ ] `<PaginationControls>` component in filter-panel-header\_\_center
- [ ] Props:
  - [ ] `current_page=Signal::derive(move || state.get().page)`
  - [ ] `total_pages=Signal::derive(move || state.get().total_pages)`
  - [ ] `total_count=Signal::derive(move || state.get().total_count)`
  - [ ] `page_size=Signal::derive(move || state.get().page_size)`
  - [ ] `on_page_change=Callback::new(go_to_page)`
  - [ ] `on_page_size_change=Callback::new(change_page_size)`
  - [ ] `page_size_options=vec![50, 100, 200, 500]`

#### 10. Batch Operations

```rust
- [ ] Post button in filter-panel-header__right
      <Button appearance=ButtonAppearance::Primary
              disabled=Signal::derive(move || state.get().selected_ids.is_empty() || posting_in_progress.get())
              on_click=move |_| { /* batch post logic */ }>
          {move || format!("✓ Post ({})", state.get().selected_ids.len())}
      </Button>

- [ ] Unpost button
      <Button appearance=ButtonAppearance::Secondary ...>

- [ ] Batch operations:
      - Call /api/.../batch-post or /api/.../batch-unpost
      - Pass {"ids": [selected_ids]} as JSON
      - Clear selection after success
      - Reload list
```

### Code Cleanup Checklist

**Remove old/duplicate code:**

- [ ] ❌ Remove old pagination HTML (native buttons, page info)
- [ ] ❌ Remove old totals display (if not needed)
- [ ] ❌ Remove old selection summary panel
- [ ] ❌ Remove duplicate helper functions (load_saved_settings, fetch_organizations, etc.)
- [ ] ❌ Remove unused signals/closures (get_items, totals, all_selected, is_selected, etc.)
- [ ] ❌ Remove old toggle_selection, toggle_all if replaced by new versions
- [ ] ❌ Remove old post_selected, unpost_selected if replaced

**Remove unused imports:**

- [ ] ❌ Remove `std::collections::HashSet` if not actually used
- [ ] ❌ Remove `AppGlobalContext` if not used

### Testing & Verification Checklist

**Compilation:**

- [ ] `cargo check` passes with zero errors
- [ ] No new warnings introduced
- [ ] Frontend compiles successfully

**UI Functionality:**

- [ ] Table displays data correctly
- [ ] Sorting works (click headers, green indicator shows)
- [ ] Pagination works (page navigation, page size change)
- [ ] Filtering works (date range, organization, search, status)
- [ ] Selection works (checkbox, select all)
- [ ] Batch operations work (Post/Unpost)
- [ ] Column resizing works
- [ ] Crosshair highlight works on hover
- [ ] Document numbers are clickable links
- [ ] Money cells are right-aligned and formatted

**Visual Checks:**

- [ ] Sort indicator turns GREEN for active column ⚠️ **Common bug!**
- [ ] Cursor changes to pointer on sortable headers
- [ ] Filter panel is single-row (compact)
- [ ] No background highlight on selected rows (only checkbox)
- [ ] Pagination controls visible and functional

### Common Pitfalls & Solutions

**1. Sort indicator not turning green**

- ❌ Problem: Missing `get_sort_class` import or not using CSS class on span
- ✅ Solution: Import `get_sort_class` and use `class=move || state.with(|s| get_sort_class(...))`

**2. "HashSet not found" error**

- ❌ Problem: Forgot to import `std::collections::HashSet`
- ✅ Solution: Add `use std::collections::HashSet;` at top

**3. "AppGlobalContext not found" error**

- ❌ Problem: Removed usage but still referenced in component
- ✅ Solution: Remove `let tabs_store = use_context::<AppGlobalContext>()` line

**4. Selection not working with Thaw components**

- ❌ Problem: Using Vec<String> instead of HashSet<String>
- ✅ Solution: Change state to use HashSet, update toggle functions

**5. Pagination not updating after filter change**

- ❌ Problem: Not resetting page to 0 when filters change
- ✅ Solution: In filter Effect, do `state.update(|s| s.page = 0);` before load_orders()

**6. Duplicate helper functions causing errors**

- ❌ Problem: Old code not fully removed after refactor
- ✅ Solution: Search for duplicate `async fn` definitions and remove old versions

**7. Column resize not working**

- ❌ Problem: Missing init_column_resize call or wrong timing
- ✅ Solution: Call in Effect::new after component mount

**8. Money cells not right-aligned**

- ❌ Problem: Not using TableCellMoney component
- ✅ Solution: Replace `<TableCell>{amount}</TableCell>` with `<TableCellMoney value=amount .../>`

### Migration Timeline (Typical)

**Phase 1: Backend (1-2 hours)**

1. Update repository for pagination/filtering
2. Update handler for paginated response
3. Unify routes
4. Test API endpoints

**Phase 2: Frontend State (30 min)**

1. Update state.rs (HashSet, pagination fields)
2. Test compilation

**Phase 3: Frontend UI (3-4 hours)**

1. Update imports
2. Add constants
3. Update state management and signals
4. Migrate filter panel
5. Migrate table to Thaw components
6. Add checkboxes and selection
7. Update data loading
8. Test each feature incrementally

**Phase 4: Cleanup & Testing (1 hour)**

1. Remove old code
2. Fix compilation errors
3. Test all functionality
4. Verify visual appearance

## Implementation Notes

### Critical Rules

- **Always use AskQuestion tool** for confirmation (not text prompts)
- **Start with Pre-flight Check** before analysis
- **Group issues by severity** in reports (Critical first)
- **Offer batch operations** for efficiency
- **Validate CSS classes** after fixes
- **Compare linter results** with baseline
- **Check sort indicator CSS class** - common bug!

### Refactoring Guidelines

- **Keep original functionality** - only change structure/classes, not logic
- **Preserve comments and formatting** where possible
- **Run ReadLints** after changes to catch issues early
- **Test incrementally** - apply high-severity fixes first

### Batch Operation Strategy

- For pages with 5+ issues: offer batch mode first
- For similar issues (same type): suggest "Apply All Similar"
- For low-risk fixes: suggest "Apply All"
- For complex fixes: default to interactive mode

### CSS Validation Strategy

- Check all classes against `layout.css` and `components.css`
- Warn about custom classes (may be intentional)
- Flag undefined classes as potential typos
- Run validation after fixes to catch introduced errors

## Success Criteria

A page passes audit when:

1. ✅ All checklist items pass
2. ✅ No new linter errors introduced (compare with baseline)
3. ✅ All CSS classes validated
4. ✅ Layout works correctly in browser
5. ✅ Code matches reference implementations
6. ✅ Proper imports and component usage

## Quick Reference

### Issue Type Categories (for batch operations)

- **Container**: `content` → `page` class changes
- **Header**: `page-header` → `page__header` BEM fixes
- **Button Migration**: Native `<button>` → Thaw `Button` (Medium severity)
- **Table Migration**: Native `<table>` → Thaw `Table` (Medium severity)
- **Filter Panel**: Filter panel structure fixes
- **Imports**: Missing Thaw or custom component imports
- **Components**: Native elements → Thaw components
- **Tree**: Tree-specific structure fixes

### Severity Decision Tree

```
Does it break layout? → Critical
Does it violate BEM? → High
Uses native buttons/tables instead of Thaw? → Medium
Does it use wrong components? → Medium
Is it a style improvement? → Low
```

### Migration Priority

When migrating components to Thaw:

1. **Buttons first** - Simplest migration, immediate visual consistency
2. **Tables second** - More complex but brings powerful features (resizable columns, sorting)
3. **Other components** - Inputs, selects, etc. as needed

### Thaw Component Benefits

- **Buttons**: Consistent styling, built-in states (disabled, loading), better accessibility
- **Tables**: Resizable columns, built-in theming, better performance, sticky headers
- **Forms**: Validation support, consistent error handling, better UX

## Quick Reference Card: List Refactoring

### Essential Imports (Copy-Paste Ready)

```rust
use std::collections::HashSet;
use thaw::*;
use crate::shared::components::date_range_picker::DateRangePicker;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::table::{
    TableCellCheckbox, TableCellMoney, TableCrosshairHighlight, TableHeaderCheckbox,
};
use crate::shared::components::ui::badge::Badge as UiBadge;
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator, Sortable};
use crate::shared::table_utils::init_column_resize;

const TABLE_ID: &str = "your-table-id";
const COLUMN_WIDTHS_KEY: &str = "your_table_column_widths";
```

### State.rs Template

```rust
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct YourState {
    pub items: Vec<YourDto>,
    pub date_from: String,
    pub date_to: String,
    pub search_field: String,
    pub filter_field: String,
    pub selected_organization_id: Option<String>,
    pub sort_field: String,
    pub sort_ascending: bool,
    pub selected_ids: HashSet<String>, // ⚠️ HashSet not Vec!
    pub is_loaded: bool,
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
}

impl Default for YourState {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            date_from: String::new(),
            date_to: String::new(),
            search_field: String::new(),
            filter_field: String::new(),
            selected_organization_id: None,
            sort_field: "date_field".to_string(),
            sort_ascending: false,
            selected_ids: HashSet::new(), // ⚠️ HashSet not Vec!
            is_loaded: false,
            page: 0,
            page_size: 50,
            total_count: 0,
            total_pages: 0,
        }
    }
}
```

### Sortable Header Pattern (Copy-Paste)

```rust
<TableHeaderCell resizable=false min_width=140.0 class="resizable">
    <div class="table__sortable-header" style="cursor: pointer;"
         on:click=move |_| toggle_sort("field_name")>
        "Column Name"
        <span class=move || state.with(|s| get_sort_class(&s.sort_field, "field_name"))>
            {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "field_name", state.with(|s| s.sort_ascending))}
        </span>
    </div>
</TableHeaderCell>
```

### Selection Management Pattern

```rust
// Signals for Thaw components
let selected = RwSignal::new(state.with_untracked(|s| s.selected_ids.clone()));

let toggle_selection = move |id: String, checked: bool| {
    selected.update(|s| {
        if checked { s.insert(id.clone()); } else { s.remove(&id); }
    });
    state.update(|s| {
        if checked { s.selected_ids.insert(id); } else { s.selected_ids.remove(&id); }
    });
};

let toggle_all = move |check_all: bool| {
    if check_all {
        let items = state.get().items;
        selected.update(|s| { s.clear(); for item in items.iter() { s.insert(item.id.clone()); } });
        state.update(|s| { s.selected_ids.clear(); for item in items.iter() { s.selected_ids.insert(item.id.clone()); } });
    } else {
        selected.update(|s| s.clear());
        state.update(|s| s.selected_ids.clear());
    }
};

let items_signal = Signal::derive(move || state.get().items);
let selected_signal = Signal::derive(move || selected.get());
```

### Filter RwSignal Pattern (with auto-reload)

```rust
let selected_org_id = RwSignal::new(state.with_untracked(|s|
    s.selected_organization_id.clone().unwrap_or_default()
));

Effect::new(move |_| {
    let val = selected_org_id.get();
    state.update(|s| {
        if val.is_empty() { s.selected_organization_id = None; }
        else { s.selected_organization_id = Some(val.clone()); }
        s.page = 0; // ⚠️ Reset page when filter changes!
    });
    load_data();
});
```

### Top 5 Bugs to Avoid

1. **Sort indicator not green** → Missing `get_sort_class` or not using CSS class
2. **HashSet error** → Using `Vec<String>` instead of `HashSet<String>` for selected_ids
3. **Page not resetting** → Not setting `s.page = 0` when filters change
4. **Duplicate functions** → Not removing old helper functions after refactor
5. **Money not aligned** → Not using `<TableCellMoney>` component

### Backend Response Format

```rust
#[derive(Debug, Serialize)]
pub struct PaginatedResponse {
    pub items: Vec<YourListDto>,
    pub total: usize,
    pub page: usize,          // ⚠️ Required!
    pub page_size: usize,     // ⚠️ Required!
    pub total_pages: usize,   // ⚠️ Required!
}

// Calculate in handler:
let page = if page_size > 0 { offset / page_size } else { 0 };
let total_pages = if page_size > 0 { (total + page_size - 1) / page_size } else { 0 };
```

### Reference Pages Priority

1. **a012_wb_sales** - Primary reference (most complete)
2. **a015_wb_orders** - Server-side pagination, single-row filters
3. **a013_ym_order** - Clean Thaw UI implementation
4. **a002_organization** - Simple list (if basic example needed)
