use contracts::shared::bi_timeline::{
    BiTimelineIndicatorsResponse, BiTimelineRequest, BiTimelineResponse,
};
use gloo_net::http::Request;

use crate::shared::api_utils::api_base;

pub async fn fetch_indicators() -> Result<BiTimelineIndicatorsResponse, String> {
    let url = format!("{}/api/bi-timeline/indicators", api_base());
    let resp = Request::get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|err| err.to_string())?;

    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    resp.json::<BiTimelineIndicatorsResponse>()
        .await
        .map_err(|err| err.to_string())
}

pub async fn fetch_series(req: &BiTimelineRequest) -> Result<BiTimelineResponse, String> {
    let url = format!("{}/api/bi-timeline/series", api_base());
    let body = serde_json::to_string(req).map_err(|err| err.to_string())?;
    let resp = Request::post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .body(body)
        .map_err(|err| err.to_string())?
        .send()
        .await
        .map_err(|err| err.to_string())?;

    let status = resp.status();
    let text = resp.text().await.map_err(|err| err.to_string())?;
    if !(200..300).contains(&status) {
        return Err(format!("HTTP {}: {}", status, text));
    }

    serde_json::from_str::<BiTimelineResponse>(&text).map_err(|err| err.to_string())
}
