use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct NomenclatureListState {
    // filters
    pub q: String,
    pub only_mp: bool,

    // server sorting
    pub sort_field: String,
    pub sort_ascending: bool,

    // selection
    pub selected_ids: Vec<String>,

    // load flag
    pub is_loaded: bool,

    // pagination
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
}

impl Default for NomenclatureListState {
    fn default() -> Self {
        Self {
            q: String::new(),
            only_mp: true,
            sort_field: "article".to_string(),
            sort_ascending: true,
            selected_ids: Vec::new(),
            is_loaded: false,
            page: 0,
            page_size: 100,
            total_count: 0,
            total_pages: 0,
        }
    }
}

pub fn create_state() -> RwSignal<NomenclatureListState> {
    RwSignal::new(NomenclatureListState::default())
}


