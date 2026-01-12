---
type: known-issue
date: 2025-01-12
tags: [leptos, rust, closures, reactive]
severity: medium
status: pattern-established
---

# Leptos Closures: FnOnce vs Fn in Reactive Context

## Problem

When creating event handlers in Leptos that capture and move variables, the closure becomes `FnOnce`. However, Leptos reactive components (like `Show`, `For`) require `Fn` closures that can be called multiple times.

## Detection

Compilation error:

```
error[E0525]: expected a closure that implements the `Fn` trait, but this closure only implements `FnOnce`
closure is `FnOnce` because it moves the variable `X` out of its environment
```

## Bad Pattern

```rust
let vm = view_model.clone();
let handle_action = move |_| vm.some_method(); // FnOnce!

view! {
    <Show when=move || condition.get()>
        <Button on_click=handle_action /> // Error: handle_action is FnOnce
    </Show>
}
```

## Solution: Use Callback

Wrap closures in `Callback::new()` which internally uses `Rc` to allow multiple calls:

```rust
let on_action = {
    let vm = view_model.clone();
    Callback::new(move |_: ()| vm.some_method())
};

view! {
    <Show when=move || condition.get()>
        <Button on_click=move |_| on_action.run(()) /> // Works!
    </Show>
}
```

## Alternative: Separate Component

Extract the button into a separate component that receives its own clone:

```rust
#[component]
fn ActionButton(vm: ViewModel) -> impl IntoView {
    view! {
        <Button on_click={
            let vm = vm.clone();
            move |_| vm.some_method()
        } />
    }
}
```

## Affected Patterns

- Conditional rendering with `Show` containing buttons
- Dynamic lists with `For` containing clickable items
- Nested reactive contexts
