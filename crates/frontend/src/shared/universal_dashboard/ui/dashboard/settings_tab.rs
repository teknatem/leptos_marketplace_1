//! Settings Tab - configuration interface

use crate::shared::universal_dashboard::ui::SettingsTable;
use contracts::shared::universal_dashboard::{DashboardConfig, DataSourceSchemaOwned};
use leptos::prelude::*;

#[component]
pub fn SettingsTab(
    #[prop(into)] config: Signal<DashboardConfig>,
    schema: DataSourceSchemaOwned,
    #[prop(into)] current_config_id: Signal<Option<String>>,
    show_only_selected: RwSignal<bool>,
    is_loading_config: RwSignal<bool>,
    on_config_change: Callback<DashboardConfig>,
    on_save: Callback<()>,
    on_save_as: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="settings-tab">
            <SettingsTable
                config=config
                schema=schema
                current_config_id=current_config_id
                show_only_selected=show_only_selected
                is_loading_config=is_loading_config
                on_config_change=on_config_change
                on_save=on_save
                on_save_as=on_save_as
            />
        </div>
    }
}
