//! LLM Chat Details UI Module (MVVM Standard)
//!
//! Structure:
//! - model.rs: DTOs and API functions
//! - view_model.rs: LlmChatDetailsVm with RwSignals
//! - view.rs: Main component LlmChatDetails
//! - artifact_card.rs: Component for displaying artifact cards

mod artifact_card;
mod model;
mod view;
mod view_model;

pub use artifact_card::ArtifactCard;
pub use view::LlmChatDetails;
pub use view_model::LlmChatDetailsVm;
