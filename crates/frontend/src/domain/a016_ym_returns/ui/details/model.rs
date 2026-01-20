//! Yandex Market Returns Details - Model Layer
//!
//! DTOs and API functions for YM returns details

use serde::{Deserialize, Serialize};

/// Main return detail DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmReturnDetailDto {
    pub id: String,
    pub code: String,
    pub description: String,
    pub header: HeaderDto,
    pub lines: Vec<LineDto>,
    pub state: StateDto,
    pub source_meta: SourceMetaDto,
    pub is_posted: bool,
}

/// Return header information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderDto {
    pub return_id: i64,
    pub order_id: i64,
    pub connection_id: String,
    pub organization_id: String,
    pub marketplace_id: String,
    pub campaign_id: String,
    pub return_type: String,
    pub amount: Option<f64>,
    pub currency: Option<String>,
}

/// Return line item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineDto {
    pub item_id: i64,
    pub shop_sku: String,
    pub offer_id: String,
    pub name: String,
    pub count: i32,
    pub price: Option<f64>,
    pub return_reason: Option<String>,
    pub decisions: Vec<DecisionDto>,
    #[serde(default)]
    pub photos: Vec<String>,
}

/// Decision information for line item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionDto {
    pub decision_type: String,
    pub amount: Option<f64>,
    pub currency: Option<String>,
    pub partner_compensation_amount: Option<f64>,
    pub comment: Option<String>,
}

/// Return state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDto {
    pub refund_status: String,
    pub created_at_source: Option<String>,
    pub updated_at_source: Option<String>,
    pub refund_date: Option<String>,
}

/// Source metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMetaDto {
    pub raw_payload_ref: String,
    pub fetched_at: String,
    pub document_version: i32,
}

// Constants for table IDs
pub const TABLE_ID_LINES: &str = "a016-ym-return-lines-table";
pub const TABLE_ID_PROJECTIONS: &str = "a016-ym-return-p904-table";
