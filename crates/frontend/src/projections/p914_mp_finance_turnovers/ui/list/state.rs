use leptos::prelude::*;
use serde::{Deserialize, Serialize};

const STORAGE_KEY: &str = "p914_mp_finance_turnovers_list_state_v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersistedState {
    pub date_from: String,
    pub date_to: String,
    pub connection_mp_ref: String,
    pub registrator_type: String,
    pub turnover_code: String,
    pub order_key: String,
    pub event_kind: String,
    pub sort_by: String,
    pub sort_ascending: bool,
    pub page: usize,
    pub page_size: usize,
}

#[derive(Clone, Debug)]
pub struct P914ListState {
    pub date_from: String,
    pub date_to: String,
    pub connection_mp_ref: String,
    pub registrator_type: String,
    pub turnover_code: String,
    pub order_key: String,
    pub event_kind: String,
    pub sort_by: String,
    pub sort_ascending: bool,
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
    pub is_loaded: bool,
}

impl Default for P914ListState {
    fn default() -> Self {
        Self {
            date_from: String::new(),
            date_to: String::new(),
            connection_mp_ref: String::new(),
            registrator_type: String::new(),
            turnover_code: String::new(),
            order_key: String::new(),
            event_kind: String::new(),
            sort_by: "transaction_date".to_string(),
            sort_ascending: false,
            page: 0,
            page_size: 200,
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

fn save_persisted(s: &PersistedState) {
    let Some(storage) = storage() else { return };
    let Ok(raw) = serde_json::to_string(s) else {
        return;
    };
    let _ = storage.set_item(STORAGE_KEY, &raw);
}

pub fn persist_state(signal: RwSignal<P914ListState>) {
    let s = signal.get_untracked();
    save_persisted(&PersistedState {
        date_from: s.date_from,
        date_to: s.date_to,
        connection_mp_ref: s.connection_mp_ref,
        registrator_type: s.registrator_type,
        turnover_code: s.turnover_code,
        order_key: s.order_key,
        event_kind: s.event_kind,
        sort_by: s.sort_by,
        sort_ascending: s.sort_ascending,
        page: s.page,
        page_size: s.page_size,
    });
}

pub fn create_state() -> RwSignal<P914ListState> {
    let mut state = P914ListState::default();
    if let Some(p) = load_persisted() {
        state.date_from = p.date_from;
        state.date_to = p.date_to;
        state.connection_mp_ref = p.connection_mp_ref;
        state.registrator_type = p.registrator_type;
        state.turnover_code = p.turnover_code;
        state.order_key = p.order_key;
        state.event_kind = p.event_kind;
        state.sort_by = p.sort_by;
        state.sort_ascending = p.sort_ascending;
        state.page = p.page;
        state.page_size = p.page_size;
    }
    RwSignal::new(state)
}
