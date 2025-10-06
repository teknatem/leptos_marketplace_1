//! Counterparty Details UI Module
//!
//! Simplified MVVM pattern implementation:
//! - model.rs: API functions (fetch, save)
//! - view_model.rs: ViewModel with commands and state management
//! - view.rs: Leptos component (pure UI)

mod model;
mod view;
mod view_model;

pub use view::CounterpartyDetails;
pub use view_model::CounterpartyDetailsViewModel;
