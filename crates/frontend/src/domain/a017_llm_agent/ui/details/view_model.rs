//! LLM Agent Details - ViewModel
//!
//! Reactive state management for LLM Agent details form

use leptos::prelude::*;

/// ViewModel for LLM Agent Details form
#[derive(Clone, Copy)]
pub struct LlmAgentDetailsVm {
    // Basic fields
    pub code: RwSignal<String>,
    pub description: RwSignal<String>,
    pub comment: RwSignal<String>,
    
    // Provider configuration
    pub provider_type: RwSignal<String>,
    pub api_endpoint: RwSignal<String>,
    pub api_key: RwSignal<String>,
    
    // Model configuration
    pub model_name: RwSignal<String>,
    pub temperature: RwSignal<String>,
    pub max_tokens: RwSignal<String>,
    pub system_prompt: RwSignal<String>,
    
    // Flags
    pub is_primary: RwSignal<bool>,
    
    // State signals
    pub error: Signal<Option<String>>,
    pub set_error: WriteSignal<Option<String>>,
    
    // Test connection state
    pub test_result: Signal<Option<(bool, String)>>,
    pub set_test_result: WriteSignal<Option<(bool, String)>>,
    pub is_testing: Signal<bool>,
    pub set_is_testing: WriteSignal<bool>,
    
    // Fetch models state
    pub available_models: Signal<Vec<serde_json::Value>>,
    pub set_available_models: WriteSignal<Vec<serde_json::Value>>,
    pub is_fetching_models: Signal<bool>,
    pub set_is_fetching_models: WriteSignal<bool>,
    pub fetch_models_result: Signal<Option<(bool, String)>>,
    pub set_fetch_models_result: WriteSignal<Option<(bool, String)>>,
    pub is_models_dropdown_open: RwSignal<bool>,
}

impl LlmAgentDetailsVm {
    /// Create new ViewModel with default values
    pub fn new() -> Self {
        let (error, set_error) = signal::<Option<String>>(None);
        let (test_result, set_test_result) = signal::<Option<(bool, String)>>(None);
        let (is_testing, set_is_testing) = signal(false);
        let (available_models, set_available_models) = signal::<Vec<serde_json::Value>>(Vec::new());
        let (is_fetching_models, set_is_fetching_models) = signal(false);
        let (fetch_models_result, set_fetch_models_result) = signal::<Option<(bool, String)>>(None);
        
        Self {
            code: RwSignal::new(String::new()),
            description: RwSignal::new(String::new()),
            comment: RwSignal::new(String::new()),
            provider_type: RwSignal::new("OpenAI".to_string()),
            api_endpoint: RwSignal::new("https://api.openai.com/v1".to_string()),
            api_key: RwSignal::new(String::new()),
            model_name: RwSignal::new("gpt-4o".to_string()),
            temperature: RwSignal::new("0.7".to_string()),
            max_tokens: RwSignal::new("4096".to_string()),
            system_prompt: RwSignal::new(String::new()),
            is_primary: RwSignal::new(false),
            is_models_dropdown_open: RwSignal::new(false),
            error: error.into(),
            set_error,
            test_result: test_result.into(),
            set_test_result,
            is_testing: is_testing.into(),
            set_is_testing,
            available_models: available_models.into(),
            set_available_models,
            is_fetching_models: is_fetching_models.into(),
            set_is_fetching_models,
            fetch_models_result: fetch_models_result.into(),
            set_fetch_models_result,
        }
    }
    
    /// Get temperature as f64
    pub fn get_temperature(&self) -> f64 {
        self.temperature.get().parse().unwrap_or(0.7)
    }
    
    /// Get max_tokens as i32
    pub fn get_max_tokens(&self) -> i32 {
        self.max_tokens.get().parse().unwrap_or(4096)
    }
    
    /// Build save DTO from current values
    pub fn build_save_dto(&self, id: Option<String>) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "code": self.code.get(),
            "description": self.description.get(),
            "comment": if self.comment.get().is_empty() { None } else { Some(self.comment.get()) },
            "provider_type": self.provider_type.get(),
            "api_endpoint": self.api_endpoint.get(),
            "api_key": self.api_key.get(),
            "model_name": self.model_name.get(),
            "temperature": self.get_temperature(),
            "max_tokens": self.get_max_tokens(),
            "system_prompt": if self.system_prompt.get().is_empty() { None } else { Some(self.system_prompt.get()) },
            "is_primary": self.is_primary.get(),
        })
    }
}

impl Default for LlmAgentDetailsVm {
    fn default() -> Self {
        Self::new()
    }
}
