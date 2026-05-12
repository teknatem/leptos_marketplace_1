//! Per-page user preferences for navigator views.
//!
//! Stage 1: backed by `localStorage`. Stage 2 (planned): mirrored into the
//! `sys_navigator` table. The DTO already carries `version`, `hidden_links`
//! and `favorites` so the future migration needs no shape changes.

use super::types::ViewMode;
use serde::{Deserialize, Serialize};
use web_sys::window;

const STORAGE_PREFIX: &str = "navigator.prefs.";
const CURRENT_VERSION: u8 = 1;

/// Persisted preferences for a single navigator page.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NavigatorPagePrefs {
    pub version: u8,
    pub view_mode: ViewMode,
    /// Tab keys explicitly hidden by the user. Reserved for stage 2.
    #[serde(default)]
    pub hidden_links: Vec<String>,
    /// Tab keys pinned as favourites (ordered). Reserved for stage 2.
    #[serde(default)]
    pub favorites: Vec<String>,
}

impl Default for NavigatorPagePrefs {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION,
            view_mode: ViewMode::default(),
            hidden_links: Vec::new(),
            favorites: Vec::new(),
        }
    }
}

impl NavigatorPagePrefs {
    /// Load preferences for the given page id. Falls back to defaults if the
    /// browser has no storage, the entry is missing, or JSON is malformed.
    pub fn load(page_id: &str) -> Self {
        let Some(storage) = window().and_then(|w| w.local_storage().ok().flatten()) else {
            return Self::default();
        };
        let key = storage_key(page_id);
        match storage.get_item(&key) {
            Ok(Some(raw)) => serde_json::from_str(&raw).unwrap_or_default(),
            _ => Self::default(),
        }
    }

    /// Persist preferences. Errors are swallowed silently — UX should never
    /// break because of a storage hiccup.
    pub fn save(&self, page_id: &str) {
        let Some(storage) = window().and_then(|w| w.local_storage().ok().flatten()) else {
            return;
        };
        if let Ok(raw) = serde_json::to_string(self) {
            let _ = storage.set_item(&storage_key(page_id), &raw);
        }
    }
}

fn storage_key(page_id: &str) -> String {
    format!("{STORAGE_PREFIX}{page_id}")
}
