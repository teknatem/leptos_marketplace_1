//! LLM Chat Details - View Model

use contracts::domain::a018_llm_chat::aggregate::{LlmChat, LlmChatMessage};
use leptos::prelude::*;

#[derive(Clone, Copy)]
pub struct LlmChatDetailsVm {
    pub chat: RwSignal<Option<LlmChat>>,
    pub messages: RwSignal<Vec<LlmChatMessage>>,
    pub new_message: RwSignal<String>,
    pub error: RwSignal<Option<String>>,
    pub is_sending: RwSignal<bool>,
}

impl LlmChatDetailsVm {
    pub fn new() -> Self {
        Self {
            chat: RwSignal::new(None),
            messages: RwSignal::new(Vec::new()),
            new_message: RwSignal::new(String::new()),
            error: RwSignal::new(None),
            is_sending: RwSignal::new(false),
        }
    }
}
