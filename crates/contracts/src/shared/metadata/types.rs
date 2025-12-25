//! Core metadata types for aggregates, usecases, and projections
//!
//! All types use 'static lifetimes for zero-cost compile-time constants.

use super::field_type::{FieldType, FieldSource};
use super::validation::ValidationRules;

// ============================================================================
// Entity-level metadata
// ============================================================================

/// Metadata for an entity (aggregate, usecase, projection)
/// All string fields are 'static for zero-cost compile-time access
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityMetadataInfo {
    pub schema_version: &'static str,
    pub entity_type: EntityType,
    pub entity_name: &'static str,
    pub entity_index: &'static str,
    pub collection_name: &'static str,
    pub table_name: Option<&'static str>,
    pub ui: EntityUiMetadata,
    pub ai: EntityAiMetadata,
}

/// Type of entity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    Aggregate,
    UseCase,
    Projection,
}

impl EntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Aggregate => "aggregate",
            Self::UseCase => "usecase",
            Self::Projection => "projection",
        }
    }
}

/// UI metadata for entity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityUiMetadata {
    pub element_name: &'static str,
    pub element_name_en: Option<&'static str>,
    pub list_name: &'static str,
    pub list_name_en: Option<&'static str>,
    pub icon: Option<&'static str>,
}

/// AI/LLM context metadata for entity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityAiMetadata {
    /// Business description for LLM context
    pub description: &'static str,
    /// Typical questions this entity can answer
    pub questions: &'static [&'static str],
    /// Related entities (aggregates, usecases, projections)
    pub related: &'static [&'static str],
}

// ============================================================================
// Field-level metadata
// ============================================================================

/// Metadata for a single field
/// Copy trait enabled for efficient passing by value
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldMetadata {
    pub name: &'static str,
    pub rust_type: &'static str,
    pub field_type: FieldType,
    pub source: FieldSource,
    pub ui: FieldUiMetadata,
    pub validation: ValidationRules,
    pub ai_hint: Option<&'static str>,
    
    // For nested types (recursive reference via static slice)
    pub nested_fields: Option<&'static [FieldMetadata]>,
    pub ref_aggregate: Option<&'static str>,
    pub enum_values: Option<&'static [&'static str]>,
}

impl FieldMetadata {
    /// Get nested fields metadata (for NestedStruct/NestedTable)
    pub fn nested(&self) -> Option<&'static [FieldMetadata]> {
        self.nested_fields
    }

    /// Check if field is optional
    pub fn is_optional(&self) -> bool {
        !self.validation.required
    }

    /// Get referenced aggregate index (for AggregateRef)
    pub fn referenced_aggregate(&self) -> Option<&'static str> {
        self.ref_aggregate
    }

    /// Check if field should be visible in list view
    pub fn visible_in_list(&self) -> bool {
        self.ui.visible_in_list
    }

    /// Check if field should be visible in form
    pub fn visible_in_form(&self) -> bool {
        self.ui.visible_in_form
    }
}

/// UI metadata for a field
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldUiMetadata {
    pub label: &'static str,
    pub label_en: Option<&'static str>,
    pub placeholder: Option<&'static str>,
    pub hint: Option<&'static str>,
    pub visible_in_list: bool,
    pub visible_in_form: bool,
    pub widget: Option<&'static str>,
    pub column_width: Option<u32>,
}

/// Default values for FieldUiMetadata
impl Default for FieldUiMetadata {
    fn default() -> Self {
        Self {
            label: "",
            label_en: None,
            placeholder: None,
            hint: None,
            visible_in_list: true,
            visible_in_form: true,
            widget: None,
            column_width: None,
        }
    }
}

