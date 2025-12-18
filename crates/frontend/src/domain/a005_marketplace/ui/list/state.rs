use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct MarketplaceListState {
    pub sort_field: String,
    pub sort_ascending: bool,
}

impl Default for MarketplaceListState {
    fn default() -> Self {
        Self {
            sort_field: "code".to_string(),
            sort_ascending: true,
        }
    }
}

pub fn create_state() -> RwSignal<MarketplaceListState> {
    RwSignal::new(MarketplaceListState::default())
}
