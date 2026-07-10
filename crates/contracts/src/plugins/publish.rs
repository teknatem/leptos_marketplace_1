//! Контракты для публикации/обновления плагинов через S3 (единый shared bucket,
//! один общий `catalog.json` со списком последних версий по коду плагина).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Одна запись каталога `plugins/catalog.json` — последняя опубликованная версия плагина.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCatalogEntry {
    pub version: i32,
    pub uploaded_at: DateTime<Utc>,
    pub sha256: String,
    /// Ключ S3, по которому лежит бандл: `plugins/{code}/{version}/bundle.plugin`.
    pub key: String,
    pub size_bytes: u64,
    pub title: String,
}

/// Каталог целиком: код плагина -> последняя опубликованная версия.
pub type PluginCatalog = HashMap<String, PluginCatalogEntry>;

/// Результат `POST /api/plugin/:id/publish`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPublishResult {
    pub code: String,
    pub version: i32,
    pub uploaded_at: DateTime<Utc>,
    pub sha256: String,
    pub key: String,
    pub size_bytes: u64,
}

/// Одна строка ответа `GET /api/plugin/updates`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginUpdateStatus {
    pub plugin_id: String,
    pub code: String,
    pub local_version: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_version: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_uploaded_at: Option<DateTime<Utc>>,
    pub update_available: bool,
}

/// Тело `POST /api/plugin/:id/apply-update`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginApplyUpdateRequest {
    /// Если задано — применяется именно эта версия каталога (защита от гонки
    /// «каталог успел уйти вперёд между чтением списка и кликом»); по умолчанию — последняя.
    #[serde(default)]
    pub expected_remote_version: Option<i32>,
}
