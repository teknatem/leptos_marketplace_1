use leptos::prelude::Effect;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;
use web_sys::window;

#[derive(Clone, Copy)]
pub struct AppGlobalContext {
    pub opened: RwSignal<Vec<Tab>>,
    pub active: RwSignal<Option<String>>,
    pub left_open: RwSignal<bool>,
    pub right_open: RwSignal<bool>,
    pub form_states: RwSignal<HashMap<String, serde_json::Value>>,
    /// Maps a tab key to the key of the tab that opened it ("parent").
    /// Used to return to the originating page when a tab is closed.
    pub opener: RwSignal<HashMap<String, String>>,
    /// Activation order of currently-open tabs (most recent at the end).
    /// Used as a fallback "back" chain when a tab has no live parent.
    pub visit_stack: RwSignal<Vec<String>>,
}

impl AppGlobalContext {
    pub fn new() -> Self {
        Self {
            opened: RwSignal::new(vec![]),
            active: RwSignal::new(None),
            left_open: RwSignal::new(true),
            right_open: RwSignal::new(true),
            form_states: RwSignal::new(HashMap::new()),
            opener: RwSignal::new(HashMap::new()),
            visit_stack: RwSignal::new(Vec::new()),
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
                let known = crate::layout::tabs::tab_label_for_key(&active_key);
                let title = if known.is_empty() {
                    active_key.clone()
                } else {
                    known.to_string()
                };
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
            // Record the originating tab as this tab's parent, so closing it
            // returns to where the navigation started (A → B, close B → A).
            let parent = self.active.get_untracked();
            if let Some(parent) = parent {
                if parent != key {
                    self.opener.update(|map| {
                        map.insert(key.to_string(), parent);
                    });
                }
            }
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
        // Record into the persistent "История открытых страниц" log.
        // Centralised here so every navigation is captured automatically.
        Self::record_history(key, title);
    }

    /// Fire-and-forget recording of a page open into the backend history log.
    fn record_history(key: &str, title: &str) {
        let key = key.to_string();
        let title = title.to_string();
        spawn_local(async move {
            if let Err(err) = crate::system::history::api::record(&key, &title).await {
                leptos::logging::log!("⚠️ Failed to record page history: {}", err);
            }
        });
    }

    pub fn activate_tab(&self, key: &str) {
        leptos::logging::log!("🔶 activate_tab: key='{}'", key);
        // Maintain MRU activation order: move this key to the end of the stack.
        self.visit_stack.update(|stack| {
            stack.retain(|k| k != key);
            stack.push(key.to_string());
        });
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

        let was_active = self
            .active
            .with_untracked(|active| active.as_deref() == Some(key));

        // Resolve where to land BEFORE mutating, so the parent/stack still
        // reflect the state at the moment of closing.
        let next_active = if was_active {
            self.resolve_return_target(key)
        } else {
            None
        };

        self.opened.update(|tabs| {
            let before = tabs.len();
            tabs.retain(|tab| tab.key != key);
            let after = tabs.len();
            leptos::logging::log!("📊 Tabs before: {}, after: {}", before, after);
        });

        // Drop the closed tab from the navigation bookkeeping.
        self.visit_stack.update(|stack| stack.retain(|k| k != key));
        self.opener.update(|map| {
            map.remove(key);
        });

        if was_active {
            leptos::logging::log!("➡️ Next active tab: {:?}", next_active);
            self.active.set(next_active);
        }
    }

    /// Decide which tab to activate after the active tab `key` is closed.
    ///
    /// Priority (hybrid return chain):
    /// 1. The tab that opened `key` (its parent), if still open.
    /// 2. Walking the parent chain upward to the nearest still-open ancestor.
    /// 3. The previously-visited tab from the activation stack.
    /// 4. The last remaining open tab.
    fn resolve_return_target(&self, key: &str) -> Option<String> {
        let is_open = |candidate: &str| {
            self.opened.with_untracked(|tabs| {
                tabs.iter()
                    .any(|tab| tab.key == candidate && tab.key != key)
            })
        };

        // 1 + 2: follow the opener chain to the nearest live ancestor.
        let opener = self.opener.get_untracked();
        let mut current = opener.get(key).cloned();
        let mut guard = 0;
        while let Some(parent) = current {
            if is_open(&parent) {
                return Some(parent);
            }
            current = opener.get(&parent).cloned();
            guard += 1;
            if guard > 64 {
                break; // safety against accidental cycles
            }
        }

        // 3: most recent still-open tab from the activation stack (skip `key`).
        let from_stack = self.visit_stack.with_untracked(|stack| {
            stack
                .iter()
                .rev()
                .find(|k| k.as_str() != key && is_open(k))
                .cloned()
        });
        if from_stack.is_some() {
            return from_stack;
        }

        // 4: fall back to the last remaining open tab.
        self.opened.with_untracked(|tabs| {
            tabs.iter()
                .rev()
                .find(|tab| tab.key != key)
                .map(|tab| tab.key.clone())
        })
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
