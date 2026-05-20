use leptos::prelude::*;
use serde::{Deserialize, Serialize};

const STORAGE_KEY: &str = "p913_wb_advert_order_attr_list_state_v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersistedState {
    pub date_from: String,
    pub date_to: String,
    pub connection_mp_ref: String,
    pub turnover_code: String,
    pub order_key: String,
    pub wb_advert_campaign_code: String,
    pub sort_by: String,
    pub sort_ascending: bool,
    pub page: usize,
    pub page_size: usize,
}

#[derive(Clone, Debug)]
pub struct P913ListState {
    pub date_from: String,
    pub date_to: String,
    pub connection_mp_ref: String,
    pub turnover_code: String,
    pub order_key: String,
    pub wb_advert_campaign_code: String,
    pub sort_by: String,
    pub sort_ascending: bool,
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
    pub is_loaded: bool,
}

impl Default for P913ListState {
    fn default() -> Self {
        Self {
            date_from: String::new(),
            date_to: String::new(),
            connection_mp_ref: String::new(),
            turnover_code: String::new(),
            order_key: String::new(),
            wb_advert_campaign_code: String::new(),
            sort_by: "entry_date".to_string(),
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

pub fn persist_state(signal: RwSignal<P913ListState>) {
    let s = signal.get_untracked();
    save_persisted(&PersistedState {
        date_from: s.date_from,
        date_to: s.date_to,
        connection_mp_ref: s.connection_mp_ref,
        turnover_code: s.turnover_code,
        order_key: s.order_key,
        wb_advert_campaign_code: s.wb_advert_campaign_code,
        sort_by: s.sort_by,
        sort_ascending: s.sort_ascending,
        page: s.page,
        page_size: s.page_size,
    });
}

pub fn create_state() -> RwSignal<P913ListState> {
    let mut state = P913ListState::default();
    if let Some(p) = load_persisted() {
        state.date_from = p.date_from;
        state.date_to = p.date_to;
        state.connection_mp_ref = p.connection_mp_ref;
        state.turnover_code = p.turnover_code;
        state.order_key = p.order_key;
        state.wb_advert_campaign_code = p.wb_advert_campaign_code;
        state.sort_by = p.sort_by;
        state.sort_ascending = p.sort_ascending;
        state.page = p.page;
        state.page_size = p.page_size;
    }
    RwSignal::new(state)
}
