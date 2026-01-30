//! Settings Tab - configuration interface

use leptos::prelude::*;
use contracts::shared::pivot::{DashboardConfig, DataSourceSchemaOwned};
use crate::shared::pivot::ui::SettingsTable;

#[component]
pub fn SettingsTab(
    #[prop(into)] config: Signal<DashboardConfig>,
    schema: DataSourceSchemaOwned,
    on_config_change: Callback<DashboardConfig>,
    on_save: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="settings-tab">
            <SettingsTable
                config=config
                schema=schema
                on_config_change=on_config_change
                on_save=on_save
            />
        </div>
    }
}
