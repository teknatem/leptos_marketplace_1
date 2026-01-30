---
date: 2025-01-29
tags: [lesson, thaw, ui, leptos, button]
category: frontend
---

# Lesson: Discovering Thaw UI ButtonAppearance Variants

## Context

When migrating from custom HTML buttons to Thaw UI Button components, attempted to use `ButtonAppearance::Outline` which doesn't exist in the library, causing compilation errors.

## Problem

Thaw UI library documentation was not immediately accessible, and trying to use non-existent API variants led to cryptic compilation errors.

## Solution: Grep Existing Usage

Instead of searching external documentation, used Grep to find all existing uses of `ButtonAppearance::` in the codebase:

```bash
rg "ButtonAppearance::" crates/frontend
```

## Available ButtonAppearance Variants (Thaw UI)

As of 2025-01-29, the available variants are:

1. **Primary** - Main action button (blue/brand color)
2. **Secondary** - Secondary action (gray border)
3. **Subtle** - Low emphasis (minimal styling)
4. **Transparent** - No background until hover

**Note**: `Outline` variant does NOT exist (common mistake from other UI libraries)

## ButtonSize Options

- `Small` - Low height, compact
- (Default/Medium - not specified)
- (Large - seen in usage)

## Pattern: Check Codebase First

When working with unfamiliar library APIs:

1. **Grep for existing usage** - `rg "LibraryType::" crates/`
2. **Read real code examples** - More reliable than external docs
3. **Check patterns in multiple files** - Establishes conventions
4. **Use cargo check errors** - Often reveals available variants

## Example Usage

```rust
use thaw::{Button, ButtonAppearance, ButtonSize};

// Transparent button for condition text
<Button
    appearance=ButtonAppearance::Transparent
    size=ButtonSize::Small
    on_click=move |_| on_edit.run(())
    attr:class="custom-class"
>
    {text}
</Button>

// Subtle button for "add" action
<Button
    appearance=ButtonAppearance::Subtle
    size=ButtonSize::Small
    on_click=move |_| on_add.run(())
>
    "+ Добавить"
</Button>

// Secondary for delete (with custom red styling)
<Button
    appearance=ButtonAppearance::Secondary
    on_click=handle_delete
    attr:class="delete-condition-btn"
>
    "Удалить условие"
</Button>
```

## Custom Styling with Thaw

To override Thaw button styles:

- Use `attr:class="custom-class"`
- Apply CSS overrides with `!important`
- Target specific states: `:hover`, `:active`

## Related

- Session: [[2025-01-29-session-debrief-thaw-button-migration]]
