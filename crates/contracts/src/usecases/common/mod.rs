//! Common types and traits for all UseCases

pub mod usecase_metadata;
pub mod usecase_result;

// Re-exports
pub use usecase_metadata::UseCaseMetadata;
pub use usecase_result::{UseCaseResult, UseCaseError};
