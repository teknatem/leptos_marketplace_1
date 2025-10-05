use serde::{Deserialize, Serialize};

/// Метаданные экземпляра агрегата (lifecycle tracking)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMetadata {
    /// Дата создания записи
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Дата последнего обновления
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Мягкое удаление (soft delete)
    pub is_deleted: bool,
    /// Проведен (для документов)
    pub is_posted: bool,
    /// Версия для optimistic locking
    pub version: i32,
}

impl EntityMetadata {
    /// Создать новые метаданные для нового агрегата
    pub fn new() -> Self {
        let now = chrono::Utc::now();
        Self {
            created_at: now,
            updated_at: now,
            is_deleted: false,
            is_posted: false,
            version: 0,
        }
    }

    /// Обновить timestamp
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now();
    }

    /// Увеличить версию
    pub fn increment_version(&mut self) {
        self.version += 1;
    }
}

impl Default for EntityMetadata {
    fn default() -> Self {
        Self::new()
    }
}
