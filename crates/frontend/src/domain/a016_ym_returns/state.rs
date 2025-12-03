use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct YmReturnsState {
    pub sort_field: String,
    pub sort_ascending: bool,
    pub is_loaded: bool,
}

impl Default for YmReturnsState {
    fn default() -> Self {
        Self {
            sort_field: "return_id".to_string(),
            sort_ascending: false,
            is_loaded: false,
        }
    }
}

// Create state within component scope
pub fn create_state() -> RwSignal<YmReturnsState> {
    RwSignal::new(YmReturnsState::default())
}

