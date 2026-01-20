use contracts::domain::a019_llm_artifact::aggregate::LlmArtifact;
use leptos::prelude::*;

#[derive(Clone, Copy)]
pub struct LlmArtifactDetailsVm {
    pub artifact: RwSignal<Option<LlmArtifact>>,
    pub error: RwSignal<Option<String>>,
    pub is_editing: RwSignal<bool>,
    pub is_saving: RwSignal<bool>,
}

impl LlmArtifactDetailsVm {
    pub fn new() -> Self {
        Self {
            artifact: RwSignal::new(None),
            error: RwSignal::new(None),
            is_editing: RwSignal::new(false),
            is_saving: RwSignal::new(false),
        }
    }
}
