---
title: "LL — ModalStackService builder is Fn: clone captures (Leptos)"
date: 2025-12-27
type: lesson
topics:
  - leptos
  - rust
  - closures
  - modals
---

## Lesson

`ModalStackService::push_with_frame` takes a builder closure that must be `Fn(...)`, not `FnOnce`. That means captured values **cannot be moved/consumed** inside the builder.

## Symptom

Compiler error like:

- `cannot move out of 'id', a captured variable in an 'Fn' closure`

## Fix pattern

Clone captured values before the `move |handle| { ... }` builder closure, and then clone again as needed inside view props.

Example pattern:

- `let id_val = id.clone();`
- inside builder: `id=id_val.clone()`

## Where it happened in session

This occurred when migrating list screens to `ModalStackService` and passing `Option<String>` / `String` IDs into details components.


