use leptos::prelude::*;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct ConnectionMPListState {
    pub selected: RwSignal<HashSet<String>>,
    pub sort_field: RwSignal<String>,
    pub sort_ascending: RwSignal<bool>,
}

pub fn create_state() -> ConnectionMPListState {
    ConnectionMPListState {
        selected: RwSignal::new(HashSet::new()),
        sort_field: RwSignal::new("description".to_string()),
        sort_ascending: RwSignal::new(true),
    }
}
