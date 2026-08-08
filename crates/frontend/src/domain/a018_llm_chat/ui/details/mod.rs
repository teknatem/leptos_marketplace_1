//! LLM Chat Details UI Module (MVVM Standard)
//!
//! Structure:
//! - model.rs: DTOs and API functions
//! - view_model.rs: LlmChatDetailsVm with RwSignals
//! - view.rs: Main component LlmChatDetails
//! - artifact_card.rs: Component for displaying artifact cards

mod artifact_card;
mod model;
mod prefs;
mod questions_bar;
mod settings_dialog;
mod tool_calls_trace;
mod view;
mod view_model;
mod workspace_drawer;

pub use artifact_card::ArtifactCard;
pub use tool_calls_trace::ToolCallsTrace;
pub use view::LlmChatDetails;
pub use view_model::LlmChatDetailsVm;
