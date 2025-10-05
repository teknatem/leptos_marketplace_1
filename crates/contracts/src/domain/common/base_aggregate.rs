use serde::{Deserialize, Serialize};
use super::{EntityMetadata, EventStore};

/// Базовый агрегат с обязательными полями для всех агрегатов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseAggregate<Id> {
    /// Уникальный идентификатор записи
    pub id: Id,
    /// Бизнес-код записи (например, "ORD-2025-001", "CLT-12345")
    pub code: String,
    /// Описание/название записи
    pub description: String,
    /// Комментарий
    pub comment: Option<String>,
    /// Метаданные жизненного цикла
    pub metadata: EntityMetadata,
    /// Хранилище событий
    pub events: EventStore,
}

impl<Id> BaseAggregate<Id> {
    /// Создать новый агрегат
    pub fn new(id: Id, code: String, description: String) -> Self {
        Self {
            id,
            code,
            description,
            comment: None,
            metadata: EntityMetadata::new(),
            events: EventStore::new(),
        }
    }

    /// Создать агрегат с существующими метаданными (для загрузки из БД)
    pub fn with_metadata(
        id: Id,
        code: String,
        description: String,
        comment: Option<String>,
        metadata: EntityMetadata,
    ) -> Self {
        Self {
            id,
            code,
            description,
            comment,
            metadata,
            events: EventStore::new(),
        }
    }

    /// Обновить timestamp
    pub fn touch(&mut self) {
        self.metadata.touch();
    }

    /// Установить комментарий
    pub fn set_comment(&mut self, comment: Option<String>) {
        self.comment = comment;
    }
}
