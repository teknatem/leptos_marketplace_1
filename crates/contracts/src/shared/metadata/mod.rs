//! Metadata types for describing aggregates, usecases, and projections
//!
//! This module provides compile-time metadata for all entities in the system.
//! All types use 'static lifetimes for zero-cost access to compile-time constants.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use contracts::shared::metadata::{EntityMetadataInfo, FieldMetadata};
//!
//! // Access entity metadata from aggregate
//! let meta = Connection1CDatabase::entity_metadata_info();
//! println!("Entity: {}", meta.ui.element_name);
//!
//! // Iterate over fields
//! for field in Connection1CDatabase::field_metadata() {
//!     println!("{}: {}", field.name, field.ui.label);
//! }
//! ```

mod types;
mod field_type;
mod validation;

pub use types::{
    EntityMetadataInfo,
    EntityType,
    EntityUiMetadata,
    EntityAiMetadata,
    FieldMetadata,
    FieldUiMetadata,
};
pub use field_type::{FieldType, FieldSource};
pub use validation::ValidationRules;

