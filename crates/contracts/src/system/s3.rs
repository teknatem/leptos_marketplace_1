use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum S3FileCategory {
    #[serde(rename = "documents")]
    Documents,
    #[serde(rename = "plugins")]
    Plugins,
    #[serde(rename = "backups")]
    Backups,
    #[serde(rename = "conference_audio")]
    ConferenceAudio,
    #[serde(rename = "other")]
    Other,
}

impl Default for S3FileCategory {
    fn default() -> Self {
        Self::Documents
    }
}

impl S3FileCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Documents => "documents",
            Self::Plugins => "plugins",
            Self::Backups => "backups",
            Self::ConferenceAudio => "conference_audio",
            Self::Other => "other",
        }
    }

    pub fn label_ru(&self) -> &'static str {
        match self {
            Self::Documents => "Документы",
            Self::Plugins => "Плагины",
            Self::Backups => "Архивы БД",
            Self::ConferenceAudio => "Аудио конференций",
            Self::Other => "Другое",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Documents,
            Self::Plugins,
            Self::Backups,
            Self::ConferenceAudio,
            Self::Other,
        ]
    }
}

impl From<&str> for S3FileCategory {
    fn from(value: &str) -> Self {
        match value {
            "documents" => Self::Documents,
            "plugins" => Self::Plugins,
            "backups" => Self::Backups,
            "conference_audio" => Self::ConferenceAudio,
            "other" => Self::Other,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3FileDto {
    pub id: String,
    pub category: S3FileCategory,
    pub bucket: String,
    pub object_key: String,
    pub original_filename: String,
    pub content_type: Option<String>,
    pub size_bytes: i64,
    pub etag: Option<String>,
    pub uploaded_by_user_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3FileListResponse {
    pub items: Vec<S3FileDto>,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3UploadResponse {
    pub file: S3FileDto,
}
