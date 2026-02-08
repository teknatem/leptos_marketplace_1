//! LLM Chat Details - View Model

use contracts::domain::a018_llm_chat::aggregate::{LlmChat, LlmChatMessage};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileInfo {
    pub id: String,
    pub filename: String,
    pub file_size: i64,
}

#[derive(Clone, Copy)]
pub struct LlmChatDetailsVm {
    pub chat: RwSignal<Option<LlmChat>>,
    pub messages: RwSignal<Vec<LlmChatMessage>>,
    pub new_message: RwSignal<String>,
    pub error: RwSignal<Option<String>>,
    pub is_sending: RwSignal<bool>,
    pub uploaded_files: RwSignal<Vec<FileInfo>>,
}

impl LlmChatDetailsVm {
    pub fn new() -> Self {
        Self {
            chat: RwSignal::new(None),
            messages: RwSignal::new(Vec::new()),
            new_message: RwSignal::new(String::new()),
            error: RwSignal::new(None),
            is_sending: RwSignal::new(false),
            uploaded_files: RwSignal::new(Vec::new()),
        }
    }
}
