use axum::{http::StatusCode, Json};
use contracts::shared::bi_timeline::{
    BiTimelineIndicatorsResponse, BiTimelineRequest, BiTimelineResponse,
};

/// GET /api/bi-timeline/indicators
pub async fn indicators() -> Result<Json<BiTimelineIndicatorsResponse>, (StatusCode, String)> {
    crate::shared::bi_timeline::list_compatible_indicators()
        .await
        .map(Json)
        .map_err(|err| {
            tracing::error!("BI Timeline indicators error: {}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
        })
}

/// POST /api/bi-timeline/series
pub async fn series(
    Json(req): Json<BiTimelineRequest>,
) -> Result<Json<BiTimelineResponse>, (StatusCode, String)> {
    crate::shared::bi_timeline::build_timeline(req)
        .await
        .map(Json)
        .map_err(|err| {
            tracing::error!("BI Timeline series error: {}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
        })
}
