use axum::Json;
use contracts::domain::a013_ym_order::aggregate::YmOrder;
use uuid::Uuid;

use crate::domain::a013_ym_order;

/// Handler для получения списка Yandex Market Orders
pub async fn list_orders() -> Result<Json<Vec<YmOrder>>, axum::http::StatusCode> {
    let items = a013_ym_order::service::list_all().await.map_err(|e| {
        tracing::error!("Failed to list Yandex Market orders: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(items))
}

/// Handler для получения детальной информации о Yandex Market Order
pub async fn get_order_detail(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<YmOrder>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    let item = a013_ym_order::service::get_by_id(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get Yandex Market order detail: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    Ok(Json(item))
}

