---
title: Modifying Thaw UI CSS Variables at Runtime
version: 1.0
date: 2025-12-20
tags: [runbook, thaw-ui, css-variables, leptos, web-sys]
applies_to: Leptos projects using Thaw UI 0.5.0-beta
---

# Runbook: Modifying Thaw UI CSS Variables at Runtime

## Purpose

Step-by-step procedure for dynamically modifying Thaw UI component styling by changing CSS variables programmatically using `web_sys` in a Leptos/WASM application.

## When to Use

- Need to override Thaw UI styles without creating custom theme
- Want theme-specific visual adjustments (e.g., transparency, color tweaks)
- Require runtime style changes based on application state
- Quick styling fixes during development

## Prerequisites

- Leptos project with Thaw UI integrated
- `web_sys` dependency available
- `ConfigProvider` wrapping the application
- Basic understanding of CSS custom properties (variables)

## Procedure

### Step 1: Identify the Target CSS Variable

**Action**: Find which Thaw UI CSS variable controls the style you want to change.

**Common Thaw UI variables**:

- `--colorNeutralBackground1` - Primary background color
- `--colorNeutralBackground2` - Secondary background
- `--colorBrandBackground` - Brand/accent backgrounds
- `--colorNeutralForeground1` - Primary text color
- `--colorBrandForeground1` - Brand text color

**How to find**:

1. Inspect element in browser DevTools
2. Look at computed styles → CSS variables
3. Check Thaw UI documentation (if available)
4. Examine `.thaw-config-provider` element styles

### Step 2: Create a Helper Function

**Location**: In your theme management file (e.g., `theme_select.rs`)

**Template**:

```rust
use wasm_bindgen::JsCast;

/// Set CSS variable on Thaw ConfigProvider
fn set_thaw_css_variable(variable_name: &str, value: &str) {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            // Find .thaw-config-provider element
            if let Some(element) = document
                .query_selector(".thaw-config-provider")
                .ok()
                .flatten()
            {
                if let Some(html_element) = element.dyn_ref::<web_sys::HtmlElement>() {
                    let _ = html_element
                        .style()
                        .set_property(variable_name, value);
                }
            }
        }
    }
}
```

**Notes**:

- Function silently fails if element not found (safe but invisible)
- Uses `ok().flatten()` to handle Result<Option<\_>>
- `dyn_ref` safely casts to HtmlElement

### Step 3: Call Function at Appropriate Times

**Where to call**:

1. **On theme change** (in theme switching function):

```rust
fn change_theme(theme: String) {
    // ... other theme logic ...

    // Set CSS variable based on theme
    match theme.as_str() {
        "forest" => set_thaw_css_variable("--colorNeutralBackground1", "transparent"),
        _ => set_thaw_css_variable("--colorNeutralBackground1", ""),
    }
}
```

2. **On component mount** (in Effect):

```rust
Effect::new(move |_| {
    // Apply saved settings
    let saved_theme = get_saved_theme();

    if saved_theme == "forest" {
        set_thaw_css_variable("--colorNeutralBackground1", "transparent");
    }
});
```

3. **On user action** (e.g., toggle button):

```rust
let toggle_transparency = move |_| {
    let value = if transparent.get() { "transparent" } else { "" };
    set_thaw_css_variable("--colorNeutralBackground1", value);
    transparent.update(|t| *t = !*t);
};
```

### Step 4: Test the Implementation

**Testing checklist**:

- [ ] Variable changes immediately in browser DevTools
- [ ] Visual change is visible (use contrast to verify)
- [ ] Change persists during navigation (if using signals)
- [ ] Change applies after page reload (if saved to localStorage)
- [ ] No console errors in browser
- [ ] Works in all target browsers

**Debug commands**:

```javascript
// In browser console:
// 1. Check if element exists
document.querySelector(".thaw-config-provider");

// 2. Check current variable value
getComputedStyle(
  document.querySelector(".thaw-config-provider")
).getPropertyValue("--colorNeutralBackground1");

// 3. Manually test setting
document
  .querySelector(".thaw-config-provider")
  .style.setProperty("--colorNeutralBackground1", "transparent");
```

### Step 5: Handle Edge Cases

**Edge case 1: Element not found**

- **Symptom**: Style doesn't change
- **Cause**: ConfigProvider not rendered yet or wrong selector
- **Fix**: Add retry logic or use `requestAnimationFrame`

```rust
// Option 1: Retry with timeout
use gloo_timers::callback::Timeout;

fn set_thaw_css_variable_with_retry(var: &str, value: &str, retries: u32) {
    // Try immediate
    if set_thaw_css_variable(var, value) {
        return; // Success
    }

    // Retry after short delay
    if retries > 0 {
        let var = var.to_string();
        let value = value.to_string();
        Timeout::new(100, move || {
            set_thaw_css_variable_with_retry(&var, &value, retries - 1);
        }).forget();
    }
}
```

**Edge case 2: Value doesn't apply**

- **Symptom**: Variable changes but style doesn't update
- **Cause**: Variable is overridden by more specific CSS rule
- **Fix**: Use `!important` or check CSS specificity

```rust
// Add !important to force override
set_thaw_css_variable("--colorNeutralBackground1", "transparent !important")
```

**Edge case 3: Flashing during theme switch**

- **Symptom**: Brief flash of old style before new style applies
- **Cause**: Async DOM updates
- **Fix**: Batch updates or use CSS transitions

## Common Use Cases

### Use Case 1: Transparent Background

```rust
set_thaw_css_variable("--colorNeutralBackground1", "transparent");
```

### Use Case 2: Custom Brand Color

```rust
set_thaw_css_variable("--colorBrandBackground", "#ff6b6b");
set_thaw_css_variable("--colorBrandForeground1", "#ffffff");
```

### Use Case 3: Dark Mode Override

```rust
set_thaw_css_variable("--colorNeutralBackground1", "#1a1a1a");
set_thaw_css_variable("--colorNeutralForeground1", "#e0e0e0");
```

### Use Case 4: High Contrast Mode

```rust
set_thaw_css_variable("--colorNeutralBackground1", "#000000");
set_thaw_css_variable("--colorNeutralForeground1", "#ffffff");
set_thaw_css_variable("--colorBrandBackground", "#ffff00");
```

## Alternatives

### Alternative 1: Custom Thaw Theme

**Pros**: Type-safe, compile-time checked, cleaner
**Cons**: More complex, requires Thaw API knowledge
**When**: Long-term solution, many customizations

### Alternative 2: CSS Stylesheet Override

**Pros**: Standard CSS approach, works without JS
**Cons**: Requires CSS rebuild, less dynamic
**When**: Static themes, no runtime changes needed

### Alternative 3: Inline Styles on ConfigProvider

**Pros**: Leptos-native, reactive
**Cons**: Requires changing App component, verbose
**When**: Few variables, want reactive integration

## Troubleshooting

| Problem                     | Diagnosis                      | Solution                                       |
| --------------------------- | ------------------------------ | ---------------------------------------------- |
| Style doesn't change        | Element not found              | Check selector, verify ConfigProvider rendered |
| Change not visible          | Variable not used by component | Verify correct variable name in DevTools       |
| Works in dev, fails in prod | Timing issue                   | Add retry logic or mount delay                 |
| Performance degradation     | Too many updates               | Debounce updates, batch changes                |
| Flashing UI                 | DOM update race                | Use CSS transitions or batch updates           |

## References

- [MDN: CSS Custom Properties](https://developer.mozilla.org/en-US/docs/Web/CSS/Using_CSS_custom_properties)
- [web_sys HtmlElement](https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.HtmlElement.html)
- Thaw UI Repository: https://github.com/thaw-ui/thaw
- Related: [[2025-12-20-session-debrief-thaw-forest-theme]]

## Version History

- **v1.0** (2025-12-20): Initial runbook based on forest theme implementation





