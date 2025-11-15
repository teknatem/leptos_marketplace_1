//! Nomenclature Details UI Module
//!
//! Simplified MVVM pattern implementation:
//! - model.rs: API functions (fetch, save)
//! - view_model.rs: ViewModel with commands and state management
//! - view.rs: Leptos component (pure UI)
//! - dimension_input.rs: Custom dimension input with dropdown

mod dimension_input;
mod model;
mod view;
mod view_model;

pub use dimension_input::DimensionInput;
pub use view::NomenclatureDetails;
pub use view_model::NomenclatureDetailsViewModel;
