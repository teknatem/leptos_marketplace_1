---
title: "KI — WASM closure invoked recursively or after being dropped (modal close)"
date: 2025-12-27
type: known-issue
topics:
  - leptos
  - wasm
  - modals
severity: high
---

## Symptom

Browser console error during modal close (overlay click / close button / Escape):

- `Uncaught Error: closure invoked recursively or after being dropped`

## Root cause (established in session)

Closing/removing modal components synchronously during the same DOM event can drop a closure that is still on the call stack.

## Detection

- Repro by opening any modal and closing it quickly via overlay click or header close.
- Console shows the error immediately on close.

## Fix

Defer close operations to the next microtask:

- Use `wasm_bindgen_futures::spawn_local` + `gloo_timers::future::TimeoutFuture::new(0).await`
- Apply consistently:
  - `ModalFrame` overlay close callback
  - `ModalStackService` close/pop operations (e.g., `close_deferred`, `pop_deferred`)
  - `ModalHandle::close()` should call a deferred close

## Notes

This fix was applied globally as part of the modal stack migration.


