//! Common types and traits for all aggregates

pub mod origin;
pub mod entity_metadata;
pub mod event_store;
pub mod base_aggregate;
pub mod aggregate_root;
pub mod aggregate_id;

// Re-exports
pub use origin::Origin;
pub use entity_metadata::EntityMetadata;
pub use event_store::EventStore;
pub use base_aggregate::BaseAggregate;
pub use aggregate_root::AggregateRoot;
pub use aggregate_id::AggregateId;
