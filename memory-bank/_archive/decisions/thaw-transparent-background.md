---
adr_number: 1
title: Programmatic CSS Variable Manipulation for Thaw Forest Theme
date: 2025-12-20
status: accepted
deciders: [user, assistant]
tags: [adr, thaw-ui, theming, css]
---

# ADR-0001: Programmatic CSS Variable Manipulation for Thaw Forest Theme

## Status

**Accepted** - Implemented and working

## Context

The application uses three themes: light, dark, and forest. The forest theme has a custom background texture that should be visible through Thaw UI components. By default, Thaw UI's `ConfigProvider` sets an opaque background color via the `--colorNeutralBackground1` CSS variable, which obscures the forest texture.

### Requirements

- Make Thaw UI background transparent ONLY for forest theme
- Keep default Thaw backgrounds for light and dark themes
- User specified preference for "simplest option"
- Must work on page load and theme switching

### Constraints

- Thaw UI 0.5.0-beta (limited API, beta stability)
- Leptos 0.8 with WASM target
- Existing theme system using localStorage persistence
- Three-theme system already implemented

## Decision

**Implement programmatic CSS variable override using `web_sys`**

Specifically:

1. Create `set_thaw_background(value: &str)` function that:

   - Uses `query_selector` to find `.thaw-config-provider` element
   - Sets `--colorNeutralBackground1` CSS variable via `style.set_property`
   - Sets to `"transparent"` for forest theme, `""` (empty/default) for others

2. Call this function:

   - In `change_theme()` when user switches themes
   - In `Effect::new` on component mount to apply saved theme

3. Location: `crates/frontend/src/shared/theme/theme_select.rs`

## Alternatives Considered

### Alternative 1: CSS Stylesheet Override

**Description**: Add CSS rule to override the variable

```css
[data-theme="forest"] .thaw-config-provider {
  --colorNeutralBackground1: transparent;
}
```

**Pros**:

- Standard CSS approach
- No JavaScript/WASM code needed
- Works without ConfigProvider being found

**Cons**:

- Requires CSS file modification
- Specificity conflicts possible
- Needs Trunk rebuild to see changes
- Less dynamic (can't easily toggle)

**Verdict**: ❌ Rejected - User requested "simplest option" and CSS rebuild is slower iteration

### Alternative 2: Custom Thaw Theme

**Description**: Create a third Thaw theme with transparent background

```rust
let forest_theme = Theme::dark()
    .with_variable("--colorNeutralBackground1", "transparent");
```

**Pros**:

- Type-safe
- Thaw-native approach
- Compile-time checked
- Most "proper" solution

**Cons**:

- Thaw 0.5.0-beta API unclear (may not support customization)
- More complex implementation
- Requires deeper Thaw knowledge
- Potentially fragile with beta library

**Verdict**: ❌ Rejected - Too complex for current need, save for future refactor

### Alternative 3: Conditional ConfigProvider Rendering

**Description**: Render ConfigProvider differently based on theme

```rust
{move || match theme.get() {
    "forest" => view! { <ConfigProvider theme style="--colorNeutralBackground1: transparent"> },
    _ => view! { <ConfigProvider theme> },
}}
```

**Pros**:

- Leptos-native approach
- Reactive
- Clear intent

**Cons**:

- Requires modifying App component structure
- Causes full remount of ConfigProvider on theme change
- May cause flashing/performance issues
- Verbose and repetitive

**Verdict**: ❌ Rejected - Causes unnecessary remounting, not "simplest"

### Alternative 4: Inline Style Prop (if supported)

**Description**: Pass inline style to ConfigProvider

```rust
<ConfigProvider theme style="--colorNeutralBackground1: transparent">
```

**Pros**:

- Simple one-liner
- Reactive with signals

**Cons**:

- Thaw ConfigProvider may not accept `style` prop
- Would need conditional logic anyway
- Unclear if CSS variables work via inline styles

**Verdict**: ❌ Rejected - Uncertain API support, still requires conditionals

## Decision Rationale

**Why programmatic CSS variable manipulation won:**

1. **Simplicity**: Direct, ~15 lines of code, single function
2. **User preference**: Explicitly requested "simplest option"
3. **No external dependencies**: Uses existing `web_sys`
4. **Fast iteration**: No CSS rebuild needed, instant hot-reload
5. **Surgical**: Only affects what needs to change
6. **Proven pattern**: Used in Effect and event handlers (reliable timing)
7. **Fail-safe**: Silent failure if element not found (acceptable for styling)

## Consequences

### Positive

- ✅ Quick implementation (~10 minutes)
- ✅ Works immediately on theme switch
- ✅ Works on page load with saved theme
- ✅ No impact on light/dark themes
- ✅ Easy to modify or extend
- ✅ Compiles with 0 errors

### Negative

- ⚠️ Runtime DOM query (small performance cost)
- ⚠️ Silent failure if selector changes (brittle to Thaw updates)
- ⚠️ Not type-safe (CSS variable name is string)
- ⚠️ Mixes concerns (theme logic + DOM manipulation)
- ⚠️ Potential timing issues (element must exist)

### Mitigations

- **For selector brittleness**: Document dependency on `.thaw-config-provider` class
- **For timing**: Use Effect::new (safe) and event handlers (synchronous)
- **For type safety**: Consider adding constant for variable name
- **For concerns mixing**: Acceptable for this scope, refactor if theme system grows

## Implementation Notes

```rust
/// Set CSS variable for Thaw ConfigProvider
fn set_thaw_background(value: &str) {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(element) = document
                .query_selector(".thaw-config-provider")
                .ok()
                .flatten()
            {
                if let Some(html_element) = element.dyn_ref::<web_sys::HtmlElement>() {
                    let _ = html_element
                        .style()
                        .set_property("--colorNeutralBackground1", value);
                }
            }
        }
    }
}
```

**Key design choices**:

- Returns `()` not `Result` - styling failure is non-critical
- Uses `ok().flatten()` pattern for Result<Option<\_>>
- Empty string `""` resets to default (browser behavior)
- `dyn_ref` for safe type casting

## Success Metrics

- [x] Compiles without errors
- [x] Function added successfully
- [x] Called in `change_theme()`
- [x] Called in `Effect::new`
- [ ] **Pending**: User confirms visual transparency in browser
- [ ] **Pending**: Works across theme switches
- [ ] **Pending**: Persists after reload

## Review & Evolution

**Review date**: 2025-12-20
**Next review**: When Thaw UI 1.0 releases or if issues reported

**Future considerations**:

- If Thaw adds theme customization API, migrate to Alternative 2
- If more CSS variables need modification, create generic helper
- If timing issues occur, add retry logic (see runbook)
- If performance matters, consider CSS solution (Alternative 1)

## References

- Implementation: `crates/frontend/src/shared/theme/theme_select.rs`
- Documentation: `THAW_FOREST_THEME.md`
- Session: [[2025-12-20-session-debrief-thaw-forest-theme]]
- Runbook: [[RB-thaw-css-variables-v1]]
- Lesson: [[LL-css-variable-timing-2025-12-20]]

## Decision History

- **2025-12-20**: Decision made and implemented
- **2025-12-20**: User approved plan, requested implementation
- **2025-12-20**: Implementation completed, pending user testing

---

**Deciders**: User (product owner), AI Assistant (implementer)
**Decision method**: User specified constraint ("simplest option"), AI proposed solution, user approved





