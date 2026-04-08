use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NomenclatureCostDto {
    pub id: String,
    pub period: String,
    pub nomenclature_ref: String,
    pub nomenclature_name: Option<String>,
    pub nomenclature_article: Option<String>,
    pub cost: f64,
    pub quantity: Option<f64>,
    pub amount: Option<f64>,
    pub registrator_type: String,
    pub registrator_ref: String,
    pub line_no: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NomenclatureCostListResponse {
    pub items: Vec<NomenclatureCostDto>,
    pub total_count: i64,
}
