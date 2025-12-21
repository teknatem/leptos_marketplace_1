---
title: Lesson - Hybrid Thaw UI and HTML Components
date: 2025-12-20
category: frontend
tags: [lesson, thaw-ui, html, pragmatic-decisions]
confidence: high
---

# Lesson: When to Keep HTML Components During Thaw Migration

## Context

During systematic migration of pages to Thaw UI (a006_connection_mp, u502_import_from_ozon), we encountered situations where Thaw doesn't provide necessary components.

## The Lesson

**It's acceptable to use HTML form elements styled with CSS variables instead of forcing Thaw components when:**

1. Thaw doesn't provide the component at all
2. Thaw's alternative is overly complex for the use case
3. Native HTML provides better UX (e.g., date pickers)

## Specific Cases

### ✅ Keep HTML `<select>` for dropdowns

**Reason**: Thaw only provides `Combobox` which is complex and designed for search/autocomplete. Simple dropdown selection is better served by native `<select>`.

**Implementation**:

```rust
<select
    style="width: 100%; padding: 10px; border: 1px solid var(--color-border);
           border-radius: 6px; background: var(--color-background-primary);"
    on:change=move |ev| set_value.set(event_target_value(&ev))
>
    <option value="1">"Option 1"</option>
</select>
```

**Key**: Style with CSS variables for theme consistency.

### ✅ Keep HTML `<input type="date">` for date pickers

**Reason**: Thaw has no DatePicker component. Native date input provides:

- Localized calendar UI
- Browser-native validation
- Mobile-optimized UX

**Implementation**:

```rust
<input
    type="date"
    prop:value=move || date.get().format("%Y-%m-%d").to_string()
    on:change=move |ev| {
        let value = event_target_value(&ev);
        if let Ok(d) = chrono::NaiveDate::parse_from_str(&value, "%Y-%m-%d") {
            set_date.set(d);
        }
    }
    style="padding: 8px; border: 1px solid var(--color-border);
           border-radius: 6px; font-size: 14px;"
/>
```

## Anti-Pattern: Don't Force Thaw When Inappropriate

❌ **Don't** try to recreate date picker using Thaw `Input` + manual parsing
❌ **Don't** use complex `Combobox` for simple single-select dropdowns
❌ **Don't** abandon HTML completely out of purist ideology

## The Pragmatic Principle

> "Use Thaw for consistency where it provides value. Use HTML where Thaw doesn't provide the component or where HTML is simpler and better."

## Styling Guidelines for HTML Components

When keeping HTML components, always:

1. **Use CSS variables** for colors, borders, backgrounds
2. **Match Thaw sizing**: padding `8-10px`, font-size `14px`
3. **Match Thaw radii**: `6px` or `8px` border-radius
4. **Maintain interactivity**: proper hover, focus, disabled states

## Future Considerations

- Monitor Thaw releases for new components (DatePicker, Select)
- Consider creating wrapper components for common HTML patterns
- Document which pages use HTML components for easier future migration

## Related Pages

- `u502_import_from_ozon/view.rs` - Uses HTML select and date inputs
- `a006_connection_mp/ui/details/view.rs` - Mixed Thaw and HTML

## Validation

This approach has been validated across multiple pages with:

- ✅ Successful compilation
- ✅ Theme consistency (light/dark mode)
- ✅ Good user experience
- ✅ No accessibility issues
