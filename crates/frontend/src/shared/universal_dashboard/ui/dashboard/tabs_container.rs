//! Tab container component

use super::json_tab::JsonTab;
use super::result_tab::ResultTab;
use super::saved_tab::SavedTab;
use super::settings_tab::SettingsTab;
use super::sql_tab::SqlTab;
use crate::shared::universal_dashboard::api;
use contracts::shared::universal_dashboard::{
    DashboardConfig, DataSourceSchemaOwned, ExecuteDashboardResponse, SavedDashboardConfigSummary,
};
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::{Badge, BadgeAppearance, BadgeColor, Button, ButtonAppearance, Space};

#[component]
pub fn TabsContainer(
    active_tab: RwSignal<&'static str>,
    #[prop(into)] schema: Signal<Option<DataSourceSchemaOwned>>,
    #[prop(into)] response: Signal<Option<ExecuteDashboardResponse>>,
    #[prop(into)] saved_configs: Signal<Vec<SavedDashboardConfigSummary>>,
    #[prop(into)] generated_sql: Signal<
        Option<contracts::shared::universal_dashboard::GenerateSqlResponse>,
    >,
    #[prop(into)] config: Signal<DashboardConfig>,
    #[prop(into)] loading: Signal<bool>,
    #[prop(into)] error: Signal<Option<String>>,
    #[prop(into)] current_config_id: Signal<Option<String>>,
    show_only_selected: RwSignal<bool>,
    is_loading_config: RwSignal<bool>,
    set_config: WriteSignal<DashboardConfig>,
    set_show_save_dialog: WriteSignal<bool>,
    set_generated_sql: WriteSignal<
        Option<contracts::shared::universal_dashboard::GenerateSqlResponse>,
    >,
    load_config: Callback<String>,
    delete_config: Callback<String>,
    on_save_current: Callback<()>,
) -> impl IntoView {
    view! {
        <div>
            <div style="border-bottom: 1px solid var(--thaw-color-neutral-stroke-1); margin-bottom: 16px; padding-bottom: 4px;">
                <Space>
                    <Button
                        appearance=move || if active_tab.get() == "result" { ButtonAppearance::Primary } else { ButtonAppearance::Subtle }
                        on_click=move |_| active_tab.set("result")
                    >
                        "Результат"
                        {move || response.get().map(|resp| {
                            let count = resp.rows.len();
                            view! {
                                <span style="margin-left: 8px;">
                                    <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                                        {count.to_string()}
                                    </Badge>
                                </span>
                            }
                        })}
                    </Button>
                    <Button
                        appearance=move || if active_tab.get() == "settings" { ButtonAppearance::Primary } else { ButtonAppearance::Subtle }
                        on_click=move |_| active_tab.set("settings")
                    >
                        "Настройки"
                    </Button>
                    <Button
                        appearance=move || if active_tab.get() == "saved" { ButtonAppearance::Primary } else { ButtonAppearance::Subtle }
                        on_click=move |_| active_tab.set("saved")
                    >
                        "Сохранённые"
                        {move || {
                            let count = saved_configs.get().len();
                            (count > 0).then(|| view! {
                                <span style="margin-left: 8px;">
                                    <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                                        {count.to_string()}
                                    </Badge>
                                </span>
                            })
                        }}
                    </Button>
                    <Button
                        appearance=move || if active_tab.get() == "sql" { ButtonAppearance::Primary } else { ButtonAppearance::Subtle }
                        on_click=move |_| {
                            let cfg = config.get();
                            spawn_local(async move {
                                match api::generate_sql(cfg).await {
                                    Ok(resp) => set_generated_sql.set(Some(resp)),
                                    Err(e) => log!("Failed to generate SQL: {}", e),
                                }
                            });
                            active_tab.set("sql");
                        }
                    >
                        "SQL"
                    </Button>
                    <Button
                        appearance=move || if active_tab.get() == "json" { ButtonAppearance::Primary } else { ButtonAppearance::Subtle }
                        on_click=move |_| active_tab.set("json")
                    >
                        "JSON"
                    </Button>
                </Space>
            </div>

            <div class="dashboard-tab-content">
                <Show
                    when=move || schema.get().is_some()
                    fallback=|| view! {
                        <div class="loading-state">
                            <div class="spinner"></div>
                            <p>"Загрузка схемы..."</p>
                        </div>
                    }
                >
                    {move || schema.get().map(|s| view! {
                        <div style=move || if active_tab.get() == "result" { "" } else { "display: none;" }>
                            <ResultTab loading=loading error=error response=response />
                        </div>
                        <div style=move || if active_tab.get() == "settings" { "" } else { "display: none;" }>
                            <SettingsTab
                                config=config
                                schema=s.clone()
                                current_config_id=current_config_id
                                show_only_selected=show_only_selected
                                is_loading_config=is_loading_config
                                on_config_change=Callback::new(move |new_config| {
                                    set_config.set(new_config);
                                })
                                on_save=on_save_current
                                on_save_as=Callback::new(move |_| {
                                    set_show_save_dialog.set(true);
                                })
                            />
                        </div>
                        <div style=move || if active_tab.get() == "saved" { "" } else { "display: none;" }>
                            <SavedTab
                                saved_configs=saved_configs
                                on_load=load_config
                                on_delete=delete_config
                            />
                        </div>
                        <div style=move || if active_tab.get() == "sql" { "" } else { "display: none;" }>
                            <SqlTab generated_sql=generated_sql />
                        </div>
                        <div style=move || if active_tab.get() == "json" { "" } else { "display: none;" }>
                            <JsonTab config=config />
                        </div>
                    })}
                </Show>
            </div>
        </div>
    }
}
