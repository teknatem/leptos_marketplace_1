//! API layer for WB Sales details
//!
//! Contains DTOs and async API functions for fetching and mutating WB Sales data.

use crate::shared::api_utils::api_base;
use contracts::projections::p903_wb_finance_report::dto::WbFinanceReportDto;
use gloo_net::http::Request;
use serde::{Deserialize, Serialize};

// ============================================
// DTOs
// ============================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSalesDetailDto {
    pub id: String,
    pub code: String,
    pub description: String,
    pub header: HeaderDto,
    pub line: LineDto,
    pub state: StateDto,
    pub warehouse: WarehouseDto,
    pub source_meta: SourceMetaDto,
    pub metadata: MetadataDto,
    pub marketplace_product_ref: Option<String>,
    pub nomenclature_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderDto {
    pub document_no: String,
    pub sale_id: Option<String>,
    pub connection_id: String,
    pub organization_id: String,
    pub marketplace_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineDto {
    pub line_id: String,
    pub supplier_article: String,
    pub nm_id: i64,
    pub barcode: String,
    pub name: String,
    pub qty: f64,
    pub price_list: Option<f64>,
    pub discount_total: Option<f64>,
    pub price_effective: Option<f64>,
    pub amount_line: Option<f64>,
    pub currency_code: Option<String>,
    pub total_price: Option<f64>,
    pub payment_sale_amount: Option<f64>,
    pub discount_percent: Option<f64>,
    pub spp: Option<f64>,
    pub finished_price: Option<f64>,

    // Plan/Fact fields
    pub is_fact: Option<bool>,
    pub sell_out_plan: Option<f64>,
    pub sell_out_fact: Option<f64>,
    pub acquiring_fee_plan: Option<f64>,
    pub acquiring_fee_fact: Option<f64>,
    pub other_fee_plan: Option<f64>,
    pub other_fee_fact: Option<f64>,
    pub supplier_payout_plan: Option<f64>,
    pub supplier_payout_fact: Option<f64>,
    pub profit_plan: Option<f64>,
    pub profit_fact: Option<f64>,
    pub cost_of_production: Option<f64>,
    pub commission_plan: Option<f64>,
    pub commission_fact: Option<f64>,
    pub dealer_price_ut: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDto {
    pub event_type: String,
    pub status_norm: String,
    pub sale_dt: String,
    pub last_change_dt: Option<String>,
    pub is_supply: Option<bool>,
    pub is_realization: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarehouseDto {
    pub warehouse_name: Option<String>,
    pub warehouse_type: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceProductInfo {
    pub description: String,
    pub article: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NomenclatureInfo {
    pub description: String,
    pub article: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationInfo {
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceInfo {
    pub name: String,
}

// ============================================
// API Functions
// ============================================

/// Fetch WB Sales detail by ID
pub async fn fetch_by_id(id: &str) -> Result<WbSalesDetailDto, String> {
    let url = format!("{}/api/a012/wb-sales/{}", api_base(), id);

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

/// Fetch raw JSON from WB API
pub async fn fetch_raw_json(raw_payload_ref: &str) -> Result<String, String> {
    let url = format!("{}/api/a012/raw/{}", api_base(), raw_payload_ref);

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

    // Parse and pretty-print JSON
    let json_value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    serde_json::to_string_pretty(&json_value).map_err(|e| format!("Failed to format JSON: {}", e))
}

/// Fetch projections for a WB Sales document
pub async fn fetch_projections(id: &str) -> Result<serde_json::Value, String> {
    let url = format!("{}/api/a012/wb-sales/{}/projections", api_base(), id);

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

/// Fetch linked finance reports by SRID
pub async fn fetch_finance_reports(srid: &str) -> Result<Vec<WbFinanceReportDto>, String> {
    let url = format!(
        "{}/api/p903/finance-report/search-by-srid?srid={}",
        api_base(),
        srid
    );

    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch finance reports: {}", e))?;

    if response.status() != 200 {
        return Err(format!("Server error: {}", response.status()));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse finance reports: {}", e))
}

/// Fetch marketplace product info
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

/// Fetch nomenclature info
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

/// Post (проведение) document
pub async fn post_document(id: &str) -> Result<(), String> {
    let url = format!("{}/api/a012/wb-sales/{}/post", api_base(), id);

    let response = Request::post(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to post document: {}", e))?;

    if response.status() != 200 {
        return Err(format!("Failed to post: status {}", response.status()));
    }

    Ok(())
}

/// Unpost (отмена проведения) document
pub async fn unpost_document(id: &str) -> Result<(), String> {
    let url = format!("{}/api/a012/wb-sales/{}/unpost", api_base(), id);

    let response = Request::post(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to unpost document: {}", e))?;

    if response.status() != 200 {
        return Err(format!("Failed to unpost: status {}", response.status()));
    }

    Ok(())
}

/// Fetch connection info
pub async fn fetch_connection(id: &str) -> Result<ConnectionInfo, String> {
    let url = format!("{}/api/connection_mp/{}", api_base(), id);

    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch connection: {}", e))?;

    if response.status() != 200 {
        return Err(format!("Server error: {}", response.status()));
    }

    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let json: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("Failed to parse: {}", e))?;

    Ok(ConnectionInfo {
        description: json
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

/// Fetch organization info
pub async fn fetch_organization(id: &str) -> Result<OrganizationInfo, String> {
    let url = format!("{}/api/organization/{}", api_base(), id);

    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch organization: {}", e))?;

    if response.status() != 200 {
        return Err(format!("Server error: {}", response.status()));
    }

    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let json: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("Failed to parse: {}", e))?;

    Ok(OrganizationInfo {
        description: json
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

/// Fetch marketplace info
pub async fn fetch_marketplace(id: &str) -> Result<MarketplaceInfo, String> {
    let url = format!("{}/api/marketplace/{}", api_base(), id);

    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch marketplace: {}", e))?;

    if response.status() != 200 {
        return Err(format!("Server error: {}", response.status()));
    }

    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let json: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("Failed to parse: {}", e))?;

    Ok(MarketplaceInfo {
        name: json
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

/// Refresh dealer price
pub async fn refresh_dealer_price(id: &str) -> Result<(), String> {
    let url = format!(
        "{}/api/a012/wb-sales/{}/refresh-dealer-price",
        api_base(),
        id
    );

    let response = Request::post(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to refresh dealer price: {}", e))?;

    if response.status() != 200 {
        return Err(format!("Failed to refresh: status {}", response.status()));
    }

    Ok(())
}
