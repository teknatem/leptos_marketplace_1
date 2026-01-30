//! Saved Tab - saved configurations list

use leptos::prelude::*;
use contracts::shared::pivot::SavedDashboardConfigSummary;
use crate::shared::pivot::ui::SavedConfigsList;

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
