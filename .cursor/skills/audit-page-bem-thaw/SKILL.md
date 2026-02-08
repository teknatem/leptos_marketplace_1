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
- [ ] Title uses `<h1 class="page__title">` (NOT "page-header__title")
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

```markdown
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

Fixed code:
```rust
[corrected code snippet]
```

[Repeat for each issue, grouped by severity: Critical first, then High, Medium, Low]
```

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
```

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

```
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
```

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

## Implementation Notes

### Critical Rules
- **Always use AskQuestion tool** for confirmation (not text prompts)
- **Start with Pre-flight Check** before analysis
- **Group issues by severity** in reports (Critical first)
- **Offer batch operations** for efficiency
- **Validate CSS classes** after fixes
- **Compare linter results** with baseline

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
