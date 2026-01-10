use leptos::prelude::*;
use serde::{Deserialize, Serialize};

const STORAGE_KEY: &str = "p906_nomenclature_prices_list_state_v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersistedState {
    pub period: String,
    pub q: String,
    pub sort_by: String,
    pub sort_ascending: bool,
    pub page: usize,
    pub page_size: usize,
}

#[derive(Clone, Debug)]
pub struct P906ListState {
    // filters
    pub period: String,
    pub q: String,

    // server sorting
    pub sort_by: String,
    pub sort_ascending: bool,

    // pagination
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,

    // load flag
    pub is_loaded: bool,
}

impl Default for P906ListState {
    fn default() -> Self {
        Self {
            period: String::new(),
            q: String::new(),
            // Default backend order is period DESC, so UI default is period + descending
            sort_by: "period".to_string(),
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

pub fn persist_state(signal: RwSignal<P906ListState>) {
    let st = signal.get_untracked();
    let persisted = PersistedState {
        period: st.period,
        q: st.q,
        sort_by: st.sort_by,
        sort_ascending: st.sort_ascending,
        page: st.page,
        page_size: st.page_size,
    };
    save_persisted(&persisted);
}

pub fn create_state() -> RwSignal<P906ListState> {
    let mut st = P906ListState::default();
    if let Some(p) = load_persisted() {
        st.period = p.period;
        st.q = p.q;
        st.sort_by = p.sort_by;
        st.sort_ascending = p.sort_ascending;
        st.page = p.page;
        st.page_size = p.page_size;
    }
    RwSignal::new(st)
}


