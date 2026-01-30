---
type: runbook
version: 1
date: 2025-01-26
topics:
  - pivot dashboard
  - d401 architecture
  - data flow
applies-to:
  - d401_wb_finance
  - future pivot dashboards
---

# Runbook: Pivot Dashboard Pattern

## Overview

Pattern for creating OLAP-style pivot dashboards with grouping, aggregation, and filtering capabilities.

## Architecture Components

### 1. Contracts Layer
**Location**: `crates/contracts/src/shared/pivot/`

**Key Types**:
- `DashboardConfig` - User configuration (groupings, measures, filters, display_fields)
- `FieldRole` - None, Grouping, Measure, Display
- `DataSourceSchema` / `FieldDef` - Schema definition with field metadata
- `ExecuteDashboardResponse` - Hierarchical results (columns + pivot rows)
- `CellValue` - Text, Number, Integer, Null

### 2. Backend Schema Definition
**Location**: `crates/backend/src/dashboards/dNNN_*/schema.rs`

**Pattern**:
```rust
pub static SCHEMA: DataSourceSchema = DataSourceSchema {
    id: "table_name",
    name: "Display Name",
    table: "database_table",
    fields: &[
        FieldDef {
            id: "field_id",
            name: "Field Display Name",
            db_column: "db_column_name",
            field_type: FieldType::Text,
            can_group: true,
            can_aggregate: false,
            ref_table: Some("referenced_table"),
            ref_display_column: Some("description"),
        },
        // ... more fields
    ],
};
```

### 3. Query Builder (SQL Generation)
**Location**: `crates/backend/src/shared/pivot/query_builder.rs`

**Responsibilities**:
- Generate SELECT with groupings and aggregations
- Add LEFT JOINs for ref fields
- Select both `field` and `field_display` for refs
- Apply WHERE filters
- Add GROUP BY and ORDER BY clauses

**Key Pattern**:
```sql
SELECT 
  main_table.grouping_field,
  ref_table.description AS grouping_field_display,
  SUM(main_table.measure_field)
FROM main_table
LEFT JOIN ref_table ON main_table.grouping_field = ref_table.id
WHERE conditions
GROUP BY main_table.grouping_field, ref_table.description
ORDER BY ref_table.description
```

### 4. Tree Builder (Hierarchical Transformation)
**Location**: `crates/backend/src/shared/pivot/tree_builder.rs`

**Responsibilities**:
- Convert flat SQL results to hierarchical PivotRow tree
- Calculate subtotals at each grouping level
- Support multi-level grouping

### 5. Service Layer
**Location**: `crates/backend/src/dashboards/dNNN_*/service.rs`

**Key Functions**:
- `execute_dashboard(config)` - Main execution flow
- `generate_sql(config)` - SQL preview generation
- `get_distinct_values(field)` - Filter value lists

**Data Flow**:
```
1. Validate config
2. QueryBuilder::build() → SQL + params
3. Execute SQL → Vec<QueryResult>
4. Parse results → Vec<RawRow>
5. TreeBuilder::build() → Vec<PivotRow>
6. Create column headers
7. Return ExecuteDashboardResponse
```

**Ref Field Pattern**:
```rust
// Try _display column first for ref fields
if field.ref_table.is_some() {
    let display_col = format!("{}_display", field_id);
    if let Ok(Some(display)) = result.try_get(&display_col) {
        CellValue::Text(display)
    } else {
        // fallback to raw value
    }
}
```

### 6. Frontend Components
**Location**: `crates/frontend/src/shared/pivot/`

**Components**:
- `SettingsTable` - Field configuration UI
- `PivotTable` - Results display
- `SqlViewer` - SQL preview
- `SavedConfigsList` - Saved configurations

**Location**: `crates/frontend/src/dashboards/dNNN_*/`

**Structure**:
- `api.rs` - HTTP client functions
- `ui/dashboard.rs` - Main dashboard component with tabs

## Adding a New Dashboard

### Step 1: Define Schema
Create `crates/backend/src/dashboards/dNNN_<name>/schema.rs`:
```rust
use crate::shared::pivot::schema::*;

pub static SCHEMA: DataSourceSchema = DataSourceSchema {
    // ... definition
};
```

### Step 2: Create Service
Create `crates/backend/src/dashboards/dNNN_<name>/service.rs`:
```rust
pub async fn execute_dashboard(config: DashboardConfig) -> Result<ExecuteDashboardResponse> {
    // Validate data source
    // Build and execute query
    // Parse results
    // Build tree
    // Return response
}
```

### Step 3: Add API Handlers
In `crates/backend/src/api/handlers/dNNN_<name>.rs`:
- `execute_dashboard`
- `generate_sql`
- `get_distinct_values`
- Config CRUD handlers

### Step 4: Register Routes
In `crates/backend/src/api/routes.rs`:
```rust
.route("/api/dNNN/execute", post(handlers::dNNN::execute_dashboard))
.route("/api/dNNN/generate-sql", post(handlers::dNNN::generate_sql))
// ... other routes
```

### Step 5: Create Frontend
- `crates/frontend/src/dashboards/dNNN_<name>/api.rs` - API client
- `crates/frontend/src/dashboards/dNNN_<name>/ui/dashboard.rs` - Component
- `crates/frontend/static/dashboards/dNNN.css` - Styles

### Step 6: Register Frontend Route
Add to frontend routing configuration.

## Best Practices

1. **Always use ref_table/ref_display_column** for foreign keys
2. **Query Builder handles JOIN logic** - don't duplicate in service
3. **Use StoredValue** for multi-use reactive closures in Leptos
4. **Theme-aware styles**: Use CSS variables (var(--surface), var(--border))
5. **Lazy load distinct values** for filter dropdowns (on focus)
6. **Preserve sort order** in display_fields, groupings for user control

## Common Pitfalls

1. Forgetting to add `display_fields` initialization in frontend config
2. Not checking for `_display` columns when parsing ref field results
3. Moving closures in Leptos reactive contexts (use StoredValue)
4. Hardcoding colors instead of CSS theme variables
5. Not adding `#[serde(default)]` for new optional config fields
