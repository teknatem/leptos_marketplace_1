//! LLM Connection Details UI Module (MVVM Standard)
//!
//! Structure:
//! - model.rs: DTOs and API functions
//! - view_model.rs: LlmConnectionDetailsVm with RwSignals
//! - view.rs: Main component LlmConnectionDetails

mod model;
mod view;
mod view_model;

pub use view::LlmConnectionDetails;
pub use view_model::LlmConnectionDetailsVm;
