#![allow(deprecated)]
//! Converts entity metadata to pivot schema definitions
//!
//! This module provides automatic generation of pivot schemas from
//! the field metadata system (metadata.json -> FieldMetadata).

use contracts::shared::metadata::{EntityMetadataInfo, FieldMetadata, FieldType as MetaFieldType};
use contracts::shared::universal_dashboard::{
    DataSourceSchemaOwned, FieldDefOwned, FieldType as PivotFieldType, ValueType,
};

/// Trait for resolving aggregate references to table names
pub trait RefResolver {
    /// Resolve aggregate index to (table_name, display_column)
    fn resolve_ref(&self, aggregate_index: &str) -> (Option<String>, Option<String>);
}

/// Convert entity metadata to pivot schema
pub fn metadata_to_pivot_schema(
    entity: &EntityMetadataInfo,
    fields: &[FieldMetadata],
    ref_resolver: &impl RefResolver,
) -> DataSourceSchemaOwned {
    let pivot_fields: Vec<FieldDefOwned> = fields
        .iter()
        .filter(|f| should_include_field(f))
        .map(|f| field_to_pivot_def(f, ref_resolver))
        .collect();

    DataSourceSchemaOwned {
        id: entity.entity_index.to_string(),
        name: entity.ui.list_name.to_string(),
        fields: pivot_fields,
    }
}

/// Convert a single field metadata to pivot field definition
fn field_to_pivot_def(field: &FieldMetadata, resolver: &impl RefResolver) -> FieldDefOwned {
    let (pivot_type, can_group, can_aggregate) = map_field_type(field);

    // Resolve reference to another aggregate
    let (ref_table, ref_display) = if field.field_type == MetaFieldType::AggregateRef {
        if let Some(ref_agg) = field.ref_aggregate {
            resolver.resolve_ref(ref_agg)
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    FieldDefOwned {
        id: field.name.to_string(),
        name: field.ui.label.to_string(),
        value_type: ValueType::from_field_type(pivot_type, ref_table.as_deref()),
        can_group,
        can_aggregate,
        db_column: field.name.to_string(),
        ref_display_column: ref_display,
        field_type: Some(pivot_type),
        ref_table,
    }
}

/// Map metadata field type to pivot field type with grouping/aggregation flags
fn map_field_type(field: &FieldMetadata) -> (PivotFieldType, bool, bool) {
    // Check rust_type for numeric types
    match field.rust_type {
        // Numeric types - can aggregate, cannot group
        "f64" | "f32" | "Decimal" => (PivotFieldType::Numeric, false, true),
        "i64" | "i32" | "u64" | "u32" | "usize" | "isize" => (PivotFieldType::Integer, false, true),

        // Date types - can group, cannot aggregate
        "DateTime" | "NaiveDate" | "NaiveDateTime" | "chrono::NaiveDate" | "chrono::DateTime" => {
            (PivotFieldType::Date, true, false)
        }

        // Check for Option<numeric> types
        rt if rt.starts_with("Option<f") => (PivotFieldType::Numeric, false, true),
        rt if rt.starts_with("Option<i") || rt.starts_with("Option<u") => {
            (PivotFieldType::Integer, false, true)
        }
        rt if rt.contains("Date") => (PivotFieldType::Date, true, false),

        // Everything else is text - can group, cannot aggregate
        _ => match field.field_type {
            MetaFieldType::AggregateRef => (PivotFieldType::Text, true, false),
            MetaFieldType::Enum => (PivotFieldType::Text, true, false),
            _ => (PivotFieldType::Text, true, false),
        },
    }
}

/// Determine if a field should be included in pivot schema
fn should_include_field(field: &FieldMetadata) -> bool {
    // Exclude fields not visible in list
    if !field.ui.visible_in_list {
        return false;
    }

    // Exclude system/sensitive fields
    let excluded_names = ["id", "created_at", "updated_at", "is_deleted", "password"];
    if excluded_names.contains(&field.name) {
        return false;
    }

    // Exclude nested types (not supported in pivot)
    if matches!(
        field.field_type,
        MetaFieldType::NestedStruct | MetaFieldType::NestedTable
    ) {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::shared::metadata::{FieldSource, FieldUiMetadata, ValidationRules};

    #[test]
    fn test_map_numeric_field() {
        let field = FieldMetadata {
            name: "amount",
            rust_type: "f64",
            field_type: MetaFieldType::Primitive,
            source: FieldSource::Specific,
            ui: FieldUiMetadata {
                label: "Amount",
                label_en: None,
                placeholder: None,
                hint: None,
                visible_in_list: true,
                visible_in_form: true,
                widget: None,
                column_width: None,
            },
            validation: ValidationRules::default(),
            ai_hint: None,
            nested_fields: None,
            ref_aggregate: None,
            enum_values: None,
        };

        let (field_type, can_group, can_aggregate) = map_field_type(&field);

        assert_eq!(field_type, PivotFieldType::Numeric);
        assert!(!can_group);
        assert!(can_aggregate);
    }

    #[test]
    fn test_map_text_field() {
        let field = FieldMetadata {
            name: "name",
            rust_type: "String",
            field_type: MetaFieldType::Primitive,
            source: FieldSource::Specific,
            ui: FieldUiMetadata {
                label: "Name",
                label_en: None,
                placeholder: None,
                hint: None,
                visible_in_list: true,
                visible_in_form: true,
                widget: None,
                column_width: None,
            },
            validation: ValidationRules::default(),
            ai_hint: None,
            nested_fields: None,
            ref_aggregate: None,
            enum_values: None,
        };

        let (field_type, can_group, can_aggregate) = map_field_type(&field);

        assert_eq!(field_type, PivotFieldType::Text);
        assert!(can_group);
        assert!(!can_aggregate);
    }

    #[test]
    fn test_exclude_password_field() {
        let field = FieldMetadata {
            name: "password",
            rust_type: "String",
            field_type: MetaFieldType::Primitive,
            source: FieldSource::Specific,
            ui: FieldUiMetadata {
                label: "Password",
                label_en: None,
                placeholder: None,
                hint: None,
                visible_in_list: false,
                visible_in_form: true,
                widget: Some("password"),
                column_width: None,
            },
            validation: ValidationRules::default(),
            ai_hint: None,
            nested_fields: None,
            ref_aggregate: None,
            enum_values: None,
        };

        assert!(!should_include_field(&field));
    }
}
