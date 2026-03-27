use crate::shared::api_utils::api_base;
use contracts::projections::general_ledger::GeneralLedgerEntryDto;
use gloo_net::http::Request;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub struct GeneralLedgerListQuery {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub registrator_ref: Option<String>,
    pub registrator_type: Option<String>,
    pub layer: Option<String>,
    pub turnover_code: Option<String>,
    pub debit_account: Option<String>,
    pub credit_account: Option<String>,
    pub sort_by: Option<String>,
    pub sort_desc: bool,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralLedgerListResponse {
    pub entries: Vec<GeneralLedgerEntryDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

pub async fn fetch_general_ledger(
    query: &GeneralLedgerListQuery,
) -> Result<GeneralLedgerListResponse, String> {
    let mut url = format!(
        "{}/api/general-ledger?limit={}&offset={}&sort_desc={}",
        api_base(),
        query.limit,
        query.offset,
        query.sort_desc
    );

    append_query_param(&mut url, "date_from", query.date_from.as_deref());
    append_query_param(&mut url, "date_to", query.date_to.as_deref());
    append_query_param(
        &mut url,
        "registrator_ref",
        query.registrator_ref.as_deref(),
    );
    append_query_param(
        &mut url,
        "registrator_type",
        query.registrator_type.as_deref(),
    );
    append_query_param(&mut url, "layer", query.layer.as_deref());
    append_query_param(&mut url, "turnover_code", query.turnover_code.as_deref());
    append_query_param(&mut url, "debit_account", query.debit_account.as_deref());
    append_query_param(&mut url, "credit_account", query.credit_account.as_deref());
    append_query_param(&mut url, "sort_by", query.sort_by.as_deref());

    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch journal: {e}"))?;

    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }

    response
        .json::<GeneralLedgerListResponse>()
        .await
        .map_err(|e| format!("Failed to parse journal response: {e}"))
}

pub async fn fetch_general_ledger_entry_by_id(id: &str) -> Result<GeneralLedgerEntryDto, String> {
    let url = format!(
        "{}/api/general-ledger/{}",
        api_base(),
        urlencoding::encode(id)
    );
    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch journal entry: {e}"))?;

    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }

    response
        .json::<GeneralLedgerEntryDto>()
        .await
        .map_err(|e| format!("Failed to parse journal entry: {e}"))
}

fn append_query_param(url: &mut String, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        let value = value.trim();
        if !value.is_empty() {
            url.push('&');
            url.push_str(key);
            url.push('=');
            url.push_str(&urlencoding::encode(value));
        }
    }
}
