---
date: 2025-01-29
tags: [known-issue, rust, ownership, leptos]
severity: medium
status: documented
---

# Known Issue: Checking Option State After Value is Moved

## Issue Description

When a function parameter of type `Option<T>` (where `T` doesn't implement `Copy`) is used in closures or effects, attempting to check its state (`.is_some()`) after the value has been moved causes a compilation error.

## Error Message

```
error[E0382]: borrow of moved value: `existing_condition`
   |
   | existing_condition: Option<FilterCondition>,
   | ------------------ move occurs because `existing_condition` has type
   |                    `std::option::Option<FilterCondition>`,
   |                    which does not implement the `Copy` trait
```

## Detection

- Appears when checking `option_value.is_some()` or similar
- After the option has been consumed by a closure (`move ||`) or Effect
- Error specifically mentions "borrow of moved value"

## Root Cause

Rust's ownership rules prevent borrowing a value after it has been moved. When `Option<T>` is moved into a closure or Effect, the original binding is no longer valid.

## Bad Pattern

```rust
#[component]
pub fn MyComponent(
    existing_value: Option<SomeType>,
) -> impl IntoView {
    // Effect consumes existing_value
    Effect::new(move || {
        if let Some(val) = &existing_value {
            // use val
        }
    });

    // ❌ ERROR: existing_value was moved above
    let has_value = existing_value.is_some();
}
```

## Fix: Check Before Move

Move the state check to the **very beginning** of the function, before any closures or Effects consume the value:

```rust
#[component]
pub fn MyComponent(
    existing_value: Option<SomeType>,
) -> impl IntoView {
    // ✅ Check state FIRST, before value is moved
    let has_value = existing_value.is_some();

    // Now safe to move into Effect
    Effect::new(move || {
        if let Some(val) = &existing_value {
            // use val
        }
    });

    // Can safely use has_value later
    view! {
        {move || {
            if has_value {
                view! { <div>"Has value"</div> }
            } else {
                view! { <div>"No value"</div> }
            }
        }}
    }
}
```

## Alternative: Clone if Possible

If the inner type implements `Clone`, you can clone the option:

```rust
let has_value = existing_value.is_some();
let existing_clone = existing_value.clone();

Effect::new(move || {
    // use existing_value
});

// use has_value or existing_clone
```

## Context: Leptos Components

This pattern is particularly common in Leptos components where:

- Props are consumed by reactive effects
- Component logic needs to branch based on prop state
- The reactive system moves values into closures

## Prevention Checklist

1. ✅ Extract boolean state checks immediately after `#[component]` function starts
2. ✅ Order operations: state checks → closures/Effects → view rendering
3. ✅ Consider if `Clone` is appropriate for the type
4. ✅ Use `StoredValue` or signals if multiple closures need access

## Real Example

From `ConditionEditorModal`:

```rust
#[component]
pub fn ConditionEditorModal(
    existing_condition: Option<FilterCondition>,
    on_delete: Option<Callback<()>>,
) -> impl IntoView {
    // ✅ Check BEFORE existing_condition is moved
    let has_existing_condition = existing_condition.is_some();

    // State and effects that consume existing_condition
    Effect::new(move || {
        if let Some(cond) = &existing_condition {
            // load condition into state
        }
    });

    // Later, can safely use has_existing_condition
    view! {
        {move || {
            if has_existing_condition && on_delete.is_some() {
                view! { <Button>"Delete"</Button> }
            } else {
                view! { <div></div> }
            }
        }}
    }
}
```

## Related

- Session: [[2025-01-29-session-debrief-thaw-button-migration]]
- Rust Book: Chapter 4 - Understanding Ownership
