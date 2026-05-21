use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FavoriteDto {
    pub id: String,
    pub owner_user_id: String,
    pub owner_username: Option<String>,
    pub target_kind: String,
    pub target_id: String,
    pub target_title: String,
    pub tab_key: String,
    pub color: String,
    pub comment: Option<String>,
    pub is_global: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FavoriteUpsertRequest {
    pub target_kind: String,
    pub target_id: String,
    pub target_title: String,
    pub tab_key: String,
    pub color: String,
    pub comment: Option<String>,
    pub is_global: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FavoriteUpdateRequest {
    pub color: String,
    pub comment: Option<String>,
    pub is_global: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FavoriteTargetRequest {
    pub target_kind: String,
    pub target_id: String,
}

pub const FAVORITE_COLOR_YELLOW: &str = "yellow";

pub fn is_supported_favorite_color(value: &str) -> bool {
    matches!(
        value,
        "yellow" | "red" | "green" | "blue" | "purple" | "gray"
    )
}

pub fn is_supported_favorite_target_kind(value: &str) -> bool {
    matches!(
        value,
        "a012_wb_sales_details"
            | "a015_wb_orders_details"
            | "a027_wb_documents_details"
            | "a026_wb_advert_daily_details"
            | "a007_marketplace_product_details"
    )
}
