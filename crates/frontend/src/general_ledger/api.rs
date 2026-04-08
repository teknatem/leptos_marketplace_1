use crate::shared::api_utils::api_base;
use contracts::general_ledger::{
    GeneralLedgerEntryDto, GeneralLedgerTurnoverDto, GlAccountViewQuery, GlAccountViewResponse,
    GlDimensionsResponse, GlDrilldownQuery, GlDrilldownResponse, GlDrilldownSessionCreate,
    GlDrilldownSessionCreateResponse, GlDrilldownSessionRecord, GlReportQuery, GlReportResponse,
    WbWeeklyReconciliationQuery, WbWeeklyReconciliationResponse,
};
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
    pub connection_mp_ref: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralLedgerTurnoverListResponse {
    pub items: Vec<GeneralLedgerTurnoverDto>,
    pub total: usize,
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
    append_query_param(
        &mut url,
        "connection_mp_ref",
        query.connection_mp_ref.as_deref(),
    );
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

pub async fn fetch_general_ledger_turnovers() -> Result<GeneralLedgerTurnoverListResponse, String> {
    let url = format!("{}/api/general-ledger/turnovers", api_base());
    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch GL turnovers: {e}"))?;

    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }

    response
        .json::<GeneralLedgerTurnoverListResponse>()
        .await
        .map_err(|e| format!("Failed to parse GL turnovers response: {e}"))
}

pub async fn fetch_gl_report(query: &GlReportQuery) -> Result<GlReportResponse, String> {
    let url = format!("{}/api/general-ledger/report", api_base());
    let response = Request::post(&url)
        .json(query)
        .map_err(|e| format!("Failed to serialize GL report query: {e}"))?
        .send()
        .await
        .map_err(|e| format!("Failed to fetch GL report: {e}"))?;

    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }

    response
        .json::<GlReportResponse>()
        .await
        .map_err(|e| format!("Failed to parse GL report response: {e}"))
}

pub async fn fetch_gl_dimensions(turnover_code: &str) -> Result<GlDimensionsResponse, String> {
    let url = format!(
        "{}/api/general-ledger/report/dimensions?turnover_code={}",
        api_base(),
        urlencoding::encode(turnover_code)
    );
    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch GL dimensions: {e}"))?;

    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }

    response
        .json::<GlDimensionsResponse>()
        .await
        .map_err(|e| format!("Failed to parse GL dimensions response: {e}"))
}

pub async fn fetch_gl_drilldown(query: &GlDrilldownQuery) -> Result<GlDrilldownResponse, String> {
    let url = format!("{}/api/general-ledger/report/drilldown", api_base());
    let response = Request::post(&url)
        .json(query)
        .map_err(|e| format!("Failed to serialize GL drilldown query: {e}"))?
        .send()
        .await
        .map_err(|e| format!("Failed to fetch GL drilldown: {e}"))?;

    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }

    response
        .json::<GlDrilldownResponse>()
        .await
        .map_err(|e| format!("Failed to parse GL drilldown response: {e}"))
}

pub async fn create_gl_drilldown_session(
    body: &GlDrilldownSessionCreate,
) -> Result<GlDrilldownSessionCreateResponse, String> {
    let url = format!("{}/api/general-ledger/drilldown", api_base());
    let response = Request::post(&url)
        .json(body)
        .map_err(|e| format!("Failed to serialize GL drilldown session: {e}"))?
        .send()
        .await
        .map_err(|e| format!("Failed to create GL drilldown session: {e}"))?;

    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }

    response
        .json::<GlDrilldownSessionCreateResponse>()
        .await
        .map_err(|e| format!("Failed to parse GL drilldown session response: {e}"))
}

pub async fn fetch_gl_drilldown_session(id: &str) -> Result<GlDrilldownSessionRecord, String> {
    let url = format!(
        "{}/api/general-ledger/drilldown/{}",
        api_base(),
        urlencoding::encode(id)
    );
    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch GL drilldown session: {e}"))?;

    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }

    response
        .json::<GlDrilldownSessionRecord>()
        .await
        .map_err(|e| format!("Failed to parse GL drilldown session response: {e}"))
}

pub async fn fetch_gl_drilldown_session_data(id: &str) -> Result<GlDrilldownResponse, String> {
    let url = format!(
        "{}/api/general-ledger/drilldown/{}/data",
        api_base(),
        urlencoding::encode(id)
    );
    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch GL drilldown data: {e}"))?;

    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }

    response
        .json::<GlDrilldownResponse>()
        .await
        .map_err(|e| format!("Failed to parse GL drilldown data response: {e}"))
}

pub async fn fetch_gl_account_view(
    query: &GlAccountViewQuery,
) -> Result<GlAccountViewResponse, String> {
    let url = format!("{}/api/general-ledger/account-view", api_base());
    let response = Request::post(&url)
        .json(query)
        .map_err(|e| format!("Failed to serialize GL account view query: {e}"))?
        .send()
        .await
        .map_err(|e| format!("Failed to fetch GL account view: {e}"))?;

    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }

    response
        .json::<GlAccountViewResponse>()
        .await
        .map_err(|e| format!("Failed to parse GL account view response: {e}"))
}

pub async fn fetch_wb_weekly_reconciliation(
    query: &WbWeeklyReconciliationQuery,
) -> Result<WbWeeklyReconciliationResponse, String> {
    let mut url = format!("{}/api/reports/wb-weekly-reconciliation", api_base());
    let mut has_query = false;

    let mut push = |key: &str, value: Option<&str>| {
        if let Some(value) = value {
            let value = value.trim();
            if value.is_empty() {
                return;
            }
            url.push(if has_query { '&' } else { '?' });
            has_query = true;
            url.push_str(key);
            url.push('=');
            url.push_str(&urlencoding::encode(value));
        }
    };

    push("date_from", query.date_from.as_deref());
    push("date_to", query.date_to.as_deref());
    push("connection_id", query.connection_id.as_deref());

    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch WB weekly reconciliation: {e}"))?;

    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }

    response
        .json::<WbWeeklyReconciliationResponse>()
        .await
        .map_err(|e| format!("Failed to parse WB weekly reconciliation response: {e}"))
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
