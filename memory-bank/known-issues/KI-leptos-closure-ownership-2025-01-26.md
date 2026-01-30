---
type: known-issue
date: 2025-01-26
severity: medium
framework: leptos
status: documented
tags:
  - leptos
  - closures
  - reactive
  - ownership
---

# Known Issue: Leptos Closure Ownership in Multiple Reactive Contexts

## Problem

When using a closure in multiple reactive contexts (multiple `move ||` blocks) in Leptos view macros, Rust's ownership rules cause compile errors.

## Symptoms

### Error Message
```
error[E0382]: use of moved value: `my_closure`
  --> path/to/file.rs:100:50
   |
90 | let my_closure = move || { /* ... */ };
   |     ---------- move occurs because has type `{closure@...}`, which does not implement `Copy`
95 | <option selected={move || my_closure() == value}>
   |                    ------- ---------- variable moved due to use in closure
   |                    value moved into closure here
100| <option selected={move || my_closure() == other}>
   |                   ^^^^^^^ ---------- use occurs due to use in closure
   |                   value used here after move
```

### Code Pattern That Fails
```rust
let get_value = move || {
    let cfg = config.get();
    cfg.some_field.contains(&id)
};

view! {
    <select>
        <option selected={move || get_value()}>"Option 1"</option>
        <option selected={move || get_value()}>"Option 2"</option>  // ❌ Error here
    </select>
}
```

## Root Cause

- Leptos `move ||` closures take ownership of captured variables
- First `move ||` consumes the closure
- Subsequent uses cannot access the moved closure
- Rust closures don't implement `Copy` by default

## Solution

Use `StoredValue` to wrap the closure, allowing multiple accesses via references.

### Fixed Code Pattern
```rust
use leptos::prelude::StoredValue;

let get_value = StoredValue::new(move || {
    let cfg = config.get();
    cfg.some_field.contains(&id)
});

view! {
    <select>
        <option selected={move || get_value.with_value(|f| f())}>"Option 1"</option>
        <option selected={move || get_value.with_value(|f| f())}>"Option 2"</option>  // ✅ Works
    </select>
}
```

## When to Apply

Use `StoredValue` wrapper when:

1. **Closure used in multiple reactive attributes** (selected, disabled, class)
2. **Closure used in both attributes and content** (span class + text)
3. **Event handlers used multiple times** (on:click, on:change)
4. **Closure needs to be accessed in parent and child components**

## Alternative Solutions

### 1. Clone Before Move (Not Recommended)
```rust
// Works but verbose and inefficient
let get_value = || { /* ... */ };
<option selected={move || { let f = get_value.clone(); f() }}>
```

### 2. Recompute Each Time (Not Recommended)
```rust
// Duplicated logic, hard to maintain
<option selected={move || config.get().field.contains(&id)}>
<option selected={move || config.get().field.contains(&id)}>
```

### 3. Use Signal Derived Value (Alternative)
```rust
let derived = Signal::derive(move || config.get().field.contains(&id));
<option selected={move || derived.get()}>
```

## Related Patterns

### Event Handlers
```rust
let handler = StoredValue::new(move |ev: Event| {
    // event handling logic
});

// Use in multiple places
on:click=move |ev| handler.with_value(|h| h(ev))
on:change=move |ev| handler.with_value(|h| h(ev))
```

### Complex Computed Values
```rust
let get_role = StoredValue::new({
    let field_id = field_id.clone();
    move || -> FieldRole {
        let cfg = config.get();
        if cfg.groupings.contains(&field_id) {
            FieldRole::Grouping
        } else { /* ... */ }
    }
});

// Use anywhere
if get_role.with_value(|f| f() == FieldRole::Measure) { /* ... */ }
```

## Performance Considerations

- `StoredValue` has minimal overhead (reference indirection)
- Preferred over cloning closures repeatedly
- No significant impact on reactivity

## Documentation References

- Leptos Book: Reactive System
- Leptos API: `StoredValue<T>`
- Related: `create_memo` for cached derived values

## Examples in Codebase

- `crates/frontend/src/shared/pivot/settings_table.rs:150` - `get_role_func`
- `crates/frontend/src/shared/pivot/settings_table.rs:213` - `on_aggregate_change`
- `crates/frontend/src/shared/pivot/settings_table.rs:271` - `on_filter_value_change`
