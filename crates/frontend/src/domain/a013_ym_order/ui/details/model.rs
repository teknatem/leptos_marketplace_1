//! API layer for YM Order details

use crate::shared::api_utils::api_base;
use gloo_net::http::Request;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NomenclatureInfo {
    pub description: String,
    pub article: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceProductInfo {
    pub description: String,
    pub article: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmOrderDetailDto {
    pub id: String,
    pub code: String,
    pub description: String,
    pub header: HeaderDto,
    pub lines: Vec<LineDto>,
    pub state: StateDto,
    pub source_meta: SourceMetaDto,
    pub metadata: MetadataDto,
    #[serde(default)]
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderDto {
    pub document_no: String,
    pub connection_id: String,
    pub organization_id: String,
    pub marketplace_id: String,
    pub campaign_id: String,
    pub total_amount: Option<f64>,
    pub currency: Option<String>,
    #[serde(default)]
    pub items_total: Option<f64>,
    #[serde(default)]
    pub delivery_total: Option<f64>,
    #[serde(default)]
    pub subsidies_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineDto {
    pub line_id: String,
    pub shop_sku: String,
    pub offer_id: String,
    pub name: String,
    pub qty: f64,
    pub price_list: Option<f64>,
    pub discount_total: Option<f64>,
    pub price_effective: Option<f64>,
    pub amount_line: Option<f64>,
    pub currency_code: Option<String>,
    #[serde(default)]
    pub buyer_price: Option<f64>,
    #[serde(default)]
    pub subsidies_json: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub price_plan: Option<f64>,
    #[serde(default)]
    pub marketplace_product_ref: Option<String>,
    #[serde(default)]
    pub nomenclature_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDto {
    pub status_raw: String,
    pub substatus_raw: Option<String>,
    pub status_norm: String,
    pub status_changed_at: Option<String>,
    pub updated_at_source: Option<String>,
    pub creation_date: Option<String>,
    pub delivery_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMetaDto {
    pub raw_payload_ref: String,
    pub fetched_at: String,
    pub document_version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataDto {
    pub created_at: String,
    pub updated_at: String,
    pub is_deleted: bool,
    pub is_posted: bool,
    pub version: i32,
}

pub async fn fetch_by_id(id: &str) -> Result<YmOrderDetailDto, String> {
    let url = format!("{}/api/a013/ym-order/{}", api_base(), id);
    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch: {}", e))?;

    if response.status() != 200 {
        return Err(format!("Server error: {}", response.status()));
    }

    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    serde_json::from_str(&text).map_err(|e| format!("Failed to parse: {}", e))
}

pub async fn fetch_raw_json(raw_payload_ref: &str) -> Result<String, String> {
    let url = format!("{}/api/a013/raw/{}", api_base(), raw_payload_ref);
    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch raw JSON: {}", e))?;
    if response.status() != 200 {
        return Err(format!("Server error: {}", response.status()));
    }
    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    let json_value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("Failed to parse JSON: {}", e))?;
    serde_json::to_string_pretty(&json_value).map_err(|e| format!("Failed to format JSON: {}", e))
}

pub async fn fetch_projections(id: &str) -> Result<serde_json::Value, String> {
    let url = format!("{}/api/a013/ym-order/{}/projections", api_base(), id);
    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch projections: {}", e))?;
    if response.status() != 200 {
        return Err(format!("Server error: {}", response.status()));
    }
    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    serde_json::from_str(&text).map_err(|e| format!("Failed to parse projections: {}", e))
}

pub async fn post_document(id: &str) -> Result<(), String> {
    let url = format!("{}/api/a013/ym-order/{}/post", api_base(), id);
    let response = Request::post(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to post document: {}", e))?;
    if response.status() != 200 {
        return Err(format!("Failed to post: status {}", response.status()));
    }
    Ok(())
}

pub async fn unpost_document(id: &str) -> Result<(), String> {
    let url = format!("{}/api/a013/ym-order/{}/unpost", api_base(), id);
    let response = Request::post(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to unpost document: {}", e))?;
    if response.status() != 200 {
        return Err(format!("Failed to unpost: status {}", response.status()));
    }
    Ok(())
}

pub async fn fetch_marketplace_product(id: &str) -> Result<MarketplaceProductInfo, String> {
    let url = format!("{}/api/marketplace_product/{}", api_base(), id);
    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch marketplace product: {}", e))?;
    if response.status() != 200 {
        return Err(format!("Server error: {}", response.status()));
    }
    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    let json: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("Failed to parse: {}", e))?;

    Ok(MarketplaceProductInfo {
        description: json
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        article: json
            .get("article")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

pub async fn fetch_nomenclature(id: &str) -> Result<NomenclatureInfo, String> {
    let url = format!("{}/api/nomenclature/{}", api_base(), id);
    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch nomenclature: {}", e))?;
    if response.status() != 200 {
        return Err(format!("Server error: {}", response.status()));
    }
    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    let json: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("Failed to parse: {}", e))?;

    Ok(NomenclatureInfo {
        description: json
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        article: json
            .get("article")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    })
}
