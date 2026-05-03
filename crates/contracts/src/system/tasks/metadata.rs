use serde::{Deserialize, Serialize};

// ============================================================================
// Static types (backend only, no serialization)
// ============================================================================

/// Type of a config field — determines which UI widget is rendered.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaskConfigFieldType {
    /// UUID selected from the a006_connection_mp list
    ConnectionMp,
    /// Integer number input (with optional min/max)
    Integer,
    /// Plain text input
    Text,
    /// Date input (YYYY-MM-DD), rendered as <input type="date">
    Date,
}

/// Schema definition for a single config field (static, backend-only version).
#[derive(Debug, Clone, Copy)]
pub struct TaskConfigField {
    pub key: &'static str,
    pub label: &'static str,
    /// Short hint shown below the field
    pub hint: &'static str,
    pub field_type: TaskConfigFieldType,
    pub required: bool,
    pub default_value: Option<&'static str>,
    pub min_value: Option<i64>,
    pub max_value: Option<i64>,
}

/// Информация о внешнем API, используемом задачей (статическая версия для бэкенда)
#[derive(Debug, Clone)]
pub struct ExternalApiInfo {
    pub name: &'static str,
    pub base_url: &'static str,
    /// Описание ограничений скорости, например "10 запросов/мин"
    pub rate_limit_desc: &'static str,
}

/// Статические метаданные типа задачи для бэкенда (описание для человека и LLM)
#[derive(Debug, Clone)]
pub struct TaskMetadata {
    pub task_type: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
    pub external_apis: &'static [ExternalApiInfo],
    pub constraints: &'static [&'static str],
    /// Schema for the task's config_json — drives the UI editor.
    /// Empty slice means no structured editor; raw JSON textarea is shown.
    pub config_fields: &'static [TaskConfigField],
    /// Максимальная длительность выполнения (секунды); `manager.run()` оборачивается в `tokio::time::timeout`.
    pub max_duration_seconds: u64,
}

// ============================================================================
// DTOs — owned types for HTTP serialization to the frontend
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskConfigFieldTypeDto {
    ConnectionMp,
    Integer,
    Text,
    Date,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfigFieldDto {
    pub key: String,
    pub label: String,
    pub hint: String,
    pub field_type: TaskConfigFieldTypeDto,
    pub required: bool,
    pub default_value: Option<String>,
    pub min_value: Option<i64>,
    pub max_value: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalApiInfoDto {
    pub name: String,
    pub base_url: String,
    pub rate_limit_desc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadataDto {
    pub task_type: String,
    pub display_name: String,
    pub description: String,
    pub external_apis: Vec<ExternalApiInfoDto>,
    pub constraints: Vec<String>,
    /// Structured config schema — empty vec means show raw JSON textarea.
    pub config_fields: Vec<TaskConfigFieldDto>,
    pub max_duration_seconds: u64,
}

impl From<&TaskMetadata> for TaskMetadataDto {
    fn from(m: &TaskMetadata) -> Self {
        Self {
            task_type: m.task_type.to_string(),
            display_name: m.display_name.to_string(),
            description: m.description.to_string(),
            external_apis: m
                .external_apis
                .iter()
                .map(|a| ExternalApiInfoDto {
                    name: a.name.to_string(),
                    base_url: a.base_url.to_string(),
                    rate_limit_desc: a.rate_limit_desc.to_string(),
                })
                .collect(),
            constraints: m.constraints.iter().map(|s| s.to_string()).collect(),
            max_duration_seconds: m.max_duration_seconds,
            config_fields: m
                .config_fields
                .iter()
                .map(|f| TaskConfigFieldDto {
                    key: f.key.to_string(),
                    label: f.label.to_string(),
                    hint: f.hint.to_string(),
                    field_type: match f.field_type {
                        TaskConfigFieldType::ConnectionMp => TaskConfigFieldTypeDto::ConnectionMp,
                        TaskConfigFieldType::Integer => TaskConfigFieldTypeDto::Integer,
                        TaskConfigFieldType::Text => TaskConfigFieldTypeDto::Text,
                        TaskConfigFieldType::Date => TaskConfigFieldTypeDto::Date,
                    },
                    required: f.required,
                    default_value: f.default_value.map(|s| s.to_string()),
                    min_value: f.min_value,
                    max_value: f.max_value,
                })
                .collect(),
        }
    }
}
