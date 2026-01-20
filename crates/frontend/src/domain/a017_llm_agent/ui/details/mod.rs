//! LLM Agent Details UI Module (MVVM Standard)
//!
//! Structure:
//! - model.rs: DTOs and API functions
//! - view_model.rs: LlmAgentDetailsVm with RwSignals
//! - view.rs: Main component LlmAgentDetails

mod model;
mod view;
mod view_model;

pub use view::LlmAgentDetails;
pub use view_model::LlmAgentDetailsVm;
