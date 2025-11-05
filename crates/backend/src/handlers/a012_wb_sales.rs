use axum::Json;
use contracts::domain::a012_wb_sales::aggregate::WbSales;
use uuid::Uuid;

use crate::domain::a012_wb_sales;

/// Handler для получения списка Wildberries Sales
pub async fn list_sales() -> Result<Json<Vec<WbSales>>, axum::http::StatusCode> {
    let items = a012_wb_sales::service::list_all().await.map_err(|e| {
        tracing::error!("Failed to list Wildberries sales: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(items))
}

/// Handler для получения детальной информации о Wildberries Sale
pub async fn get_sale_detail(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<WbSales>, axum::http::StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    let item = a012_wb_sales::service::get_by_id(uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get Wildberries sale detail: {}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    Ok(Json(item))
}

