use leptos::prelude::Effect;
use leptos::prelude::*;
use std::collections::HashMap;
use web_sys::window;

#[derive(Clone, Copy)]
pub struct AppGlobalContext {
    pub opened: RwSignal<Vec<Tab>>,
    pub active: RwSignal<Option<String>>,
    pub left_open: RwSignal<bool>,
    pub right_open: RwSignal<bool>,
    pub form_states: RwSignal<HashMap<String, serde_json::Value>>,
}

impl AppGlobalContext {
    pub fn new() -> Self {
        Self {
            opened: RwSignal::new(vec![]),
            active: RwSignal::new(None),
            left_open: RwSignal::new(true),
            right_open: RwSignal::new(true),
            form_states: RwSignal::new(HashMap::new()),
        }
    }

    pub fn get_form_state(&self, form_key: &str) -> Option<serde_json::Value> {
        self.form_states
            .with_untracked(|states| states.get(form_key).cloned())
    }

    pub fn set_form_state(&self, form_key: String, state: serde_json::Value) {
        self.form_states.update(|states| {
            states.insert(form_key, state);
        });
    }

    pub fn init_router_integration(&self) {
        let search = window()
            .and_then(|w| w.location().search().ok())
            .unwrap_or_default();
        let params: std::collections::HashMap<String, String> =
            serde_qs::from_str(search.trim_start_matches('?')).unwrap_or_default();
        if let Some(active_key) = params.get("active").cloned() {
            let exists = self
                .opened
                .with_untracked(|tabs| tabs.iter().any(|tab| tab.key == active_key));
            if !exists {
                let title = active_key.clone();
                self.open_tab(&active_key, &title);
            } else {
                self.activate_tab(&active_key);
            }
        }

        let this = *self;
        Effect::new(move |_| {
            if let Some(active_key) = this.active.get() {
                let query_string = serde_qs::to_string(&std::collections::HashMap::from([(
                    "active".to_string(),
                    active_key.clone(),
                )]))
                .unwrap_or_default();

                let new_url = format!("?{}", query_string);

                // Use untracked to avoid creating unnecessary reactive dependencies
                let current_search = window()
                    .and_then(|w| w.location().search().ok())
                    .unwrap_or_default();

                // Only update URL if it actually changed
                if current_search != new_url {
                    if let Some(w) = window() {
                        if let Ok(history) = w.history() {
                            let _ = history.replace_state_with_url(
                                &wasm_bindgen::JsValue::NULL,
                                "",
                                Some(&new_url),
                            );
                        }
                    }
                }
            }
        });
    }

    pub fn open_tab(&self, key: &str, title: &str) {
        leptos::logging::log!("🔷 open_tab: key='{}', title='{}'", key, title);
        let exists = self
            .opened
            .with_untracked(|tabs| tabs.iter().any(|tab| tab.key == key));
        if !exists {
            let tab = Tab {
                key: key.to_string(),
                title: title.to_string(),
                dirty: false,
                pinned: false,
            };
            self.opened.update(|tabs| {
                tabs.push(tab);
                leptos::logging::log!("✅ Tab added. Total tabs: {}", tabs.len());
            });
        } else {
            leptos::logging::log!("ℹ️ Tab already exists, just activating");
        }
        self.activate_tab(key);
    }

    pub fn activate_tab(&self, key: &str) {
        leptos::logging::log!("🔶 activate_tab: key='{}'", key);
        self.active.set(Some(key.to_string()));
    }

    pub fn update_tab_title(&self, key: &str, new_title: &str) {
        self.opened.update(|tabs| {
            if let Some(tab) = tabs.iter_mut().find(|t| t.key == key) {
                tab.title = new_title.to_string();
            }
        });
    }

    pub fn close_tab(&self, key: &str) {
        leptos::logging::log!("🔴 close_tab: key='{}'", key);
        self.opened.update(|tabs| {
            let before = tabs.len();
            tabs.retain(|tab| tab.key != key);
            let after = tabs.len();
            leptos::logging::log!("📊 Tabs before: {}, after: {}", before, after);
        });
        if self
            .active
            .with_untracked(|active| active.as_deref() == Some(key))
        {
            let next_active = self
                .opened
                .with_untracked(|tabs| tabs.last().map(|t| t.key.clone()));
            leptos::logging::log!("➡️ Next active tab: {:?}", next_active);
            self.active.set(next_active);
        }
    }

    pub fn toggle_left(&self) {
        self.left_open.update(|val| *val = !*val);
    }

    pub fn toggle_right(&self) {
        self.right_open.update(|val| *val = !*val);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tab {
    pub key: String,
    pub title: String,
    pub dirty: bool,
    pub pinned: bool,
}
