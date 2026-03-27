use leptos::prelude::*;
use serde::{Deserialize, Serialize};

const STORAGE_KEY: &str = "p910_mp_unlinked_turnovers_list_state_v2";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersistedState {
    pub date_from: String,
    pub date_to: String,
    pub connection_mp_ref: String,
    pub layer: String,
    pub turnover_code: String,
    pub registrator_type: String,
    pub sort_by: String,
    pub sort_ascending: bool,
    pub page: usize,
    pub page_size: usize,
}

#[derive(Clone, Debug)]
pub struct P910ListState {
    pub date_from: String,
    pub date_to: String,
    pub connection_mp_ref: String,
    pub layer: String,
    pub turnover_code: String,
    pub registrator_type: String,
    pub sort_by: String,
    pub sort_ascending: bool,
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
    pub is_loaded: bool,
}

impl Default for P910ListState {
    fn default() -> Self {
        Self {
            date_from: String::new(),
            date_to: String::new(),
            connection_mp_ref: String::new(),
            layer: String::new(),
            turnover_code: String::new(),
            registrator_type: String::new(),
            sort_by: "entry_date".to_string(),
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
    web_sys::window().and_then(|window| window.local_storage().ok().flatten())
}

fn load_persisted() -> Option<PersistedState> {
    let raw = storage()?.get_item(STORAGE_KEY).ok().flatten()?;
    serde_json::from_str::<PersistedState>(&raw).ok()
}

fn save_persisted(state: &PersistedState) {
    let Some(storage) = storage() else {
        return;
    };
    let Ok(raw) = serde_json::to_string(state) else {
        return;
    };
    let _ = storage.set_item(STORAGE_KEY, &raw);
}

pub fn persist_state(signal: RwSignal<P910ListState>) {
    let state = signal.get_untracked();
    save_persisted(&PersistedState {
        date_from: state.date_from,
        date_to: state.date_to,
        connection_mp_ref: state.connection_mp_ref,
        layer: state.layer,
        turnover_code: state.turnover_code,
        registrator_type: state.registrator_type,
        sort_by: state.sort_by,
        sort_ascending: state.sort_ascending,
        page: state.page,
        page_size: state.page_size,
    });
}

pub fn create_state() -> RwSignal<P910ListState> {
    let mut state = P910ListState::default();
    if let Some(persisted) = load_persisted() {
        state.date_from = persisted.date_from;
        state.date_to = persisted.date_to;
        state.connection_mp_ref = persisted.connection_mp_ref;
        state.layer = persisted.layer;
        state.turnover_code = persisted.turnover_code;
        state.registrator_type = persisted.registrator_type;
        state.sort_by = persisted.sort_by;
        state.sort_ascending = persisted.sort_ascending;
        state.page = persisted.page;
        state.page_size = persisted.page_size;
    }
    RwSignal::new(state)
}
