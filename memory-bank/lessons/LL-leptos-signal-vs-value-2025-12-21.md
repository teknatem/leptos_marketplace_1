---
title: Lesson - Signal Parameters vs Value Parameters in Leptos
date: 2025-12-21
category: leptos
tags: [lesson, leptos, signal, reactivity, parameters]
confidence: high
applies_to: All Leptos components with dynamic data
---

# Lesson: When to Use Signal Parameters in Leptos Components

## Context

Encountered a bug where a detail form always showed data from the first opened record, even when opening different records or creating new ones.

## The Problem

```rust
// Parent component
let (editing_id, set_editing_id) = signal::<Option<String>>(None);

view! {
    <ConnectionMPDetails
        id=editing_id.get()  // ❌ WRONG: Evaluates ONCE
    />
}

// Child component
#[component]
pub fn ConnectionMPDetails(
    id: Option<String>,  // ❌ Non-reactive parameter
    ...
) {
    // This only runs ONCE when component is created
    if let Some(conn_id) = id {
        // Load data...
    }
}
```

**What happens**:

1. User clicks "Record 1" → `editing_id.set(Some("uuid-1"))`
2. Component created with `id = Some("uuid-1")`
3. User clicks "Record 2" → `editing_id.set(Some("uuid-2"))`
4. Component is NOT recreated, still has `id = Some("uuid-1")`
5. Form shows wrong data!

## The Solution

```rust
// Parent component
let (editing_id, set_editing_id) = signal::<Option<String>>(None);

view! {
    <ConnectionMPDetails
        id=editing_id  // ✅ CORRECT: Pass the signal itself
    />
}

// Child component
#[component]
pub fn ConnectionMPDetails(
    #[prop(into)] id: Signal<Option<String>>,  // ✅ Reactive parameter
    ...
) {
    // This runs EVERY TIME id changes
    Effect::new(move |_| {
        match id.get() {
            Some(conn_id) => {
                // Load data for conn_id
            }
            None => {
                // Reset form
            }
        }
    });
}
```

**What happens now**:

1. User clicks "Record 1" → `editing_id.set(Some("uuid-1"))`
2. Effect runs → loads data for uuid-1
3. User clicks "Record 2" → `editing_id.set(Some("uuid-2"))`
4. Effect runs AGAIN → loads data for uuid-2
5. Form shows correct data! ✅

## Rule of Thumb

Use **Signal parameters** when:

- ✅ Component needs to respond to external state changes
- ✅ Parent component updates a value that child must react to
- ✅ Data loading should happen when parameter changes
- ✅ Form should reset/reload based on external state

Use **Value parameters** when:

- ✅ Component receives constant/static data
- ✅ Data is computed once and never changes
- ✅ Component is recreated when parent needs different data

## Common Patterns

### Pattern 1: Detail Forms

```rust
// ALWAYS use Signal for record ID
#[component]
pub fn RecordDetails(
    #[prop(into)] id: Signal<Option<String>>,
) {
    Effect::new(move |_| {
        if let Some(record_id) = id.get() {
            // Fetch and display record
        } else {
            // Show empty form for new record
        }
    });
}
```

### Pattern 2: Conditional Rendering

```rust
// Use Signal for show/hide state
#[component]
pub fn Modal(
    #[prop(into)] open: Signal<bool>,
    children: Children,
) {
    view! {
        <div class:hidden=move || !open.get()>
            {children()}
        </div>
    }
}
```

### Pattern 3: Live Search

```rust
// Use Signal for search query
#[component]
pub fn SearchResults(
    #[prop(into)] query: Signal<String>,
) {
    Effect::new(move |_| {
        let q = query.get();
        if !q.is_empty() {
            // Perform search
        }
    });
}
```

## Technical Details

**Why `#[prop(into)]`?**

Allows flexible calling:

```rust
// Can pass Signal directly
<Component id=my_signal />

// Can pass derived Signal
<Component id=Signal::derive(move || some_computation()) />

// Leptos converts automatically
```

**Why `Effect::new`?**

- Tracks all reactive dependencies (signals)
- Runs automatically when any dependency changes
- Cleans up properly when component unmounts

## Anti-Patterns

❌ **Don't** try to work around non-reactive parameters:

```rust
// BAD: Conditional rendering workaround
{move || {
    view! { <Component id=editing_id.get() /> }
}}
// This recreates component every time, inefficient
```

❌ **Don't** pass `.get()` when you need reactivity:

```rust
// BAD
<Component id=signal.get() />

// GOOD
<Component id=signal />
```

## Related

- Leptos Book: [Passing Signals to Components](https://leptos-rs.github.io/leptos/)
- Project example: `a002_organization` uses similar pattern (native table version)
- Session: [[2025-12-21-session-debrief-a006-signal-sorting]]

## Validation

Tested in `a006_connection_mp`:

- ✅ "New Connection" shows empty form
- ✅ Switching between records shows correct data
- ✅ Form resets properly when going from record to new
