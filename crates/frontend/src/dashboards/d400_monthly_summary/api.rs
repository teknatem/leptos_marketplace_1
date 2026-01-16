use contracts::dashboards::d400_monthly_summary::MonthlySummaryResponse;
use gloo_net::http::Request;

const API_BASE: &str = "/api/d400";

/// Получить список доступных периодов (YYYY-MM)
pub async fn get_available_periods() -> Result<Vec<String>, String> {
    let url = format!("{}/periods", API_BASE);

    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.ok() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let data: Vec<String> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(data)
}

/// Получить данные сводки за месяц
pub async fn get_monthly_summary(year: i32, month: u32) -> Result<MonthlySummaryResponse, String> {
    let url = format!("{}/monthly_summary?year={}&month={}", API_BASE, year, month);

    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.ok() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let data: MonthlySummaryResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(data)
}
