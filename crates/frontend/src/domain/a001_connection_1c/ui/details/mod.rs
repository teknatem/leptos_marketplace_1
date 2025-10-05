//! Connection1C Details UI Module
//! 
//! Simplified MVVM pattern implementation:
//! - model.rs: API functions (fetch, save, test)
//! - view_model.rs: ViewModel with commands and state management
//! - view.rs: Leptos component (pure UI)

mod model;
mod view;
mod view_model;

pub use view::Connection1CDetails;
pub use view_model::Connection1CDetailsViewModel;
