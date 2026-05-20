use contracts::dashboards::d402_wb_order_flow::WbOrderFlowResponse;
use gloo_net::http::Request;

pub async fn get_order_flow(srid: &str) -> Result<WbOrderFlowResponse, String> {
    let url = format!(
        "/api/dashboards/wb-order-flow?srid={}",
        urlencoding::encode(srid)
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
