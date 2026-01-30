use leptos::prelude::*;
use crate::shared::universal_dashboard::UniversalDashboard;

/// D401 WB Finance Dashboard - wrapper around UniversalDashboard
#[component]
pub fn D401WbFinanceDashboard() -> impl IntoView {
    view! {
        <UniversalDashboard
            initial_schema_id="p903_wb_finance_report".to_string()
            fixed_schema=true
            title="Финансовый отчет Wildberries".to_string()
            subtitle="Данные из регистра P903 WB Finance Report".to_string()
        />
    }
}
