use axum::{extract::Path, Json};
use once_cell::sync::Lazy;
use std::sync::Arc;

use crate::usecases;

// ============================================================================
// UseCase u501: Import from UT
// ============================================================================

static IMPORT_EXECUTOR: Lazy<Arc<usecases::u501_import_from_ut::ImportExecutor>> =
    Lazy::new(|| {
        let tracker = Arc::new(usecases::u501_import_from_ut::ProgressTracker::new());
        Arc::new(usecases::u501_import_from_ut::ImportExecutor::new(tracker))
    });

/// POST /api/u501/import/start
pub async fn u501_start_import(
    Json(request): Json<contracts::usecases::u501_import_from_ut::ImportRequest>,
) -> Result<Json<contracts::usecases::u501_import_from_ut::ImportResponse>, axum::http::StatusCode>
{
    match IMPORT_EXECUTOR.start_import(request).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            tracing::error!("Failed to start import: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/u501/import/:session_id/progress
pub async fn u501_get_progress(
    Path(session_id): Path<String>,
) -> Result<
    Json<contracts::usecases::u501_import_from_ut::progress::ImportProgress>,
    axum::http::StatusCode,
> {
    match IMPORT_EXECUTOR.get_progress(&session_id) {
        Some(progress) => Ok(Json(progress)),
        None => Err(axum::http::StatusCode::NOT_FOUND),
    }
}

// ============================================================================
// UseCase u502: Import from OZON
// ============================================================================

static OZON_IMPORT_EXECUTOR: Lazy<Arc<usecases::u502_import_from_ozon::ImportExecutor>> =
    Lazy::new(|| {
        let tracker = Arc::new(usecases::u502_import_from_ozon::ProgressTracker::new());
        Arc::new(usecases::u502_import_from_ozon::ImportExecutor::new(
            tracker,
        ))
    });

/// POST /api/u502/import/start
pub async fn u502_start_import(
    Json(request): Json<contracts::usecases::u502_import_from_ozon::ImportRequest>,
) -> Result<Json<contracts::usecases::u502_import_from_ozon::ImportResponse>, axum::http::StatusCode>
{
    match OZON_IMPORT_EXECUTOR.start_import(request).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            tracing::error!("Failed to start OZON import: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/u502/import/:session_id/progress
pub async fn u502_get_progress(
    Path(session_id): Path<String>,
) -> Result<
    Json<contracts::usecases::u502_import_from_ozon::progress::ImportProgress>,
    axum::http::StatusCode,
> {
    match OZON_IMPORT_EXECUTOR.get_progress(&session_id) {
        Some(progress) => Ok(Json(progress)),
        None => Err(axum::http::StatusCode::NOT_FOUND),
    }
}

// ============================================================================
// UseCase u503: Import from Yandex Market
// ============================================================================

static YANDEX_IMPORT_EXECUTOR: Lazy<Arc<usecases::u503_import_from_yandex::ImportExecutor>> =
    Lazy::new(|| {
        let tracker = Arc::new(usecases::u503_import_from_yandex::ProgressTracker::new());
        Arc::new(usecases::u503_import_from_yandex::ImportExecutor::new(
            tracker,
        ))
    });

/// POST /api/u503/import/start
pub async fn u503_start_import(
    Json(request): Json<contracts::usecases::u503_import_from_yandex::ImportRequest>,
) -> Result<
    Json<contracts::usecases::u503_import_from_yandex::ImportResponse>,
    axum::http::StatusCode,
> {
    match YANDEX_IMPORT_EXECUTOR.start_import(request).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            tracing::error!("Failed to start Yandex Market import: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/u503/import/:session_id/progress
pub async fn u503_get_progress(
    Path(session_id): Path<String>,
) -> Result<
    Json<contracts::usecases::u503_import_from_yandex::progress::ImportProgress>,
    axum::http::StatusCode,
> {
    match YANDEX_IMPORT_EXECUTOR.get_progress(&session_id) {
        Some(progress) => Ok(Json(progress)),
        None => Err(axum::http::StatusCode::NOT_FOUND),
    }
}

// ============================================================================
// UseCase u504: Import from Wildberries
// ============================================================================

static WB_IMPORT_EXECUTOR: Lazy<Arc<usecases::u504_import_from_wildberries::ImportExecutor>> =
    Lazy::new(|| {
        let tracker = Arc::new(usecases::u504_import_from_wildberries::ProgressTracker::new());
        Arc::new(usecases::u504_import_from_wildberries::ImportExecutor::new(
            tracker,
        ))
    });

/// POST /api/u504/import/start
pub async fn u504_start_import(
    Json(request): Json<contracts::usecases::u504_import_from_wildberries::ImportRequest>,
) -> Result<
    Json<contracts::usecases::u504_import_from_wildberries::ImportResponse>,
    axum::http::StatusCode,
> {
    match WB_IMPORT_EXECUTOR.start_import(request).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            tracing::error!("Failed to start Wildberries import: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/u504/import/:session_id/progress
pub async fn u504_get_progress(
    Path(session_id): Path<String>,
) -> Result<
    Json<contracts::usecases::u504_import_from_wildberries::progress::ImportProgress>,
    axum::http::StatusCode,
> {
    match WB_IMPORT_EXECUTOR.get_progress(&session_id) {
        Some(progress) => Ok(Json(progress)),
        None => Err(axum::http::StatusCode::NOT_FOUND),
    }
}

// ============================================================================
// UseCase u505: Match Nomenclature
// ============================================================================

static MATCH_NOMENCLATURE_EXECUTOR: Lazy<Arc<usecases::u505_match_nomenclature::MatchExecutor>> =
    Lazy::new(|| {
        let tracker = Arc::new(usecases::u505_match_nomenclature::ProgressTracker::new());
        Arc::new(usecases::u505_match_nomenclature::MatchExecutor::new(
            tracker,
        ))
    });

/// POST /api/u505/match/start
pub async fn u505_start_matching(
    Json(request): Json<contracts::usecases::u505_match_nomenclature::MatchRequest>,
) -> Result<Json<contracts::usecases::u505_match_nomenclature::MatchResponse>, axum::http::StatusCode>
{
    match MATCH_NOMENCLATURE_EXECUTOR.start_matching(request).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            tracing::error!("Failed to start nomenclature matching: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/u505/match/:session_id/progress
pub async fn u505_get_progress(
    Path(session_id): Path<String>,
) -> Result<
    Json<contracts::usecases::u505_match_nomenclature::progress::MatchProgress>,
    axum::http::StatusCode,
> {
    match MATCH_NOMENCLATURE_EXECUTOR.get_progress(&session_id) {
        Some(progress) => Ok(Json(progress)),
        None => Err(axum::http::StatusCode::NOT_FOUND),
    }
}

// ============================================================================
// UseCase u506: Import from LemanaPro
// ============================================================================

static LEMANAPRO_IMPORT_EXECUTOR: Lazy<Arc<usecases::u506_import_from_lemanapro::ImportExecutor>> =
    Lazy::new(|| {
        let tracker = Arc::new(usecases::u506_import_from_lemanapro::ProgressTracker::new());
        Arc::new(usecases::u506_import_from_lemanapro::ImportExecutor::new(
            tracker,
        ))
    });

/// POST /api/u506/import/start
pub async fn u506_start_import(
    Json(request): Json<contracts::usecases::u506_import_from_lemanapro::ImportRequest>,
) -> Result<
    Json<contracts::usecases::u506_import_from_lemanapro::ImportResponse>,
    axum::http::StatusCode,
> {
    match LEMANAPRO_IMPORT_EXECUTOR.start_import(request).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            tracing::error!("Failed to start LemanaPro import: {}", e);
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/u506/import/:session_id/progress
pub async fn u506_get_progress(
    Path(session_id): Path<String>,
) -> Result<
    Json<contracts::usecases::u506_import_from_lemanapro::progress::ImportProgress>,
    axum::http::StatusCode,
> {
    match LEMANAPRO_IMPORT_EXECUTOR.get_progress(&session_id) {
        Some(progress) => Ok(Json(progress)),
        None => Err(axum::http::StatusCode::NOT_FOUND),
    }
}
