use axum::{extract::Path, extract::Query, Json};
use contracts::domain::a007_marketplace_product::aggregate::{
    MarketplaceProductListItemDto, WbMappingProblemDto, WbMappingProblemsResponse,
    WbStalePostingsRepostResponse, WbStalePostingsRequest, WbStalePostingsSummary,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::domain::a007_marketplace_product;

#[derive(Debug, Clone, Serialize)]
pub struct PaginatedMarketplaceProductResponse {
    pub items: Vec<MarketplaceProductListItemDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

#[derive(Debug, Deserialize)]
pub struct ListMarketplaceProductsQuery {
    pub marketplace_ref: Option<String>,
    pub connection_mp_ref: Option<String>,
    pub problems_only: Option<bool>,
    pub search: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub sort_by: Option<String>,
    pub sort_desc: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct WbMappingProblemsQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub connection_mp_ref: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// GET /api/a007/marketplace-product
pub async fn list_paginated(
    Query(query): Query<ListMarketplaceProductsQuery>,
) -> Result<Json<PaginatedMarketplaceProductResponse>, axum::http::StatusCode> {
    use a007_marketplace_product::repository::MarketplaceProductListQuery;

    let limit = query.limit.unwrap_or(100);
    let offset = query.offset.unwrap_or(0);
    let page = if limit > 0 { offset / limit } else { 0 };

    let list_query = MarketplaceProductListQuery {
        marketplace_ref: query.marketplace_ref,
        connection_mp_ref: query.connection_mp_ref,
        problems_only: query.problems_only.unwrap_or(false),
        search: query.search,
        sort_by: query.sort_by.unwrap_or_else(|| "code".to_string()),
        sort_desc: query.sort_desc.unwrap_or(false),
        limit,
        offset,
    };

    match a007_marketplace_product::service::list_paginated(list_query).await {
        Ok(result) => {
            let total_pages = if limit > 0 {
                (result.total + limit - 1) / limit
            } else {
                0
            };

            let items = result
                .items
                .into_iter()
                .map(|p| MarketplaceProductListItemDto {
                    id: p.base.id.0.to_string(),
                    code: p.base.code,
                    description: p.base.description,
                    marketplace_ref: p.marketplace_ref,
                    connection_mp_ref: p.connection_mp_ref,
                    marketplace_sku: p.marketplace_sku,
                    barcode: p.barcode,
                    article: p.article,
                    nomenclature_ref: p.nomenclature_ref,
                    is_posted: p.base.metadata.is_posted,
                    created_at: p.base.metadata.created_at.to_rfc3339(),
                })
                .collect();

            Ok(Json(PaginatedMarketplaceProductResponse {
                items,
                total: result.total,
                page,
                page_size: limit,
                total_pages,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to list marketplace products: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn wb_mapping_problems(
    Query(query): Query<WbMappingProblemsQuery>,
) -> Result<Json<WbMappingProblemsResponse>, axum::http::StatusCode> {
    use a007_marketplace_product::repository::WbMappingProblemsQuery as RepositoryQuery;

    let today = chrono::Utc::now().date_naive();
    let default_from = today - chrono::Duration::days(30);
    let date_from = query
        .date_from
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default_from.to_string());
    let date_to = query
        .date_to
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| today.to_string());

    let repository_query = RepositoryQuery {
        date_from: date_from.clone(),
        date_to: date_to.clone(),
        connection_mp_ref: query.connection_mp_ref,
        limit: query.limit.unwrap_or(500),
        offset: query.offset.unwrap_or(0),
    };

    match a007_marketplace_product::service::list_wb_mapping_problems(repository_query).await {
        Ok(result) => {
            let items = result
                .items
                .into_iter()
                .map(|row| WbMappingProblemDto {
                    problem_kind: row.problem_kind,
                    connection_mp_ref: row.connection_mp_ref,
                    connection_name: row.connection_name,
                    nm_id: row.nm_id,
                    supplier_article: row.supplier_article,
                    marketplace_product_id: row.marketplace_product_id,
                    marketplace_sku: row.marketplace_sku,
                    marketplace_article: row.marketplace_article,
                    marketplace_nomenclature_ref: row.marketplace_nomenclature_ref,
                    nomenclature_name: row.nomenclature_name,
                    nomenclature_article: row.nomenclature_article,
                    p903_rows: row.p903_rows,
                    order_rows: row.order_rows,
                    sale_rows: row.sale_rows,
                    missing_document_links: row.missing_document_links,
                    mismatched_document_links: row.mismatched_document_links,
                    article_match_count: row.article_match_count,
                })
                .collect();

            Ok(Json(WbMappingProblemsResponse {
                items,
                total: result.total,
                date_from,
                date_to,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to list WB mapping problems: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct WbStalePostingsQuery {
    pub connection_mp_ref: String,
    pub nm_id: i64,
    pub supplier_article: Option<String>,
    pub date_from: String,
    pub date_to: String,
}

pub async fn wb_stale_postings_summary(
    Query(query): Query<WbStalePostingsQuery>,
) -> Result<Json<WbStalePostingsSummary>, axum::http::StatusCode> {
    let supplier_article = query
        .supplier_article
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());

    match a007_marketplace_product::service::get_stale_postings_summary(
        &query.connection_mp_ref,
        query.nm_id,
        supplier_article,
        &query.date_from,
        &query.date_to,
    )
    .await
    {
        Ok(summary) => Ok(Json(summary)),
        Err(e) => {
            tracing::error!("Failed to get stale postings summary: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn wb_stale_postings_repost(
    Json(req): Json<WbStalePostingsRequest>,
) -> Result<Json<WbStalePostingsRepostResponse>, axum::http::StatusCode> {
    let supplier_article = req
        .supplier_article
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());

    match a007_marketplace_product::service::repost_stale_postings(
        &req.connection_mp_ref,
        req.nm_id,
        supplier_article,
        &req.date_from,
        &req.date_to,
    )
    .await
    {
        Ok(resp) => Ok(Json(resp)),
        Err(e) => {
            tracing::error!("Failed to repost stale postings: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/marketplace_product
pub async fn list_all() -> Result<
    Json<Vec<contracts::domain::a007_marketplace_product::aggregate::MarketplaceProduct>>,
    axum::http::StatusCode,
> {
    match a007_marketplace_product::service::list_all().await {
        Ok(v) => Ok(Json(v)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /api/marketplace_product/:id
pub async fn get_by_id(
    Path(id): Path<String>,
) -> Result<
    Json<contracts::domain::a007_marketplace_product::aggregate::MarketplaceProduct>,
    axum::http::StatusCode,
> {
    let uuid = match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
    };
    match a007_marketplace_product::service::get_by_id(uuid).await {
        Ok(Some(v)) => Ok(Json(v)),
        Ok(None) => Err(axum::http::StatusCode::NOT_FOUND),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/marketplace_product
pub async fn upsert(
    Json(dto): Json<contracts::domain::a007_marketplace_product::aggregate::MarketplaceProductDto>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let result = if dto.id.is_some() {
        a007_marketplace_product::service::update(dto)
            .await
            .map(|_| uuid::Uuid::nil().to_string())
    } else {
        a007_marketplace_product::service::create(dto)
            .await
            .map(|id| id.to_string())
    };
    match result {
        Ok(id) => Ok(Json(json!({"id": id}))),
        Err(e) => {
            tracing::error!("Failed to save marketplace_product: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// DELETE /api/marketplace_product/:id
pub async fn delete(Path(id): Path<String>) -> Result<(), axum::http::StatusCode> {
    let uuid = match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => return Err(axum::http::StatusCode::BAD_REQUEST),
    };
    match a007_marketplace_product::service::delete(uuid).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(axum::http::StatusCode::NOT_FOUND),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/marketplace_product/testdata
pub async fn insert_test_data() -> axum::http::StatusCode {
    match a007_marketplace_product::service::insert_test_data().await {
        Ok(_) => axum::http::StatusCode::OK,
        Err(e) => {
            tracing::error!("Failed to insert test data: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
