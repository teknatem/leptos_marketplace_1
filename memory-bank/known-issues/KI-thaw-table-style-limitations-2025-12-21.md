---
title: Known Issue - Thaw Table Style System Limitations
issue_id: KI-THAW-STYLES-001
date_identified: 2025-12-21
severity: low
status: documented
tags: [known-issue, thaw-ui, css, styling]
affected_components:
  - Thaw UI Table components
workaround_available: yes
---

# Known Issue: Thaw Table Style System Limitations

## Summary

Thaw UI tables use a different CSS variable system than the project's custom design system, making it impossible to achieve 100% visual consistency when mixing native HTML tables with Thaw tables, or when trying to apply Thaw styles to native tables.

## Issue Details

### Description

The project uses two parallel design systems:

1. **Thaw UI Design System** (Fluent-based):

   - CSS variables: `--colorNeutralBackground1`, `--colorBrandForeground1`, etc.
   - Applied via `<ConfigProvider theme>`
   - Only affects Thaw components
   - Styles embedded in WASM/JS bundle

2. **Project Design System** (Custom):
   - CSS variables: `--color-bg-primary`, `--color-text-primary`, etc.
   - Defined in `crates/frontend/static/themes/`
   - Applied via custom CSS classes
   - Used for native HTML elements

### Impact

**Cannot do**:

- ❌ Apply Thaw's exact visual styles to native `<table>` elements
- ❌ Extract Thaw's CSS to use independently
- ❌ Guarantee 100% visual consistency between Thaw and native tables
- ❌ Share color schemes directly between systems

**Can do**:

- ✅ Imitate Thaw's visual style using similar values
- ✅ Map Thaw variables to project variables for theme switching
- ✅ Use Thaw components with project colors (via variable overrides)

## Examples

### Attempting to Use Thaw Styles on Native Table

```html
<!-- This WON'T work as expected -->
<table class="thaw-table">
  <tr class="thaw-table__row">
    <td class="thaw-table__cell">
      <!-- Variables like --colorNeutralBackground1 are undefined -->
    </td>
  </tr>
</table>
```

**Problem**: Thaw's CSS classes expect Thaw's CSS variables, which aren't defined for native elements.

### Mixing Tables in Same View

```rust
// Thaw Table
<Table>
  <TableRow>
    <TableCell>"Data"</TableCell>
  </TableRow>
</Table>

// Native Table
<table class="table__data">
  <tr class="table__row">
    <td class="table__cell">"Data"</td>
  </tr>
</table>
```

**Problem**: Even with similar styling, subtle differences in padding, borders, colors will be visible.

## Root Cause

1. **Different CSS Variable Namespaces**:

   - Thaw: `--color*` (Fluent naming)
   - Project: `--color-*` (BEM-style naming)

2. **Style Bundling**:

   - Thaw styles are compiled into WASM/JS
   - Cannot extract to external CSS file
   - Cannot view or copy Thaw's CSS rules

3. **Component Coupling**:
   - Thaw's components are tightly coupled to its theme system
   - Cannot use Thaw components without ConfigProvider
   - Cannot use Thaw styles without Thaw components

## Workarounds

### Workaround 1: CSS Variable Mapping (for theme switching)

Map Thaw variables to project variables in root CSS:

```css
:root {
  /* Map Thaw variables to project variables */
  --colorNeutralBackground1: var(--color-bg-primary);
  --colorNeutralForeground1: var(--color-text-primary);
  --colorBrandBackground: var(--color-primary);
}
```

**Use when**: Need Thaw components to follow project theme.

### Workaround 2: Visual Imitation (for native tables)

Manually replicate Thaw's visual style:

```css
.table__cell--thaw-style {
  padding: 8px 12px;
  font-size: 14px;
  border-bottom: 1px solid var(--color-border);
}
```

**Use when**: Want native table to look similar to Thaw.

### Workaround 3: Standardize on One System

**Option A**: Use Thaw Tables everywhere

- Pros: Consistent component API
- Cons: Less control, resize issues, event conflicts

**Option B**: Use native tables everywhere

- Pros: Full control, proven patterns in project
- Cons: More code, manual styling

**Project choice**: Hybrid - most tables use native HTML, some use Thaw where convenient.

## Detection

**How to identify when this affects you**:

1. Visual inconsistency between tables
2. Styles not applying to native elements with Thaw classes
3. Theme changes affecting only some tables
4. DevTools showing undefined CSS variables

**Browser console check**:

```javascript
// Check if Thaw variable is defined
getComputedStyle(document.querySelector(".thaw-table")).getPropertyValue(
  "--colorNeutralBackground1"
);
// Returns value for Thaw components, empty for native elements
```

## Related Issues

- [[LL-thaw-html-hybrid-2025-12-20]] - When to use HTML vs Thaw
- [[ADR-0001-thaw-transparent-background]] - Overriding Thaw variables
- [[THAW_THEME_SYNC.md]] - Theme synchronization system

## Resolution

**Status**: Accepted as design constraint.

**Decision**: Project will continue using hybrid approach (native + Thaw), accepting minor visual differences.

**Rationale**:

- Native tables provide more flexibility for complex interactions
- Thaw tables are simpler for basic cases
- Visual differences are minor and acceptable
- Cost of standardizing entire codebase is too high

## Recommendations

For new tables:

1. **Use native HTML** if table needs:

   - Column resizing
   - Complex row interactions
   - Custom layouts
   - Following existing patterns (a002, a016)

2. **Use Thaw Table** if table is:

   - Simple display-only
   - Benefits from Thaw's built-in features
   - Part of form within Thaw Dialog

3. **Never** try to mix Thaw and native in same table

## Testing

When mixing Thaw and native tables:

- [ ] Check visual consistency in light/dark/forest themes
- [ ] Verify theme switching affects both table types
- [ ] Test color contrast meets accessibility standards
- [ ] Review in different browsers (style variable support)

## References

- Thaw UI Repository: https://github.com/thaw-ui/thaw
- Project theme files: `crates/frontend/static/themes/`
- Session discussion: [[2025-12-21-session-debrief-a006-signal-sorting]]
