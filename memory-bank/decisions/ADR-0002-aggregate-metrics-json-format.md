---
title: "ADR0001: Domain aggregate_metrics.json schema and metric selection UI"
date: 2025-12-31
status: accepted
tags:
  - adr
  - metrics
  - json
  - egui
---

## Context

Need a stable file format for per-aggregate metrics used by the UI to render a table for the first-level `domain` node.

An example file in an existing project showed a nested object:
`aggregate -> block -> column -> number`.

## Decision

Use a metric-indexed top-level object to support multiple metric families without changing the inner structure:

- File: `.vsa_designer/aggregate_metrics.json`
- Schema:
  - `metrics.<metricName>.<aggregate>.<block>.<column> = u64`
- Minimum metrics supported:
  - `bytes`
  - `lines`

UI uses a single metric selector to choose which metric to display (one number per cell).

## Rationale

- Preserves the established nesting from the example, but allows adding new metrics without modifying UI table layout logic.
- Keeps the rendering simple (one number in each cell) while still supporting multiple metric families.

## Alternatives considered

1. **Single-metric file (no `metrics` object)**
   - Pros: matches the example exactly
   - Cons: hard to extend; would require new files or schema changes for new metrics
2. **Array-based schema**
   - Pros: explicit objects can be validated easier
   - Cons: more verbose; example already uses object keyed by aggregate name
