---
title: "ADR0004 — Modal standard: ModalStackService + ModalFrame"
date: 2025-12-27
type: decision
status: accepted
drivers:
  - Need a single consistent modal mechanism across the frontend
  - Need support for sequential / stacked modals
  - Avoid double headers (modal header + details header)
---

## Context

The codebase had multiple modal mechanisms in parallel:

- Thaw dialogs
- `shared/modal::Modal` (legacy)
- ad-hoc `modal-overlay/modal-content` overlays in various features
- `shared/picker_aggregate::ModalService/ModalRenderer` for some pickers

This caused inconsistent UX, duplicated headers/actions, and made it hard to support stacked modals reliably.

## Decision

Adopt **one standard** for modals:

- **Container**: `ModalFrame` (overlay + sizing + z-index + animation + close mechanics; no header/actions)
- **Stack manager**: `ModalStackService` + `ModalHost` (global stack, push/pop/close, Escape closes top-most)
- **Content**: details/pickers render their own **compact header + actions** so they look identical in a modal or tab.

Close behavior is deferred (next microtask) to avoid WASM closure lifecycle issues.

## Consequences

### Positive

- Consistent UX and styling for all modals.
- Supports stacked modals (pickers + linked details).
- Removes duplicated modal headers and wasted vertical space.

### Negative / tradeoffs

- Migration effort: must convert all existing modal usage to the new stack.
- Some features using old picker modal service need to be updated and/or removed.

## Alternatives considered

- **Use Thaw dialogs everywhere**: rejected due to stack requirements and desire for “details self-contained header” standard.
- **Keep multiple systems**: rejected due to inconsistent UX and recurring bugs.

## Implementation notes (established in session)

- `ModalFrame` overlay close should be “direct click only” (avoid drag-release closing).
- All close operations should be deferred (see KI note).


