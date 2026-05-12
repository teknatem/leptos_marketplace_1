use crate::shared::api_utils::api_base;
use gloo_net::http::Request;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbReturnsClaimsDetailDto {
    pub id: String,
    pub code: String,
    pub description: String,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "organizationId")]
    pub organization_id: String,
    #[serde(rename = "marketplaceId")]
    pub marketplace_id: String,
    #[serde(rename = "claimId")]
    pub claim_id: String,
    #[serde(rename = "claimType")]
    pub claim_type: Option<i32>,
    pub status: Option<i32>,
    #[serde(rename = "statusEx")]
    pub status_ex: Option<i32>,
    #[serde(rename = "nmId")]
    pub nm_id: i64,
    #[serde(rename = "imtName")]
    pub imt_name: Option<String>,
    #[serde(rename = "userComment")]
    pub user_comment: Option<String>,
    #[serde(rename = "wbComment")]
    pub wb_comment: Option<String>,
    pub dt: String,
    #[serde(rename = "orderDt")]
    pub order_dt: Option<String>,
    #[serde(rename = "dtUpdate")]
    pub dt_update: Option<String>,
    #[serde(rename = "deliveryDt")]
    pub delivery_dt: Option<String>,
    pub price: Option<f64>,
    #[serde(rename = "currencyCode")]
    pub currency_code: Option<String>,
    pub srid: Option<String>,
    #[serde(rename = "originIdInfo")]
    pub origin_id_info: Option<String>,
    pub actions: Option<String>,
    #[serde(rename = "isArchive")]
    pub is_archive: bool,
    pub metadata: MetadataDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataDto {
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "isDeleted")]
    pub is_deleted: bool,
    #[serde(rename = "isPosted")]
    pub is_posted: bool,
    pub version: i32,
}

pub async fn fetch_by_id(id: &str) -> Result<WbReturnsClaimsDetailDto, String> {
    let url = format!("{}/api/a032/wb-returns-claims/{}", api_base(), id);
    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Ошибка запроса: {}", e))?;

    if response.status() == 404 {
        return Err(format!("Заявка {} не найдена", id));
    }
    if response.status() != 200 {
        return Err(format!("Ошибка сервера: HTTP {}", response.status()));
    }

    let text = response
        .text()
        .await
        .map_err(|e| format!("Ошибка чтения ответа: {}", e))?;
    serde_json::from_str(&text).map_err(|e| format!("Ошибка парсинга: {}", e))
}
