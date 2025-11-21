use std::collections::HashMap;
use serde_json::Value;

/// FormStateStore manages form filter states across tab switches
/// States are stored in memory and persist when switching between tabs
#[derive(Clone, Debug)]
pub struct FormStateStore {
    states: HashMap<String, Value>,
}

impl FormStateStore {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
        }
    }

    /// Get the state for a specific form
    pub fn get_state(&self, form_key: &str) -> Option<&Value> {
        self.states.get(form_key)
    }

    /// Set the state for a specific form
    pub fn set_state(&mut self, form_key: String, state: Value) {
        self.states.insert(form_key, state);
    }

    /// Remove the state for a specific form
    pub fn remove_state(&mut self, form_key: &str) {
        self.states.remove(form_key);
    }

    /// Clear all states
    pub fn clear_all(&mut self) {
        self.states.clear();
    }
}

impl Default for FormStateStore {
    fn default() -> Self {
        Self::new()
    }
}

