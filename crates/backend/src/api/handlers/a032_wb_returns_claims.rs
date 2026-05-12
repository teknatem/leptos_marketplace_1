use axum::{extract::Path, extract::Query, Json};
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

use crate::domain::a002_organization;
use crate::domain::a032_wb_returns_claims;

#[derive(Debug, Deserialize)]
pub struct ListReturnsClaimsQuery {
    pub connection_id: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub is_archive: Option<bool>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// GET /api/a032/wb-returns-claims
pub async fn list_returns_claims(
    Query(_query): Query<ListReturnsClaimsQuery>,
) -> Result<
    Json<Vec<contracts::domain::a032_wb_returns_claims::aggregate::WbReturnsClaimsListDto>>,
    axum::http::StatusCode,
> {
    let items = a032_wb_returns_claims::service::list_all().await.map_err(|e| {
        tracing::error!("Failed to list wb returns claims: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Resolve organization names: collect unique org UUIDs, batch-load orgs
    let org_name_map: HashMap<String, String> = {
        let unique_ids: Vec<Uuid> = {
            let mut seen = std::collections::HashSet::new();
            items
                .iter()
                .filter_map(|a| Uuid::parse_str(&a.organization_id).ok())
                .filter(|id| seen.insert(*id))
                .collect()
        };

        let mut map = HashMap::new();
        for org_id in unique_ids {
            if let Ok(Some(org)) = a002_organization::service::get_by_id(org_id).await {
                let name = if !org.full_name.trim().is_empty() {
                    org.full_name.clone()
                } else {
                    org.base.description.clone()
                };
                map.insert(org_id.to_string(), name);
            }
        }
        map
    };

    let dtos: Vec<_> = items
        .into_iter()
        .map(|a| {
            let mut dto = a.to_list_dto();
            dto.org_name = org_name_map.get(&a.organization_id).cloned();
            dto
        })
        .collect();

    Ok(Json(dtos))
}

/// GET /api/a032/wb-returns-claims/:id
pub async fn get_returns_claim_detail(
    Path(id): Path<String>,
) -> Result<
    Json<contracts::domain::a032_wb_returns_claims::aggregate::WbReturnsClaimsDetailDto>,
    axum::http::StatusCode,
> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
    match a032_wb_returns_claims::service::get_by_id(uuid).await {
        Ok(Some(item)) => Ok(Json(item.to_detail_dto())),
        Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to get wb returns claim {}: {}", id, e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
