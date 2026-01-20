//! LLM Artifact Details UI Module (MVVM Standard)
//!
//! Structure:
//! - model.rs: DTOs and API functions
//! - view_model.rs: LlmArtifactDetailsVm with RwSignals
//! - view.rs: Main component LlmArtifactDetails

mod model;
mod view;
mod view_model;

pub use view::LlmArtifactDetails;
pub use view_model::LlmArtifactDetailsVm;
