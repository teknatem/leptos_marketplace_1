---
title: "LL — Extract doc/posting list inline styles into shared CSS classes"
date: 2025-12-27
type: lesson
topics:
  - UI
  - CSS
  - thaw
  - consistency
---

## Lesson

Document/posting list screens (`a009/a010/a011`) accumulated many inline styles (dozens per file). Converting the highest-impact layout blocks (toolbar, filters, inputs, summary, results table) into shared CSS classes in `static/themes/core/components.css` improved consistency and reduced future drift.

## What to extract first (works well)

- Toolbar header row: title + refresh button
- Filters panel container and rows
- Date inputs / status select sizing (height 30px, padding 5px 12px)
- Progress text + summary line
- “operation results” modal table

## Resulting shared classes (created in session)

- `doc-list__toolbar`, `doc-list__title`
- `doc-filters`, `doc-filters__row`, `doc-filter`, `doc-filter__label`, `doc-filter__input`, `doc-filter__select`
- `doc-list__progress`, `doc-list__summary`
- `results-table`, `results-table__id`

## Follow-up

There is still more inline styling inside table cells/headers in those screens; extract gradually to avoid over-large patches.


