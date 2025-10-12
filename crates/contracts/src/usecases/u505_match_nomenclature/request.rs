use serde::{Deserialize, Serialize};

/// Запрос на сопоставление номенклатуры
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchRequest {
    /// ID маркетплейса для фильтрации (опционально, если пусто - обработать все)
    #[serde(rename = "marketplaceId")]
    pub marketplace_id: Option<String>,

    /// Перезаписать существующие связи (если false - обновлять только пустые nomenclature_id)
    #[serde(rename = "overwriteExisting", default)]
    pub overwrite_existing: bool,

    /// Игнорировать регистр при сопоставлении артикулов
    #[serde(rename = "ignoreCase", default = "default_true")]
    pub ignore_case: bool,
}

fn default_true() -> bool {
    true
}

impl Default for MatchRequest {
    fn default() -> Self {
        Self {
            marketplace_id: None,
            overwrite_existing: false,
            ignore_case: true,
        }
    }
}
