use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::logging::log;
use contracts::shared::pivot::{
    DashboardConfig, DashboardFilters, DataSourceSchemaOwned, ExecuteDashboardRequest,
    ExecuteDashboardResponse, SaveDashboardConfigRequest, SavedDashboardConfigSummary,
};

use crate::shared::pivot::{PivotTable, SaveConfigDialog, SavedConfigsList, SettingsTable, SqlViewer};
use crate::dashboards::d401_wb_finance::api;

#[component]
pub fn D401WbFinanceDashboard() -> impl IntoView {
    // State
    let (config, set_config) = create_signal(DashboardConfig {
        data_source: "p903_wb_finance_report".to_string(),
        selected_fields: vec![],
        groupings: vec![],
        display_fields: vec![],
        filters: DashboardFilters::default(),
    });

    let (response, set_response) = create_signal(None::<ExecuteDashboardResponse>);
    let (saved_configs, set_saved_configs) = create_signal(Vec::<SavedDashboardConfigSummary>::new());
    let (schema, set_schema) = create_signal(None::<DataSourceSchemaOwned>);
    let (show_save_dialog, set_show_save_dialog) = create_signal(false);
    let (loading, set_loading) = create_signal(false);
    let (error, set_error) = create_signal(None::<String>);
    let (active_tab, set_active_tab) = create_signal("settings");
    let (generated_sql, set_generated_sql) = create_signal(None::<contracts::shared::pivot::GenerateSqlResponse>);

    // Load schema and saved configs on mount
    Effect::new(move |_| {
        spawn_local(async move {
            // Load schema
            match api::get_schema("p903_wb_finance_report").await {
                Ok(resp) => set_schema.set(Some(resp.schema)),
                Err(e) => {
                    log!("Failed to load schema: {}", e);
                }
            }
            
            // Load saved configs
            match api::list_configs().await {
                Ok(resp) => set_saved_configs.set(resp.configs),
                Err(e) => {
                    log!("Failed to load configs: {}", e);
                }
            }
        });
    });

    // Execute dashboard query
    let execute_query = move |_| {
        set_loading.set(true);
        set_error.set(None);

        let current_config = config.get();
        spawn_local(async move {
            match api::execute_dashboard(ExecuteDashboardRequest {
                config: current_config,
            })
            .await
            {
                Ok(resp) => {
                    set_response.set(Some(resp));
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    };

    // Load a saved config
    let load_config = move |config_id: String| {
        spawn_local(async move {
            match api::get_config(&config_id).await {
                Ok(saved_config) => {
                    set_config.set(saved_config.config);
                }
                Err(e) => {
                    log!("Failed to load config: {}", e);
                }
            }
        });
    };

    // Delete a config
    let delete_config = move |config_id: String| {
        spawn_local(async move {
            match api::delete_config(&config_id).await {
                Ok(_) => {
                    // Reload configs list
                    if let Ok(resp) = api::list_configs().await {
                        set_saved_configs.set(resp.configs);
                    }
                }
                Err(e) => {
                    log!("Failed to delete config: {}", e);
                }
            }
        });
    };

    // Save current config
    let save_config = move |(name, description): (String, Option<String>)| {
        let current_config = config.get();
        spawn_local(async move {
            let request = SaveDashboardConfigRequest {
                name,
                description,
                config: current_config,
            };

            match api::save_config(request).await {
                Ok(_) => {
                    set_show_save_dialog.set(false);
                    // Reload configs list
                    if let Ok(resp) = api::list_configs().await {
                        set_saved_configs.set(resp.configs);
                    }
                }
                Err(e) => {
                    log!("Failed to save config: {}", e);
                }
            }
        });
    };

    view! {
        <div class="d401-dashboard">
            // Header
            <div class="dashboard-header">
                <div class="header-left">
                    <h1>"Финансовый отчет Wildberries"</h1>
                    <small class="text-muted">"Данные из регистра P903 WB Finance Report"</small>
                </div>
                <div class="header-actions">
                    <button
                        class="btn btn-outline"
                        on:click=move |_| {
                            set_show_save_dialog.set(true);
                        }
                        title="Сохранить текущую настройку"
                    >
                        <i class="icon-save"></i>
                        " Сохранить"
                    </button>
                    <button
                        class="btn btn-primary"
                        on:click=move |_| {
                            execute_query(());
                            set_active_tab.set("result");
                        }

                        disabled=move || {
                            config.get().selected_fields.is_empty()
                                && config.get().groupings.is_empty()
                        }

                        title="Выполнить запрос"
                    >
                        <i class="icon-play"></i>
                        " Выполнить"
                    </button>
                </div>
            </div>

            // Tab Bar
            <div class="dashboard-tabs">
                <button
                    class=move || {
                        if active_tab.get() == "result" {
                            "tab-button active"
                        } else {
                            "tab-button"
                        }
                    }

                    on:click=move |_| set_active_tab.set("result")
                >
                    "Результат"
                    {move || {
                        if let Some(resp) = response.get() {
                            view! {
                                <span class="tab-badge">{resp.rows.len().to_string()}</span>
                            }
                                .into_any()
                        } else {
                            view! {}.into_any()
                        }
                    }}

                </button>
                <button
                    class=move || {
                        if active_tab.get() == "settings" {
                            "tab-button active"
                        } else {
                            "tab-button"
                        }
                    }

                    on:click=move |_| set_active_tab.set("settings")
                >
                    "Текущие настройки"
                </button>
                <button
                    class=move || {
                        if active_tab.get() == "saved" {
                            "tab-button active"
                        } else {
                            "tab-button"
                        }
                    }

                    on:click=move |_| set_active_tab.set("saved")
                >
                    "Сохраненные настройки"
                    {move || {
                        let count = saved_configs.get().len();
                        if count > 0 {
                            view! {
                                <span class="tab-badge">{count.to_string()}</span>
                            }
                                .into_any()
                        } else {
                            view! {}.into_any()
                        }
                    }}

                </button>
                <button
                    class=move || {
                        if active_tab.get() == "sql" {
                            "tab-button active"
                        } else {
                            "tab-button"
                        }
                    }

                    on:click=move |_| {
                        let cfg = config.get();
                        spawn_local(async move {
                            match api::generate_sql(cfg).await {
                                Ok(resp) => {
                                    set_generated_sql.set(Some(resp));
                                }
                                Err(e) => {
                                    log!("Failed to generate SQL: {}", e);
                                }
                            }
                        });
                        set_active_tab.set("sql");
                    }
                >

                    "SQL"
                </button>
            </div>

            // Tab Content
            <div class="dashboard-tab-content">
                <Show
                    when=move || schema.get().is_some()
                    fallback=|| {
                        view! {
                            <div class="dashboard-loading">
                                <div class="spinner"></div>
                                <p>"Загрузка схемы данных..."</p>
                            </div>
                        }
                    }
                >

                    {move || {
                        let tab = active_tab.get();
                        schema
                            .get()
                            .map(|s| {
                                if tab == "result" {
                                    view! {
                                            <div class="result-tab">
                                                {move || {
                                                    if loading.get() {
                                                        view! {
                                                            <div class="results-placeholder loading-state">
                                                                <div class="spinner"></div>
                                                                <p>"Выполнение запроса..."</p>
                                                            </div>
                                                        }
                                                            .into_any()
                                                    } else if let Some(err) = error.get() {
                                                        view! {
                                                            <div class="results-placeholder error-state">
                                                                <i class="icon-alert"></i>
                                                                <p class="error-title">"Ошибка выполнения"</p>
                                                                <p class="error-message">{err}</p>
                                                            </div>
                                                        }
                                                            .into_any()
                                                    } else if response.get().is_none() {
                                                        view! {
                                                            <div class="results-placeholder empty-state">
                                                                <i class="icon-search"></i>
                                                                <p>"Настройте поля и нажмите \"Выполнить\""</p>
                                                                <small>"Перейдите на вкладку \"Текущие настройки\""</small>
                                                            </div>
                                                        }
                                                            .into_any()
                                                    } else {
                                                        view! { <PivotTable response=response /> }.into_any()
                                                    }
                                                }}

                                        </div>
                                    }
                                        .into_any()
                                } else if tab == "settings" {
                                        view! {
                                            <div class="settings-tab">
                                                <div class="settings-header">
                                                    <h2>"Настройка полей"</h2>
                                                    <p class="text-muted">
                                                        "Выберите роль каждого поля, функцию агрегации и условия фильтрации"
                                                    </p>
                                                </div>
                                                <SettingsTable
                                                    config=config
                                                    schema=s.clone()
                                                    on_config_change=Callback::new(move |new_config| {
                                                        set_config.set(new_config);
                                                    })
                                                />
                                            </div>
                                        }
                                            .into_any()
                                } else if tab == "saved" {
                                        view! {
                                            <div class="saved-tab">
                                                <div class="saved-header">
                                                    <h2>"Сохраненные настройки"</h2>
                                                    <p class="text-muted">
                                                        "Загрузите ранее сохраненную конфигурацию отчета"
                                                    </p>
                                                </div>
                                                <Show
                                                    when=move || !saved_configs.get().is_empty()
                                                    fallback=|| {
                                                        view! {
                                                            <div class="empty-state-section">
                                                                <i class="icon-bookmark"></i>
                                                                <p>"Нет сохраненных настроек"</p>
                                                                <small>
                                                                    "Настройте отчет и нажмите кнопку \"Сохранить\""
                                                                </small>
                                                            </div>
                                                        }
                                                    }
                                                >

                                                    <SavedConfigsList
                                                        configs=saved_configs
                                                        on_load=Callback::new(move |config_id| {
                                                            load_config(config_id);
                                                            set_active_tab.set("settings");
                                                        })

                                                        on_delete=Callback::new(delete_config)
                                                    />
                                                </Show>
                                            </div>
                                        }
                                            .into_any()
                                } else if tab == "sql" {
                                        view! {
                                            <div class="sql-tab">
                                                <div class="sql-header">
                                                    <h2>"SQL запрос"</h2>
                                                    <p class="text-muted">
                                                        "Сгенерированный запрос на основе текущих настроек"
                                                    </p>
                                                </div>
                                                <SqlViewer sql=generated_sql />
                                            </div>
                                        }
                                            .into_any()
                                } else {
                                        view! {
                                            <div class="result-tab">
                                                <div class="results-placeholder empty-state">
                                                    <p>"Неизвестная вкладка"</p>
                                                </div>
                                            </div>
                                        }
                                            .into_any()
                                }
                            })
                    }}

                </Show>
            </div>

            <SaveConfigDialog
                show=show_save_dialog
                _config=config.get()
                on_save=Callback::new(save_config)
                on_cancel=Callback::new(move |_| {
                    set_show_save_dialog.set(false);
                })
            />
        </div>
    }
}
