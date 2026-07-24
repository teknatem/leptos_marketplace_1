use contracts::dashboards::d406_wb_sales_funnel::{
    FunnelDateAxis, WbSalesFunnelOrdersResponse, WbSalesFunnelResponse,
};
use gloo_net::http::Request;

/// Ось агрегации → значение query-параметра (совпадает с serde snake_case).
fn axis_param(axis: FunnelDateAxis) -> &'static str {
    match axis {
        FunnelDateAxis::Cohort => "cohort",
        FunnelDateAxis::Event => "event",
    }
}

pub async fn get_wb_sales_funnel(
    date_from: &str,
    date_to: &str,
    connection_mp_ref: &str,
    nm_id: &str,
    axis: FunnelDateAxis,
) -> Result<WbSalesFunnelResponse, String> {
    let mut params = vec![format!("axis={}", axis_param(axis))];
    if !date_from.trim().is_empty() {
        params.push(format!(
            "date_from={}",
            urlencoding::encode(date_from.trim())
        ));
    }
    if !date_to.trim().is_empty() {
        params.push(format!("date_to={}", urlencoding::encode(date_to.trim())));
    }
    if !connection_mp_ref.trim().is_empty() {
        params.push(format!(
            "connection_mp_ref={}",
            urlencoding::encode(connection_mp_ref.trim())
        ));
    }
    if !nm_id.trim().is_empty() {
        params.push(format!("nm_id={}", urlencoding::encode(nm_id.trim())));
    }

    let url = format!("/api/dashboards/wb-sales-funnel?{}", params.join("&"));

    let response = Request::get(&url)
        .send()
        .await
        .map_err(|error| format!("Request failed: {error}"))?;
    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }
    response
        .json()
        .await
        .map_err(|error| format!("Failed to parse response: {error}"))
}

/// Drilldown: конкретные заказы ячейки воронки (`nm_id × дата`) с меткой канала (платн./беспл.).
pub async fn get_wb_sales_funnel_orders(
    connection_mp_ref: &str,
    nm_id: i64,
    date: &str,
) -> Result<WbSalesFunnelOrdersResponse, String> {
    let url = format!(
        "/api/dashboards/wb-sales-funnel/orders?connection_mp_ref={}&nm_id={}&date={}",
        urlencoding::encode(connection_mp_ref.trim()),
        nm_id,
        urlencoding::encode(date.trim()),
    );

    let response = Request::get(&url)
        .send()
        .await
        .map_err(|error| format!("Request failed: {error}"))?;
    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }
    response
        .json()
        .await
        .map_err(|error| format!("Failed to parse response: {error}"))
}
