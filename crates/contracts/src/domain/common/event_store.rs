use serde::{Deserialize, Serialize};

/// Хранилище доменных событий (для будущей реализации Event Sourcing)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventStore {
    // Пока пустая структура, будет расширена позже
    _placeholder: (),
}

impl EventStore {
    pub fn new() -> Self {
        Self::default()
    }
}
