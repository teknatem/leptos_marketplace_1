---
type: lesson
date: 2025-01-26
topics:
  - database patterns
  - ref fields
  - foreign keys
  - display values
category: architecture
---

# Lesson: Ref Field Display Pattern

## Context

When displaying data with foreign key references (UUIDs), users need human-readable descriptions, not raw IDs.

**Example**: Show "Wildberries" instead of "abc-123-def-456" for marketplace connection.

## The Pattern

### Schema Definition (Contracts)
```rust
FieldDef {
    id: "connection_mp_ref",
    name: "Marketplace Connection",
    db_column: "connection_mp_ref",
    field_type: FieldType::Text,
    can_group: true,
    can_aggregate: false,
    ref_table: Some("a006_connection_mp"),      // ← Foreign table
    ref_display_column: Some("description"),     // ← Display field
}
```

### Query Builder (Backend)
Automatically generates JOINs and selects display columns:

```sql
SELECT 
  main_table.connection_mp_ref,
  a006_connection_mp.description AS connection_mp_ref_display,  -- ← Display alias
  SUM(main_table.amount)
FROM main_table
LEFT JOIN a006_connection_mp 
  ON main_table.connection_mp_ref = a006_connection_mp.id
GROUP BY main_table.connection_mp_ref, a006_connection_mp.description
```

**Key Points**:
- JOIN happens automatically when `ref_table` is present
- Display column uses `{field_id}_display` naming convention
- LEFT JOIN ensures null refs don't break query
- Both raw ref and display are in GROUP BY for correct grouping

### Service Layer (Backend)
Parse results preferring display values:

```rust
if field.ref_table.is_some() {
    let display_col = format!("{}_display", field_id);
    if let Ok(Some(display)) = query_result.try_get::<Option<String>>("", &display_col) {
        CellValue::Text(display)  // ← Use display value
    } else {
        // Fallback to raw ref value
        query_result.try_get::<Option<String>>("", field_id)
            .ok()
            .flatten()
            .map(CellValue::Text)
            .unwrap_or(CellValue::Null)
    }
} else {
    // Non-ref field: use directly
}
```

### Frontend (Display)
No special handling needed - receives display values transparently.

## Benefits

1. **Separation of Concerns**: Display logic in backend, UI just renders
2. **Consistency**: All ref fields follow same pattern
3. **Performance**: Single query with JOIN (no N+1 problem)
4. **Flexibility**: Schema-driven (works for any ref field)
5. **Safety**: Fallback to raw value if display unavailable

## When to Use

✅ **Use this pattern for**:
- Foreign key references
- Lookup tables (organizations, marketplaces, categories)
- User-facing reports and dashboards
- Any UUID → human-readable mapping

❌ **Don't use for**:
- Internal technical displays where UUID is acceptable
- APIs where caller wants raw IDs
- Performance-critical queries where JOIN is expensive

## Implementation Checklist

- [ ] Add `ref_table` and `ref_display_column` to schema FieldDef
- [ ] Query builder automatically adds LEFT JOIN (verify in generated SQL)
- [ ] Service layer checks for `{field_id}_display` columns
- [ ] Test with null refs (LEFT JOIN should handle gracefully)
- [ ] Verify display values appear in frontend

## Common Mistakes

### ❌ Missing Display Column in Schema
```rust
ref_table: Some("a006_connection_mp"),
ref_display_column: None,  // ← Forgot this!
```
**Result**: No JOIN happens, raw UUIDs displayed.

### ❌ Wrong Display Column Name
```rust
ref_display_column: Some("name"),  // But table has "description"
```
**Result**: SQL error or null values.

### ❌ Not Checking for _display in Service
```rust
// Always uses raw ref, ignoring display
query_result.try_get::<Option<String>>("", field_id)
```
**Result**: UUIDs displayed even though display values are in result set.

### ❌ INNER JOIN Instead of LEFT JOIN
```rust
.join(ref_table)  // ← INNER JOIN
```
**Result**: Rows with null refs disappear from results.

## Related Patterns

- **Lazy Loading Display Values**: For filter dropdowns, fetch distinct values with displays
- **Cascade Display**: Multi-level refs (e.g., connection → marketplace → platform)
- **Composite Display**: Combining multiple fields (e.g., "Wildberries (Test Mode)")

## Real-World Example

**p903_wb_finance_report table**:
- `connection_mp_ref` (UUID) → `a006_connection_mp.description` ("Wildberries WB-001")
- `organization_ref` (UUID) → `a002_organization.description` ("ООО Компания")

**User sees**:
```
Marketplace          | Amount
---------------------|--------
Wildberries WB-001   | 15,234
Ozon OZ-002          |  8,456
```

**Not**:
```
Marketplace                              | Amount
-----------------------------------------|--------
a1b2c3d4-e5f6-7890-abcd-ef1234567890    | 15,234
f0e9d8c7-b6a5-4321-09ab-cdef87654321    |  8,456
```

## Testing

```rust
#[test]
fn test_ref_field_display() {
    // Given: Field with ref_table
    let field = FieldDef {
        id: "connection_mp_ref",
        ref_table: Some("a006_connection_mp"),
        ref_display_column: Some("description"),
        // ...
    };
    
    // When: Query executed
    let sql = query_builder.build(&config);
    
    // Then: Should include display column
    assert!(sql.contains("AS connection_mp_ref_display"));
    assert!(sql.contains("LEFT JOIN a006_connection_mp"));
}
```
