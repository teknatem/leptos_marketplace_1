use contracts::system::users::User;
use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct UsersListState {
    pub items: Vec<User>,
    pub search_query: String,
    pub sort_field: String,
    pub sort_ascending: bool,
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
    pub is_loaded: bool,
}

impl Default for UsersListState {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            search_query: String::new(),
            sort_field: "username".to_string(),
            sort_ascending: true,
            page: 0,
            page_size: 50,
            total_count: 0,
            total_pages: 1,
            is_loaded: false,
        }
    }
}

pub fn create_state() -> RwSignal<UsersListState> {
    RwSignal::new(UsersListState::default())
}
