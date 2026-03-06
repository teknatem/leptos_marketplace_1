use contracts::system::tasks::response::ScheduledTaskResponse;
use leptos::prelude::*;

#[derive(Debug, Clone, Default)]
pub struct ScheduledTaskListState {
    pub tasks: Vec<ScheduledTaskResponse>,
    pub is_loaded: bool,
    pub selected_ids: Vec<String>,
}

pub fn create_state() -> RwSignal<ScheduledTaskListState> {
    RwSignal::new(ScheduledTaskListState::default())
}
