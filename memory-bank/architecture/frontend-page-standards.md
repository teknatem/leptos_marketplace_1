# Frontend Page Standards (BEM + Thaw UI)

_Last updated: 2026-01-30_

This document defines the standard structure and patterns for frontend pages in the Leptos marketplace application. All pages should follow these conventions for consistency, maintainability, and proper layout behavior.

## Core Principles

1. **BEM Methodology** - Block Element Modifier naming for CSS classes
2. **Thaw UI First** - Prefer Thaw components over native HTML elements
3. **Full Width Layout** - Pages should occupy 100% of available space
4. **Consistent Structure** - All pages follow the same hierarchy

## Container Structure

### Root Container

**ALWAYS use:**
```rust
<div class="page">
    // Page content
</div>
```

**CSS Definition:**
```css
.page {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  scrollbar-width: thin;
  scrollbar-color: var(--list-scrollbar-thumb) var(--list-scrollbar-track);
}
```

**NEVER use:**
- `class="content"` (old pattern)
- Inline `style="padding: 20px;"` on root
- Missing width/height specifications

## Header Pattern (BEM)

### Standard Header Structure

```rust
<div class="page__header">
    <div class="page__header-left">
        <h1 class="page__title">{"Page Title"}</h1>
        // Optional: badges, icons, subtitles
    </div>
    <div class="page__header-right">
        // Action buttons
    </div>
</div>
```

### CSS Classes Available

- `.page__header` - Sticky header with shadow and border
- `.page__header-left` - Left section with title (flex: 1)
- `.page__header-right` - Right section with actions (flex-shrink: 0)
- `.page__title` - H1 title styling
- `.page__icon` - Optional icon before title

### Example (Good)

```rust
<div class="page__header">
    <div class="page__header-left">
        <h1 class="page__title">{"Организации"}</h1>
    </div>
    <div class="page__header-right">
        <button class="button button--primary" on:click=create_new>
            {icon("plus")}
            {"Новая организация"}
        </button>
        <button class="button button--secondary" on:click=refresh>
            {icon("refresh")}
            {"Обновить"}
        </button>
    </div>
</div>
```

### Anti-Patterns (Bad)

```rust
// ❌ Wrong class name (not BEM)
<div class="page-header">
    <div class="page-header__content">
        <div class="page-header__icon">...</div>
        <div class="page-header__text">
            <h1 class="page-header__title">...</h1>
        </div>
    </div>
    <div class="page-header__actions">...</div>
</div>
```

**Issues:**
- Uses `page-header` instead of `page__header`
- Overcomplicated nested structure
- Non-standard element names

## Table Patterns

### Choice: Thaw Table vs Native Table

**Use Thaw Table when:**
- Need complex features (sorting, filtering, pagination)
- Want automatic theme integration
- Have resizable columns

**Use Native Table with BEM when:**
- Simple data display
- Need full control over styling
- Custom interaction patterns

### Pattern 1: Thaw Table (Complex)

```rust
use thaw::*;

<Table>
    <TableHeader>
        <TableRow>
            <TableHeaderCell resizable=false class="fixed-checkbox-column">
                <input type="checkbox" ... />
            </TableHeaderCell>
            <TableHeaderCell 
                on:click=sort_handler 
                class=get_sort_class("code", sort_field, sort_ascending)
            >
                {"Код"}
                {get_sort_indicator("code", sort_field, sort_ascending)}
            </TableHeaderCell>
            // More columns...
        </TableRow>
    </TableHeader>
    <TableBody>
        {move || items.get().into_iter().map(|row| {
            view! {
                <TableRow>
                    <TableCell>...</TableCell>
                </TableRow>
            }
        }).collect_view()}
    </TableBody>
</Table>
```

### Pattern 2: Native Table with BEM (Simple)

```rust
<div class="table">
    <table class="table__data table--striped">
        <thead class="table__head">
            <tr>
                <th class="table__header-cell table__header-cell--checkbox">
                    <input type="checkbox" class="table__checkbox" ... />
                </th>
                <th class="table__header-cell">{"Код"}</th>
                <th class="table__header-cell">{"Наименование"}</th>
            </tr>
        </thead>
        <tbody>
            {move || items.get().into_iter().map(|row| {
                view! {
                    <tr class="table__row" class:table__row--selected=is_selected>
                        <td class="table__cell table__cell--checkbox">
                            <input type="checkbox" class="table__checkbox" ... />
                        </td>
                        <td class="table__cell">{row.code}</td>
                        <td class="table__cell">{row.description}</td>
                    </tr>
                }
            }).collect_view()}
        </tbody>
    </table>
</div>
```

**BEM Classes for Tables:**
- `.table` - Table container wrapper
- `.table__data` - The `<table>` element
- `.table--striped` - Modifier for striped rows
- `.table__head` - `<thead>` element
- `.table__header-cell` - `<th>` elements
- `.table__header-cell--checkbox` - Modifier for checkbox column
- `.table__row` - `<tr>` elements
- `.table__row--selected` - Modifier for selected rows
- `.table__cell` - `<td>` elements
- `.table__cell--checkbox` - Modifier for checkbox cells
- `.table__checkbox` - Checkbox input styling

## Filter Panel Pattern (Optional)

For pages with filters (date ranges, dropdowns, search):

```rust
<div class="filter-panel">
    <div class="filter-panel-header">
        <div class="filter-panel-header__left">
            <button on:click=toggle_filters>
                {icon("filter")}
                {"Фильтры"}
                {icon(if expanded { "chevron-up" } else { "chevron-down" })}
            </button>
        </div>
        <div class="filter-panel-header__center">
            // Optional: quick filters or badges
        </div>
        <div class="filter-panel-header__right">
            <button on:click=clear_filters>
                {icon("x")}
                {"Сбросить"}
            </button>
        </div>
    </div>
    <Show when=move || expanded>
        <div class="filter-panel-content">
            // Filter controls using Thaw components
        </div>
    </Show>
</div>
```

## Component Library Usage

### Thaw UI Components (Preferred)

**Buttons:**
```rust
use thaw::*;

<Button 
    appearance=ButtonAppearance::Primary
    on_click=handler
>
    {icon("plus")}
    " Создать"
</Button>
```

**Inputs:**
```rust
<Input 
    value=signal
    on_input=move |val| set_value.set(val)
    placeholder="Введите текст"
/>
```

**Select/Dropdowns:**
```rust
<Select 
    value=selected
    on_change=move |val| set_selected.set(val)
>
    <SelectOption value="1">{"Вариант 1"}</SelectOption>
    <SelectOption value="2">{"Вариант 2"}</SelectOption>
</Select>
```

**Layout:**
```rust
<Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center>
    <h1>{"Title"}</h1>
    <Space>
        <Button>{"Action 1"}</Button>
        <Button>{"Action 2"}</Button>
    </Space>
</Flex>
```

### Custom UI Components

From `crates/frontend/src/shared/components/ui/`:

- `Badge` - Status indicators
- `Button` - Enhanced buttons with variants
- `DateRangePicker` - Date range selection
- `PaginationControls` - Table pagination

### Native Elements (When Appropriate)

Use native elements with BEM classes when:
- Thaw doesn't provide the component
- Need specific styling control
- Simple semantic HTML is clearer

```rust
<button class="button button--primary">{"Action"}</button>
<button class="button button--secondary">{"Action"}</button>
<button class="button button--ghost">{"Action"}</button>
```

## Complete Page Templates

### Template 1: Simple List View

**File:** `crates/frontend/src/domain/aXXX_entity/ui/list/mod.rs`

```rust
use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use leptos::prelude::*;
use std::collections::HashSet;

#[component]
pub fn EntityList() -> impl IntoView {
    let ctx = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let (items, set_items) = signal::<Vec<Entity>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);
    let (selected, set_selected) = signal::<HashSet<String>>(HashSet::new());

    let fetch = move || {
        // Fetch logic
    };

    let handle_create_new = move || {
        ctx.open_tab("aXXX_entity_new", "Новая сущность");
    };

    fetch();

    view! {
        <div class="page">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">{"Сущности"}</h1>
                </div>
                <div class="page__header-right">
                    <button class="button button--primary" on:click=move |_| handle_create_new()>
                        {icon("plus")}
                        {"Новая сущность"}
                    </button>
                    <button class="button button--secondary" on:click=move |_| fetch()>
                        {icon("refresh")}
                        {"Обновить"}
                    </button>
                </div>
            </div>

            <div class="table">
                <table class="table__data table--striped">
                    <thead class="table__head">
                        <tr>
                            <th class="table__header-cell">{"Код"}</th>
                            <th class="table__header-cell">{"Наименование"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || items.get().into_iter().map(|row| {
                            view! {
                                <tr class="table__row">
                                    <td class="table__cell">{row.code}</td>
                                    <td class="table__cell">{row.description}</td>
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table>
            </div>
        </div>
    }
}
```

### Template 2: Complex List with Filters

**File:** `crates/frontend/src/domain/aXXX_entity/ui/list/mod.rs`

```rust
use thaw::*;
use crate::shared::components::date_range_picker::DateRangePicker;
use crate::shared::components::ui::badge::Badge as UiBadge;
use crate::shared::components::ui::button::Button as UiButton;

#[component]
pub fn EntityList() -> impl IntoView {
    let (is_filter_expanded, set_is_filter_expanded) = signal(false);
    let (date_from, set_date_from) = signal(String::new());
    let (date_to, set_date_to) = signal(String::new());
    
    view! {
        <div class="page">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">{"Сущности"}</h1>
                    <UiBadge variant="primary".to_string()>
                        {move || total_count().to_string()}
                    </UiBadge>
                </div>
                <div class="page__header-right">
                    <Space>
                        <UiButton variant="primary".to_string() on_click=Callback::new(create_new)>
                            {icon("plus")}
                            "Создать"
                        </UiButton>
                    </Space>
                </div>
            </div>

            <div class="filter-panel">
                <div class="filter-panel-header">
                    <div class="filter-panel-header__left">
                        <button 
                            class="filter-panel-toggle"
                            on:click=move |_| set_is_filter_expanded.update(|e| *e = !*e)
                        >
                            {icon("filter")}
                            "Фильтры"
                            {icon(if is_filter_expanded.get() { "chevron-up" } else { "chevron-down" })}
                        </button>
                    </div>
                    <div class="filter-panel-header__right">
                        <button class="filter-panel-clear" on:click=clear_filters>
                            {icon("x")}
                            "Сбросить"
                        </button>
                    </div>
                </div>
                
                <Show when=move || is_filter_expanded.get()>
                    <div class="filter-panel-content">
                        <Flex gap=FlexGap::Size(16.0) wrap=FlexWrap::Wrap>
                            <DateRangePicker 
                                date_from=date_from 
                                date_to=date_to
                                on_date_from_change=move |val| set_date_from.set(val)
                                on_date_to_change=move |val| set_date_to.set(val)
                            />
                            // More filters...
                        </Flex>
                    </div>
                </Show>
            </div>

            <Table>
                <TableHeader>
                    <TableRow>
                        <TableHeaderCell>{"Код"}</TableHeaderCell>
                        <TableHeaderCell>{"Наименование"}</TableHeaderCell>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {move || items.get().into_iter().map(|row| {
                        view! {
                            <TableRow>
                                <TableCell><TableCellLayout>{row.code}</TableCellLayout></TableCell>
                                <TableCell><TableCellLayout>{row.description}</TableCellLayout></TableCell>
                            </TableRow>
                        }
                    }).collect_view()}
                </TableBody>
            </Table>
        </div>
    }
}
```

## Common Mistakes and Fixes

### Mistake 1: Wrong Container Class

**Bad:**
```rust
<div class="content">  // ❌
```

**Good:**
```rust
<div class="page">  // ✅
```

### Mistake 2: Non-BEM Header

**Bad:**
```rust
<div class="page-header">  // ❌ Hyphen instead of double underscore
    <div class="page-header__content">
        <div class="page-header__icon">...</div>
        <div class="page-header__text">
            <h1 class="page-header__title">...</h1>
        </div>
    </div>
</div>
```

**Good:**
```rust
<div class="page__header">  // ✅ Proper BEM
    <div class="page__header-left">
        <h1 class="page__title">...</h1>
    </div>
    <div class="page__header-right">...</div>
</div>
```

### Mistake 3: Plain Tables Without Classes

**Bad:**
```rust
<table>  // ❌ No classes
    <thead>
        <tr>
            <th>Code</th>
        </tr>
    </thead>
</table>
```

**Good:**
```rust
<div class="table">
    <table class="table__data table--striped">  // ✅ BEM classes
        <thead class="table__head">
            <tr>
                <th class="table__header-cell">{"Код"}</th>
            </tr>
        </thead>
    </table>
</div>
```

### Mistake 4: Inline Styles Instead of Thaw

**Bad:**
```rust
<div style="padding: 20px;">  // ❌ Inline styles
    <h1 style="font-size: 24px; font-weight: bold;">{"Title"}</h1>
    <button style="background: blue;">{"Action"}</button>
</div>
```

**Good:**
```rust
<div class="page">  // ✅ CSS classes
    <div class="page__header">
        <div class="page__header-left">
            <h1 class="page__title">{"Title"}</h1>
        </div>
        <div class="page__header-right">
            <Button appearance=ButtonAppearance::Primary>
                {"Action"}
            </Button>
        </div>
    </div>
</div>
```

## Layout Hierarchy

```
.page (100% width/height, flex column)
├── .page__header (sticky, shadow)
│   ├── .page__header-left
│   │   └── .page__title (h1)
│   └── .page__header-right
│       └── buttons / actions
├── .filter-panel (optional, collapsible)
│   ├── .filter-panel-header
│   └── .filter-panel-content
└── .table OR Table (Thaw)
    └── table content
```

## CSS Variables Reference

Available CSS variables for theming:

**Colors:**
- `--color-text-primary` - Primary text color
- `--color-text-secondary` - Secondary text color
- `--color-text-muted` - Muted/disabled text
- `--color-primary` - Brand primary color
- `--color-surface` - Surface background
- `--color-border` - Border color
- `--color-hover` - Hover background

**Spacing:**
- `--spacing-xs` - 4px
- `--spacing-sm` - 8px
- `--spacing-md` - 12px
- `--spacing-lg` - 16px
- `--spacing-xl` - 24px
- `--spacing-2xl` - 40px

**Sidebar:**
- `--sidebar-item-hover` - Sidebar item hover background
- `--sidebar-item-active-bg` - Active item background
- `--sidebar-item-active-border` - Active item border color
- `--sidebar-text-active` - Active text color

## State Management Patterns

### Pattern: Separate state.rs

For complex pages, extract state into separate file:

**File:** `crates/frontend/src/domain/aXXX_entity/ui/list/state.rs`

```rust
use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct PageState {
    pub items: Vec<Entity>,
    pub selected_ids: HashSet<String>,
    pub sort_field: String,
    pub sort_ascending: bool,
    pub page: usize,
    pub page_size: usize,
    // ... more fields
}

pub fn create_state() -> RwSignal<PageState> {
    RwSignal::new(PageState {
        items: Vec::new(),
        selected_ids: HashSet::new(),
        sort_field: "code".to_string(),
        sort_ascending: true,
        page: 0,
        page_size: 50,
    })
}
```

## Reference Implementations

### Good Examples (Follow These)

1. **a002_organization** - Simple list with BEM tables
   - File: `crates/frontend/src/domain/a002_organization/ui/list/mod.rs`
   - Pattern: page → page__header → table with BEM classes

2. **a012_wb_sales** - Complex list with filters and Thaw Table
   - File: `crates/frontend/src/domain/a012_wb_sales/ui/list/mod.rs`
   - Pattern: page → page__header → filter-panel → Thaw Table

3. **a006_connection_mp** - Thaw Table with sorting
   - File: `crates/frontend/src/domain/a006_connection_mp/ui/list/mod.rs`
   - Pattern: Thaw components throughout

### Bad Examples (Avoid These)

Pages needing refactoring:
- a016_ym_returns - Uses `page-header` instead of `page__header`
- a008_marketplace_sales - Uses `content` instead of `page`
- a005_marketplace - Uses `content` instead of `page`

## Checklist for New Pages

Before submitting a new page, verify:

- [ ] Root container is `class="page"`
- [ ] Header uses `page__header` with BEM children
- [ ] Title is `<h1 class="page__title">`
- [ ] Tables use either Thaw Table OR BEM table classes
- [ ] No plain `<table>` without classes
- [ ] Buttons use Thaw or standard `.button` classes
- [ ] No excessive inline styles
- [ ] Filter panel follows BEM structure (if present)
- [ ] All CSS classes exist in layout.css or components.css

## Migration Guide

### Step-by-Step Refactoring

1. **Identify page type** (list/tree/details)
2. **Fix container:** `content` → `page`
3. **Fix header:** `page-header` → `page__header`
4. **Update header children** to BEM structure
5. **Standardize tables** (Thaw or BEM classes)
6. **Replace native elements** with Thaw components
7. **Test in browser** to verify layout
8. **Run linter** to check for issues

### Before/After Example

**Before:**
```rust
view! {
    <div class="content">
        <div class="page-header">
            <div class="page-header__content">
                <h1 class="page-header__title">{"Title"}</h1>
            </div>
        </div>
        <table>
            <tr><td>Data</td></tr>
        </table>
    </div>
}
```

**After:**
```rust
view! {
    <div class="page">
        <div class="page__header">
            <div class="page__header-left">
                <h1 class="page__title">{"Title"}</h1>
            </div>
            <div class="page__header-right">
                // Actions
            </div>
        </div>
        <div class="table">
            <table class="table__data table--striped">
                <thead class="table__head">
                    <tr>
                        <th class="table__header-cell">{"Колонка"}</th>
                    </tr>
                </thead>
                <tbody>
                    <tr class="table__row">
                        <td class="table__cell">{"Data"}</td>
                    </tr>
                </tbody>
            </table>
        </div>
    </div>
}
```

## Related Documentation

- `systemPatterns.md` - Overall architecture patterns
- `memory-bank/runbooks/RB-thaw-ui-migration-v1.md` - Thaw UI migration guide
- `memory-bank/runbooks/RB-thaw-table-sorting-v1.md` - Table sorting patterns
- `.cursor/skills/audit-page-bem-thaw/SKILL.md` - Automated audit skill
