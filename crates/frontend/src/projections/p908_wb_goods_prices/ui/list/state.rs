use leptos::prelude::*;
use serde::{Deserialize, Serialize};

const STORAGE_KEY: &str = "p908_wb_goods_prices_list_state_v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersistedState {
    pub connection_filter: String,
    pub vendor_code_filter: String,
    pub search_filter: String,
    pub sort_by: String,
    pub sort_ascending: bool,
    pub page: usize,
    pub page_size: usize,
}

#[derive(Clone, Debug)]
pub struct P908ListState {
    // Filters
    pub connection_filter: String,
    pub vendor_code_filter: String,
    pub search_filter: String,

    // Sorting
    pub sort_by: String,
    pub sort_ascending: bool,

    // Pagination
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,

    // Load flag
    pub is_loaded: bool,
}

impl Default for P908ListState {
    fn default() -> Self {
        Self {
            connection_filter: String::new(),
            vendor_code_filter: String::new(),
            search_filter: String::new(),
            sort_by: "nm_id".to_string(),
            sort_ascending: false,
            page: 0,
            page_size: 100,
            total_count: 0,
            total_pages: 0,
            is_loaded: false,
        }
    }
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window().and_then(|w| w.local_storage().ok().flatten())
}

fn load_persisted() -> Option<PersistedState> {
    let raw = storage()?.get_item(STORAGE_KEY).ok().flatten()?;
    serde_json::from_str::<PersistedState>(&raw).ok()
}

fn save_persisted(st: &PersistedState) {
    let Some(storage) = storage() else { return };
    let Ok(raw) = serde_json::to_string(st) else {
        return;
    };
    let _ = storage.set_item(STORAGE_KEY, &raw);
}

pub fn persist_state(signal: RwSignal<P908ListState>) {
    let st = signal.get_untracked();
    let persisted = PersistedState {
        connection_filter: st.connection_filter,
        vendor_code_filter: st.vendor_code_filter,
        search_filter: st.search_filter,
        sort_by: st.sort_by,
        sort_ascending: st.sort_ascending,
        page: st.page,
        page_size: st.page_size,
    };
    save_persisted(&persisted);
}

pub fn create_state() -> RwSignal<P908ListState> {
    let mut st = P908ListState::default();
    if let Some(p) = load_persisted() {
        st.connection_filter = p.connection_filter;
        st.vendor_code_filter = p.vendor_code_filter;
        st.search_filter = p.search_filter;
        st.sort_by = p.sort_by;
        st.sort_ascending = p.sort_ascending;
        st.page = p.page;
        st.page_size = p.page_size;
    }
    RwSignal::new(st)
}
