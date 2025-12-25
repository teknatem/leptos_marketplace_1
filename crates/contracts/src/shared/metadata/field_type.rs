//! Field type enumeration for metadata system

/// Category of field type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FieldType {
    #[default]
    Primitive,      // String, i32, f64, bool, DateTime
    Enum,           // Rust enum with variants
    AggregateRef,   // Reference to another aggregate by ID
    NestedStruct,   // Embedded struct (not Vec)
    NestedTable,    // Vec<T> of embedded structs
}

impl FieldType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Primitive => "primitive",
            Self::Enum => "enum",
            Self::AggregateRef => "aggregate_ref",
            Self::NestedStruct => "nested_struct",
            Self::NestedTable => "nested_table",
        }
    }
}

/// Source of field in the aggregate structure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FieldSource {
    #[default]
    Specific,  // Field specific to this aggregate
    Base,      // Field from BaseAggregate (id, code, description, comment)
    Metadata,  // Field from EntityMetadata (created_at, updated_at, etc.)
}

impl FieldSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Specific => "specific",
            Self::Base => "base",
            Self::Metadata => "metadata",
        }
    }
}

