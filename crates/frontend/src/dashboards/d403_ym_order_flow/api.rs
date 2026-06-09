use contracts::dashboards::d403_ym_order_flow::YmOrderFlowResponse;
use gloo_net::http::Request;

pub async fn get_ym_order_flow(order_id: &str) -> Result<YmOrderFlowResponse, String> {
    let url = format!(
        "/api/dashboards/ym-order-flow?order_id={}",
        urlencoding::encode(order_id)
    );
    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Запрос не выполнен: {}", e))?;
    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }
    response
        .json()
        .await
        .map_err(|e| format!("Ошибка разбора ответа: {}", e))
}
