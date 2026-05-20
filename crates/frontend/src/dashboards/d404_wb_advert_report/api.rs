use contracts::dashboards::d404_wb_advert_report::WbAdvertReportResponse;
use gloo_net::http::Request;

pub async fn get_wb_advert_report(
    date_from: &str,
    date_to: &str,
    wb_advert_campaign_code: &str,
    connection_mp_ref: &str,
) -> Result<WbAdvertReportResponse, String> {
    let mut params = Vec::new();
    if !date_from.trim().is_empty() {
        params.push(format!(
            "date_from={}",
            urlencoding::encode(date_from.trim())
        ));
    }
    if !date_to.trim().is_empty() {
        params.push(format!("date_to={}", urlencoding::encode(date_to.trim())));
    }
    if !wb_advert_campaign_code.trim().is_empty() {
        params.push(format!(
            "wb_advert_campaign_code={}",
            urlencoding::encode(wb_advert_campaign_code.trim())
        ));
    }
    if !connection_mp_ref.trim().is_empty() {
        params.push(format!(
            "connection_mp_ref={}",
            urlencoding::encode(connection_mp_ref.trim())
        ));
    }

    let url = if params.is_empty() {
        "/api/dashboards/wb-advert-report".to_string()
    } else {
        format!("/api/dashboards/wb-advert-report?{}", params.join("&"))
    };

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
