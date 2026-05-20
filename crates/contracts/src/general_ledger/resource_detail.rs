//! DTO для детализации GL-проводки: строки из resource_table и сверка с amount.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlResourceDetailTotals {
    pub row_count: usize,
    pub sum_resource: f64,
    pub sum_signed: f64,
    pub gl_amount: f64,
    pub delta: f64,
    pub is_match: bool,
}

/// Сводка по целостности `general_ledger_ref` в найденных detail-строках.
///
/// Каждая detail-строка обязана нести `general_ledger_ref = gl.id`. Любое
/// отклонение (NULL или иной id) трактуется как ошибка целостности данных,
/// которую необходимо показать пользователю.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlResourceDetailIntegrity {
    /// Сколько строк имеют корректный `general_ledger_ref = gl.id`.
    pub matched_count: usize,
    /// Сколько строк имеют `general_ledger_ref = NULL` или пустую строку.
    pub missing_count: usize,
    /// Сколько строк имеют `general_ledger_ref`, отличный от gl.id.
    pub mismatched_count: usize,
    /// Примеры отличающихся значений (до 5 уникальных), для диагностики.
    #[serde(default)]
    pub mismatched_refs_sample: Vec<String>,
    /// `true`, если matched_count == row_count и row_count >= 1.
    pub is_ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlResourceDetailResponse {
    pub gl_id: String,
    pub resource_table: String,
    pub resource_field: String,
    pub resource_sign: i32,
    pub supported: bool,
    pub rows: Vec<JsonValue>,
    pub totals: GlResourceDetailTotals,
    #[serde(default)]
    pub integrity: GlResourceDetailIntegrity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
