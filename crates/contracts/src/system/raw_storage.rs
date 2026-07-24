use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawStorageTypeStat {
    pub marketplace: String,
    pub document_type: String,
    pub rows: u64,
    pub raw_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawStorageStatus {
    pub capture_enabled: bool,
    pub total_rows: u64,
    pub total_mb: f64,
    pub referenced_rows: u64,
    pub unreferenced_rows: u64,
    pub by_type: Vec<RawStorageTypeStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawStorageSettings {
    pub capture_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawStorageCleanupMode {
    Unreferenced,
    Duplicates,
    OlderThanDays,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawStorageCleanupRequest {
    pub mode: RawStorageCleanupMode,
    pub older_than_days: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawStorageCleanupPreview {
    pub rows_to_delete: u64,
    pub estimated_mb: f64,
}

/// Текущий размер файла БД и объём, который освободит VACUUM (влияет на всю
/// базу, не только на document_raw_storage — свободные страницы копятся от
/// любых DELETE/UPDATE во всех таблицах).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbVacuumStatus {
    pub file_mb: f64,
    pub reclaimable_mb: f64,
    pub wal_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbVacuumResult {
    pub file_mb_before: f64,
    pub file_mb_after: f64,
    pub freed_mb: f64,
    pub duration_ms: u64,
    pub wal_mb_before: f64,
    pub wal_mb_after: f64,
    pub wal_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbWalCheckpointResult {
    pub wal_mb_before: f64,
    pub wal_mb_after: f64,
    pub truncated: bool,
}
