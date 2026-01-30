---
type: decision
adr: 8
date: 2025-01-26
status: accepted
topics:
  - sql preview
  - dashboard configuration
  - performance
---

# ADR-0008: SQL Preview On-Demand Generation

## Status

**Accepted** - Implemented in d401_wb_finance dashboard

## Context

Users need to see the generated SQL query for their dashboard configuration for:
- Understanding what data is being queried
- Debugging unexpected results
- Learning SQL for manual queries
- Transparency in report generation

Two approaches considered:
1. **Save SQL in database** with configuration
2. **Generate SQL on-demand** when viewing SQL tab

## Decision

Generate SQL on-demand via dedicated endpoint `/api/d401/generate-sql`.

SQL is **not** saved in the database with configuration.

## Rationale

### Advantages of On-Demand Generation

1. **Configuration Portability**: 
   - Config JSON remains database-agnostic
   - Can be imported/exported without SQL dialect dependencies
   - Schema changes don't invalidate saved configs

2. **Always Current**: 
   - SQL reflects current schema definition
   - No risk of stale SQL from old schema version
   - Query builder improvements automatically benefit all configs

3. **Storage Efficiency**:
   - No SQL text stored in database
   - Smaller config records
   - Less migration overhead

4. **Single Source of Truth**:
   - Query builder is authoritative
   - No risk of SQL/config mismatch
   - Easier to maintain and debug

5. **Performance Acceptable**:
   - SQL generation is fast (<10ms)
   - Only happens when user views SQL tab
   - No impact on main dashboard execution

### Disadvantages (Accepted Trade-offs)

1. **Not Available Offline**: Need backend to see SQL (acceptable - need backend to run query anyway)
2. **Slight Tab Switch Delay**: ~10ms to generate (imperceptible to users)
3. **Can't Index SQL**: Can't search configs by SQL text (use config JSON instead)

## Implementation

### Backend Endpoint
```rust
// POST /api/d401/generate-sql
pub async fn generate_sql(config: DashboardConfig) -> Result<GenerateSqlResponse> {
    let query_builder = QueryBuilder::new(&schema, &config);
    let result = query_builder.build()?;
    
    Ok(GenerateSqlResponse {
        sql: result.sql,
        params: format_params(result.params),
    })
}
```

### Frontend Usage
```rust
on:click=move |_| {
    let cfg = config.get();
    spawn_local(async move {
        if let Ok(resp) = api::generate_sql(cfg).await {
            set_generated_sql.set(Some(resp));
        }
    });
    set_active_tab.set("sql");
}
```

## Alternatives Considered

### Alternative 1: Save SQL in Database
```sql
CREATE TABLE sys_dashboard_configs (
    -- ...
    sql_query TEXT,  -- ← Saved SQL
    sql_generated_at TIMESTAMP
);
```

**Rejected because**:
- SQL becomes stale when schema changes
- Need regeneration logic on schema updates
- Adds complexity to config save/update
- Makes configs less portable

### Alternative 2: Save Both Config and SQL
```json
{
  "config": { /* ... */ },
  "cached_sql": "SELECT ...",
  "sql_version": 1
}
```

**Rejected because**:
- Still have staleness problem
- Added complexity for questionable benefit
- Versioning adds maintenance burden

### Alternative 3: Generate Only on Execute, Cache in Memory
```rust
// Cache SQL after execution
execute_dashboard(config) -> (Response, GeneratedSQL)
```

**Rejected because**:
- Need SQL preview before execution
- Cache invalidation complexity
- Doesn't work across sessions

## Consequences

### Positive
- Clean separation: config storage vs SQL generation
- Schema evolution friendly
- Simpler config save/load logic
- No migration needed for query builder changes

### Negative
- Need backend connection to view SQL
- Can't analyze SQL of saved configs in bulk
- Slight delay on SQL tab switch (acceptable)

### Neutral
- SQL tab must call separate API endpoint
- Frontend needs to handle loading state

## Related Decisions

- ADR-0007: Dashboard configuration in JSON (enabled this decision)
- Future: SQL export/copy functionality (benefits from on-demand generation)

## Follow-up Actions

- [x] Implement backend endpoint
- [x] Implement frontend API client
- [x] Create SqlViewer component
- [x] Add SQL tab to dashboard
- [ ] Consider adding "Copy SQL" button (future enhancement)
- [ ] Consider SQL parameter substitution view (future enhancement)

## Review Date

2025-07-26 (6 months) - Assess if on-demand generation continues to meet needs
