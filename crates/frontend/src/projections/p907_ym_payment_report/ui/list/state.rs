use leptos::prelude::*;
use serde::{Deserialize, Serialize};

const STORAGE_KEY: &str = "p907_ym_payment_report_list_state_v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersistedState {
    pub date_from: String,
    pub date_to: String,
    pub transaction_type_filter: String,
    pub payment_status_filter: String,
    pub shop_sku_filter: String,
    pub connection_filter: String,
    pub sort_by: String,
    pub sort_ascending: bool,
    pub page: usize,
    pub page_size: usize,
}

#[derive(Clone, Debug)]
pub struct P907ListState {
    // Filters
    pub date_from: String,
    pub date_to: String,
    pub transaction_type_filter: String,
    pub payment_status_filter: String,
    pub shop_sku_filter: String,
    pub connection_filter: String,

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

impl Default for P907ListState {
    fn default() -> Self {
        let now = chrono::Utc::now().date_naive();
        let default_start = now - chrono::Duration::days(30);
        let default_end = now;

        Self {
            date_from: default_start.format("%Y-%m-%d").to_string(),
            date_to: default_end.format("%Y-%m-%d").to_string(),
            transaction_type_filter: String::new(),
            payment_status_filter: String::new(),
            shop_sku_filter: String::new(),
            connection_filter: String::new(),
            sort_by: "transaction_date".to_string(),
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

pub fn persist_state(signal: RwSignal<P907ListState>) {
    let st = signal.get_untracked();
    let persisted = PersistedState {
        date_from: st.date_from,
        date_to: st.date_to,
        transaction_type_filter: st.transaction_type_filter,
        payment_status_filter: st.payment_status_filter,
        shop_sku_filter: st.shop_sku_filter,
        connection_filter: st.connection_filter,
        sort_by: st.sort_by,
        sort_ascending: st.sort_ascending,
        page: st.page,
        page_size: st.page_size,
    };
    save_persisted(&persisted);
}

pub fn create_state() -> RwSignal<P907ListState> {
    let mut st = P907ListState::default();
    if let Some(p) = load_persisted() {
        st.date_from = p.date_from;
        st.date_to = p.date_to;
        st.transaction_type_filter = p.transaction_type_filter;
        st.payment_status_filter = p.payment_status_filter;
        st.shop_sku_filter = p.shop_sku_filter;
        st.connection_filter = p.connection_filter;
        st.sort_by = p.sort_by;
        st.sort_ascending = p.sort_ascending;
        st.page = p.page;
        st.page_size = p.page_size;
    }
    RwSignal::new(st)
}
