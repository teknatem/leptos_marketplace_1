use leptos::prelude::Effect;
use leptos::prelude::*;
use web_sys::window;

#[derive(Clone, Copy)]
pub struct AppGlobalContext {
    pub opened: RwSignal<Vec<Tab>>,
    pub active: RwSignal<Option<String>>,
    pub left_open: RwSignal<bool>,
    pub right_open: RwSignal<bool>,
}

impl AppGlobalContext {
    pub fn new() -> Self {
        Self {
            opened: RwSignal::new(vec![]),
            active: RwSignal::new(None),
            left_open: RwSignal::new(true),
            right_open: RwSignal::new(true),
        }
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
                    active_key,
                )]))
                .unwrap_or_default();
                if let Some(w) = window() {
                    if let Ok(history) = w.history() {
                        let _ = history.replace_state_with_url(
                            &wasm_bindgen::JsValue::NULL,
                            "",
                            Some(&format!("?{}", query_string)),
                        );
                    }
                }
            }
        });
    }

    pub fn open_tab(&self, key: &str, title: &str) {
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
            self.opened.update(|tabs| tabs.push(tab));
        }
        self.activate_tab(key);
    }

    pub fn activate_tab(&self, key: &str) {
        self.active.set(Some(key.to_string()));
    }

    pub fn close_tab(&self, key: &str) {
        self.opened.update(|tabs| tabs.retain(|tab| tab.key != key));
        if self
            .active
            .with_untracked(|active| active.as_deref() == Some(key))
        {
            let next_active = self
                .opened
                .with_untracked(|tabs| tabs.last().map(|t| t.key.clone()));
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
