---
type: lesson-learned
date: 2025-01-27
tags: [refactoring, schema, pivot, migration]
context: Migrating from hardcoded P903_SCHEMA to dynamic SchemaRegistry
---

# Lesson: Schema Registry Migration Requires Full Service Layer Update

## Context

When migrating from a hardcoded schema (`P903_SCHEMA`) to a dynamic `SchemaRegistry` system, the following components were updated:
- Schema definitions (new `s001_wb_finance`)
- Registry (lists all schemas)
- Handlers (use registry)

But service layer was forgotten.

## Lesson

**All service functions that reference the old schema ID must be updated** when migrating to a registry-based system.

### Checklist for Schema Migration

1. [ ] Schema definition created
2. [ ] Schema registered in `SchemaRegistry`
3. [ ] API handlers updated to use registry
4. [ ] **Service functions updated** - often missed!
   - `execute_dashboard()` - validation check
   - `generate_sql()` - validation check
   - `get_distinct_values()` - table name lookup
5. [ ] Function signatures updated (add `schema_id` parameter where needed)

## Pattern: Temporary Dual Support

For backward compatibility during migration:

```rust
let schema = if config.data_source == P903_SCHEMA.id 
    || config.data_source == "s001_wb_finance" {
    &P903_SCHEMA
} else {
    return Err(anyhow::anyhow!("Unsupported data source"));
};
```

## Better Pattern: Full Dynamic Lookup

```rust
let registry = get_registry();
let schema = registry.get_schema(&config.data_source)
    .ok_or_else(|| anyhow::anyhow!("Unknown schema"))?;
let table_name = registry.get_table_name(&config.data_source)
    .ok_or_else(|| anyhow::anyhow!("Table not found"))?;
```

## Impact of Missing This

- Frontend gets "EOF while parsing" JSON error
- Backend returns 500 without useful error message
- Hard to debug because error appears to be JSON-related
