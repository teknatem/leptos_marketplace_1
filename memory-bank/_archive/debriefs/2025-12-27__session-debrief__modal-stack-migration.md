---
title: "2025-12-27 — Session Debrief — ModalStack migration + modal styles"
date: 2025-12-27
type: session-debrief
topics:
  - modals
  - UI
  - leptos
  - thaw
status: completed
---

## Summary

This session completed the project-wide migration to a single modal mechanism: **`ModalStackService` + `ModalFrame`** (details render their own header/actions). Legacy modal systems (`shared/modal::Modal` and `picker_aggregate::ModalService/ModalRenderer`) were removed. Phase 3 document/posting screens were migrated, and initial **style cleanup** started for `a009/a010/a011` lists by extracting common CSS classes.

## What was done (high level)

- Implemented and rolled out **modal stack** (stackable modals, Escape closes top-most, overlay click safe).
- Standardized “details” UX: details components render **compact header** (Save/Test/Close) and work identically in tab or modal.
- Migrated remaining areas:
  - Phase 1: a005/a007 lists off legacy `shared/modal::Modal`
  - Phase 2: a003/a004 tree/list ad-hoc overlays to stack
  - Phase 3: a009/a010/a011 list overlays and a012/a013/a015 linked-aggregate overlays to stack
- Fixed nested overlay in `a007_marketplace_product` details (Nomenclature picker) to use modal stack.
- Cleanup: removed unused legacy mechanisms and updated picker docs to reflect stack usage.
- Styling: extracted common classes for doc/posting lists and results modal table; replaced some inline styles in `a009/a010/a011`.

## Main difficulties / uncertainty sources

- **WASM closure lifecycle**: closing modals from within DOM event handlers caused `closure invoked recursively or after being dropped`.
- **Leptos closure trait bounds**: `ModalStackService::push_with_frame` uses `Fn` builder, so captured values must be cloneable and not moved.
- **Legacy modal overlap**: multiple co-existing modal systems (Thaw Dialog, `shared/modal`, picker aggregate modal service, ad-hoc overlays) made migration non-trivial and error-prone.
- **Styling drift**: posting/document lists had heavy inline styling and diverged from Thaw/project CSS variables.

## Resolutions

- Centralized “close” behavior by deferring close operations to the next microtask (see known-issue note).
- Updated modal stack entry representation to avoid `Send + Sync` requirements on `AnyView` by using `signal(...)` and `Rc<dyn Fn(ModalHandle) -> AnyView>`.
- Standardized details: move all action buttons into details header; `ModalFrame` is a pure container with overlay/animation and close mechanics.
- Added reusable CSS classes for doc lists + results table, replacing high-value inline styling first.

## Links to created notes

- [[memory-bank/decisions/ADR0004__modal-stack-standard.md|ADR0004 — ModalStack standard]]
- [[memory-bank/runbooks/RB__modal-migration-to-modalstack__v1.md|RB — Modal migration to ModalStack]]
- [[memory-bank/known-issues/KI-wasm-closure-dropped-on-modal-close-2025-12-27.md|KI — WASM closure dropped on modal close]]
- [[memory-bank/lessons/LL-modalstack-fn-capture-clone-2025-12-27.md|LL — Fn capture + clone rules]]
- [[memory-bank/lessons/LL-doc-list-style-extraction-2025-12-27.md|LL — Doc list style extraction]]

## TODO / open questions

- Finish style cleanup for doc/posting lists (tables/headers) and align with UI reference (bolt-mpi-ui-redesign).
- Confirm visual QA of stacked linked-aggregate modals (e.g., a012 finance report detail) on real data.
- Decide whether to remove `shared/picker_aggregate/modal` references from older feature docs beyond README updates (if any remain).


