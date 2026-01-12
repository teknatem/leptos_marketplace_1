---
type: known-issue
date: 2025-01-12
tags: [thaw, checkbox, ui, leptos]
severity: low
status: workaround-available
---

# THAW Checkbox Has No Disabled Property

## Problem

THAW UI library's `Checkbox` component doesn't support a `disabled` property for creating read-only checkboxes.

## Detection

Compilation error:

```
error[E0599]: no method named `disabled` found for struct `thaw::CheckboxPropsBuilder`
```

When trying:

```rust
<Checkbox checked=RwSignal::new(value) disabled=true label="Label" />
```

## Workaround

Use `Badge` component to display boolean values in read-only forms:

```rust
<Badge
    appearance=BadgeAppearance::Outline
    color=if value { BadgeColor::Success } else { BadgeColor::Danger }
>
    {if value { "Label: Yes" } else { "Label: No" }}
</Badge>
```

Or use readonly `Input` with text (allows selection/copy):

```rust
<Input value=RwSignal::new(if value { "Да" } else { "Нет" }) attr:readonly=true />
```

## Affected Components

- Any read-only detail form displaying boolean values
- `a012_wb_sales` - is_supply, is_realization fields
