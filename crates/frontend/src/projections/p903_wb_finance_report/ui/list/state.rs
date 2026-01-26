use leptos::prelude::*;
use serde::{Deserialize, Serialize};

const STORAGE_KEY: &str = "p903_wb_finance_report_list_state_v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersistedState {
    pub date_from: String,
    pub date_to: String,
    pub nm_id_filter: String,
    pub sa_name_filter: String,
    pub connection_filter: String,
    pub operation_filter: String,
    pub srid_filter: String,
    pub sort_by: String,
    pub sort_ascending: bool,
    pub page: usize,
    pub page_size: usize,
}

#[derive(Clone, Debug)]
pub struct P903ListState {
    // Filters
    pub date_from: String,
    pub date_to: String,
    pub nm_id_filter: String,
    pub sa_name_filter: String,
    pub connection_filter: String,
    pub operation_filter: String,
    pub srid_filter: String,
    
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

impl Default for P903ListState {
    fn default() -> Self {
        let now = chrono::Utc::now().date_naive();
        let default_start = now - chrono::Duration::days(30);
        let default_end = now;
        
        Self {
            date_from: default_start.format("%Y-%m-%d").to_string(),
            date_to: default_end.format("%Y-%m-%d").to_string(),
            nm_id_filter: String::new(),
            sa_name_filter: String::new(),
            connection_filter: String::new(),
            operation_filter: String::new(),
            srid_filter: String::new(),
            sort_by: "rr_dt".to_string(),
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
    let Ok(raw) = serde_json::to_string(st) else { return };
    let _ = storage.set_item(STORAGE_KEY, &raw);
}

pub fn persist_state(signal: RwSignal<P903ListState>) {
    let st = signal.get_untracked();
    let persisted = PersistedState {
        date_from: st.date_from,
        date_to: st.date_to,
        nm_id_filter: st.nm_id_filter,
        sa_name_filter: st.sa_name_filter,
        connection_filter: st.connection_filter,
        operation_filter: st.operation_filter,
        srid_filter: st.srid_filter,
        sort_by: st.sort_by,
        sort_ascending: st.sort_ascending,
        page: st.page,
        page_size: st.page_size,
    };
    save_persisted(&persisted);
}

pub fn create_state() -> RwSignal<P903ListState> {
    let mut st = P903ListState::default();
    if let Some(p) = load_persisted() {
        st.date_from = p.date_from;
        st.date_to = p.date_to;
        st.nm_id_filter = p.nm_id_filter;
        st.sa_name_filter = p.sa_name_filter;
        st.connection_filter = p.connection_filter;
        st.operation_filter = p.operation_filter;
        st.srid_filter = p.srid_filter;
        st.sort_by = p.sort_by;
        st.sort_ascending = p.sort_ascending;
        st.page = p.page;
        st.page_size = p.page_size;
    }
    RwSignal::new(st)
}
