---
title: "RB — Migrate modals to ModalStackService (v1)"
date: 2025-12-27
type: runbook
topics:
  - modals
  - leptos
  - UI
---

## Goal

Migrate any modal/overlay implementation to the unified standard:

- `ModalStackService` + `ModalHost`
- `ModalFrame` container
- “Details render their own header/actions”

## When to use

Apply this runbook when you see any of:

- `shared/modal::Modal`
- `shared/picker_aggregate::ModalService` / `ModalRenderer`
- `modal-overlay` / `modal-content` ad-hoc overlays
- Thaw `Dialog` used to open details screens

## Steps

### 1) Identify the current pattern

Search within the feature module:

- `modal-overlay|modal-content`
- `shared::modal::Modal|<Modal`
- `ModalService|ModalRenderer`
- `Dialog`

### 2) Replace “open details” with `ModalStackService`

In the list/tree component:

1. Get the service:
   - `let modal_stack = use_context::<ModalStackService>().expect("ModalStackService not found in context");`
2. Replace `show_modal` rendering with a call:
   - `modal_stack.push_with_frame(modal_style, modal_class, move |handle| { ... })`
3. Close via `handle.close()` inside callbacks.

Important: the builder passed to `push_with_frame` is `Fn`, so **clone** any captured IDs before moving them into the closure.

### 3) Ensure details are “self-contained”

Details components should include:

- `modal-header` title
- action buttons in header (`Сохранить`, `Тест`, `Закрыть`)
- the rest of the form in `.modal-body` / `.details-container`

`ModalFrame` itself must not add a “Закрыть” button.

### 4) Linked-aggregate modals (details inside details)

If details open other details/pickers:

- Prefer opening via `ModalStackService` directly from the event handler.
- Avoid rendering nested overlay DOM (`modal-overlay`) inside the details.

### 5) Results/confirm dialogs

For operation result dialogs (e.g., “Результаты операции”):

- Open via `ModalStackService`
- Use shared CSS classes for results table (e.g., `results-table`).

### 6) Verify

- `cargo check -p frontend`
- Grep to ensure old patterns are gone from the feature:
  - no `modal-overlay|modal-content`
  - no `ModalService|ModalRenderer`
  - no `shared::modal::Modal`

## Common pitfalls

- WASM closure error on close: use deferred close methods (see KI note).
- Fn-capture move errors: clone the captured value(s) outside the builder closure.


