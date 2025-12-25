---
type: lesson
topic: Leptos 0.8 Signal and RwSignal Patterns
date: 2025-12-23
---
# Lesson: Leptos 0.8 RwSignal and Type Inference

## Context
During the implementation of `ScheduledTaskDetails`, we encountered issues with passing `RwSignal` to components expecting `Signal<String>`.

## Lesson
- **Deprecations**: `create_signal` is now `signal()`, and `create_rw_signal` is `RwSignal::new()`.
- **Type Compatibility**: Components like `Input` or `Textarea` that take `Signal<String>` or `Into<Signal<String>>` often fail to infer types correctly from a raw `RwSignal`.
- **Solution**: Use `Signal::derive(move || rw_signal.get())` to explicitly create a `Signal` from an `RwSignal`. This avoids type inference errors like `cannot satisfy _: Into<leptos::prelude::Signal<std::string::String>>`.
- **Closure Ownership**: When using a single `id` (String) in multiple Leptos closures (button clicks, effects), always clone it *outside* the closure for each use case:
  ```rust
  let id_for_save = id.clone();
  let save_task = move |_| { let task_id = id_for_save.clone(); ... };
  ```


