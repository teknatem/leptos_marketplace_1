pub mod entity_registry;
pub mod metadata_converter;
pub mod query_builder;
pub mod schema_validator;
pub mod schemas;
pub mod tree_builder;

// Legacy alias for compatibility
pub mod schema_registry {
    pub use super::entity_registry::*;
}

pub use entity_registry::*;
pub use query_builder::*;
pub use tree_builder::*;
