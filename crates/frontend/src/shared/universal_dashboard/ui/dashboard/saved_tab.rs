//! Saved Tab - saved configurations list

use crate::shared::universal_dashboard::ui::SavedConfigsList;
use contracts::shared::universal_dashboard::SavedDashboardConfigSummary;
use leptos::prelude::*;

#[component]
pub fn SavedTab(
    #[prop(into)] saved_configs: Signal<Vec<SavedDashboardConfigSummary>>,
    on_load: Callback<String>,
    on_delete: Callback<String>,
) -> impl IntoView {
    view! {
        <div class="saved-tab">
            <SavedConfigsList
                configs=saved_configs
                on_load=on_load
                on_delete=on_delete
            />
        </div>
    }
}
