//! Universal Dashboard component

mod json_tab;
mod result_tab;
mod saved_tab;
mod settings_tab;
mod sql_tab;
mod tabs_container;

use contracts::shared::universal_dashboard::{
    DashboardConfig, DashboardFilters, DataSourceSchemaOwned, ExecuteDashboardRequest,
    ExecuteDashboardResponse, SaveDashboardConfigRequest, SavedDashboardConfigSummary, SchemaInfo,
    UpdateDashboardConfigRequest,
};
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::{Button, ButtonAppearance, Flex, FlexGap, Select};

use super::SaveConfigDialog;
use super::SchemaPicker;
use crate::shared::universal_dashboard::api;
use tabs_container::TabsContainer;

/// Universal Dashboard that works with any schema
#[component]
pub fn UniversalDashboard(
    /// Optional: Initial schema to load on mount (e.g., "p903_wb_finance_report")
    #[prop(optional)]
    initial_schema_id: Option<String>,
    /// Optional: If true, hides schema picker and locks to initial schema
    #[prop(default = false)]
    fixed_schema: bool,
    /// Optional: Custom title (default: "Сводные таблицы")
    #[prop(optional)]
    title: Option<String>,
    /// Optional: Custom subtitle
    #[prop(optional)]
    subtitle: Option<String>,
) -> impl IntoView {
    // Schema selection state
    let (schemas, set_schemas) = signal(Vec::<SchemaInfo>::new());
    let selected_schema_id = RwSignal::new(initial_schema_id.clone());
    let (schema, set_schema) = signal(None::<DataSourceSchemaOwned>);

    // Dashboard state
    let (config, set_config) = signal(DashboardConfig {
        data_source: String::new(),
        selected_fields: vec![],
        groupings: vec![],
        display_fields: vec![],
        filters: DashboardFilters::default(),
        sort: Default::default(),
        enabled_fields: vec![],
    });

    let (response, set_response) = signal(None::<ExecuteDashboardResponse>);
    let (saved_configs, set_saved_configs) = signal(Vec::<SavedDashboardConfigSummary>::new());
    let (show_save_dialog, set_show_save_dialog) = signal(false);
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(None::<String>);
    let active_tab = RwSignal::new("settings");
    let (generated_sql, set_generated_sql) =
        signal(None::<contracts::shared::universal_dashboard::GenerateSqlResponse>);

    // Current loaded config ID and name (for "Save" button)
    let current_config_id = RwSignal::new(None::<String>);
    let current_config_name = RwSignal::new(None::<String>);

    // Selected config for picker dropdown
    let selected_config_picker = RwSignal::new(String::new());

    // Filter state for settings table - "show only selected" switch
    let show_only_selected = RwSignal::new(false);

    // Flag to prevent Effect loops during config loading
    let is_loading_config = RwSignal::new(false);

    // Load schemas on mount (only if not fixed)
    Effect::new(move |_| {
        if !fixed_schema {
            spawn_local(async move {
                match api::list_schemas().await {
                    Ok(resp) => set_schemas.set(resp.schemas),
                    Err(e) => log!("Failed to load schemas: {}", e),
                }
            });
        }
    });

    // Load schema when selection changes
    Effect::new(move |_| {
        let schema_id_opt = selected_schema_id.get();
        log!(
            "[UniversalDashboard] Schema selection changed: {:?}",
            schema_id_opt
        );

        if let Some(schema_id) = schema_id_opt {
            let schema_id_for_schema = schema_id.clone();
            let schema_id_for_configs = schema_id.clone();

            spawn_local(async move {
                log!(
                    "[UniversalDashboard] Loading schema: {}",
                    schema_id_for_schema
                );

                // Load schema definition
                match api::get_schema(&schema_id_for_schema).await {
                    Ok(resp) => {
                        log!(
                            "[UniversalDashboard] Schema loaded: {} with {} fields",
                            schema_id_for_schema,
                            resp.schema.fields.len()
                        );

                        // Initialize enabled_fields with all field IDs by default
                        let all_field_ids =
                            resp.schema.fields.iter().map(|f| f.id.clone()).collect();

                        set_schema.set(Some(resp.schema.clone()));
                        // Reset config for new schema
                        set_config.set(DashboardConfig {
                            data_source: schema_id_for_schema.clone(),
                            selected_fields: vec![],
                            groupings: vec![],
                            display_fields: vec![],
                            filters: DashboardFilters::default(),
                            sort: Default::default(),
                            enabled_fields: all_field_ids,
                        });
                        set_response.set(None);
                        set_generated_sql.set(None);
                        // Reset current config when schema changes
                        current_config_id.set(None);
                        current_config_name.set(None);
                        selected_config_picker.set(String::new());
                    }
                    Err(e) => log!("[UniversalDashboard] Failed to load schema: {}", e),
                }

                // Load saved configs for this schema
                log!(
                    "[UniversalDashboard] Loading configs for schema: {}",
                    schema_id_for_configs
                );
                match api::list_configs(Some(&schema_id_for_configs)).await {
                    Ok(resp) => {
                        log!(
                            "[UniversalDashboard] Loaded {} configs for schema: {}",
                            resp.configs.len(),
                            schema_id_for_configs
                        );
                        set_saved_configs.set(resp.configs);
                    }
                    Err(e) => {
                        log!(
                            "[UniversalDashboard] Failed to load configs for schema {}: {}",
                            schema_id_for_configs,
                            e
                        );
                    }
                }
            });
        } else {
            log!("[UniversalDashboard] No schema selected, clearing state");
            set_schema.set(None);
            set_saved_configs.set(vec![]);
        }
    });

    // Load a saved config
    let load_config = Callback::new(move |config_id: String| {
        log!("[load_config] Starting load for: {}", config_id);
        is_loading_config.set(true);
        spawn_local(async move {
            match api::get_config(&config_id).await {
                Ok(saved_config) => {
                    log!(
                        "[load_config] Config loaded successfully: {}",
                        saved_config.id
                    );
                    let id = saved_config.id.clone();
                    current_config_id.set(Some(saved_config.id.clone()));
                    current_config_name.set(Some(saved_config.name.clone()));
                    set_config.set(saved_config.config);
                    active_tab.set("settings");
                    // Sync picker with loaded config
                    selected_config_picker.set(id);

                    // Reset loading flag after giving Effect 1 (Config→UI) time to complete
                    // Use nested spawn_local to defer until next tick
                    spawn_local(async move {
                        // Small delay to ensure all Effects have processed
                        gloo_timers::future::TimeoutFuture::new(50).await;
                        is_loading_config.set(false);
                        log!("[load_config] Loading flag reset");
                    });
                }
                Err(e) => {
                    log!("Failed to load config: {}", e);
                    is_loading_config.set(false);
                }
            }
        });
    });

    // Load config when selected from picker (always reload, even if same)
    Effect::new(move |prev: Option<String>| {
        let selected = selected_config_picker.get();

        // Skip first run (initialization)
        if prev.is_none() {
            return selected.clone();
        }

        // Only load if something is selected (not empty) and value changed
        if !selected.is_empty() && Some(&selected) != prev.as_ref() {
            log!("Loading config from picker: {}", selected);
            load_config.run(selected.clone());
        }

        selected
    });

    // Sync picker with current_config_id (reset picker when config is cleared)
    Effect::new(move |_| {
        if current_config_id.get().is_none() && !selected_config_picker.get().is_empty() {
            selected_config_picker.set(String::new());
        }
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
                    active_tab.set("result");
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    };

    // Delete a config
    let delete_config = Callback::new(move |config_id: String| {
        let schema_id = selected_schema_id.get();
        spawn_local(async move {
            match api::delete_config(&config_id).await {
                Ok(_) => {
                    if let Some(sid) = schema_id {
                        if let Ok(resp) = api::list_configs(Some(&sid)).await {
                            set_saved_configs.set(resp.configs);
                        }
                    }
                }
                Err(e) => log!("Failed to delete config: {}", e),
            }
        });
    });

    // Save current config as new
    let save_config = move |(name, description): (String, Option<String>)| {
        let mut current_config = config.get();

        // Filter config to only include enabled fields
        // NOTE: Conditions are preserved even for disabled fields
        if !current_config.enabled_fields.is_empty() {
            current_config
                .groupings
                .retain(|g| current_config.enabled_fields.contains(g));
            current_config
                .display_fields
                .retain(|d| current_config.enabled_fields.contains(d));
            current_config
                .selected_fields
                .retain(|sf| current_config.enabled_fields.contains(&sf.field_id));
            // Do NOT filter conditions - they should be preserved even if field is disabled
        }

        let schema_id = selected_schema_id.get();
        spawn_local(async move {
            let request = SaveDashboardConfigRequest {
                name: name.clone(),
                description: description.clone(),
                config: current_config,
            };

            match api::save_config(request).await {
                Ok(resp) => {
                    set_show_save_dialog.set(false);
                    // Set as current config
                    let config_id = resp.id.clone();
                    current_config_id.set(Some(config_id.clone()));
                    current_config_name.set(Some(name));
                    selected_config_picker.set(config_id);
                    if let Some(sid) = schema_id {
                        if let Ok(resp) = api::list_configs(Some(&sid)).await {
                            set_saved_configs.set(resp.configs);
                        }
                    }
                }
                Err(e) => log!("Failed to save config: {}", e),
            }
        });
    };

    // Update current config (overwrite existing)
    let update_current_config = Callback::new(move |_: ()| {
        let config_id = current_config_id.get();
        let config_name = current_config_name.get();
        if let (Some(id), Some(name)) = (config_id, config_name) {
            let mut current_config = config.get();

            // Filter config to only include enabled fields
            // NOTE: Conditions are preserved even for disabled fields
            if !current_config.enabled_fields.is_empty() {
                current_config
                    .groupings
                    .retain(|g| current_config.enabled_fields.contains(g));
                current_config
                    .display_fields
                    .retain(|d| current_config.enabled_fields.contains(d));
                current_config
                    .selected_fields
                    .retain(|sf| current_config.enabled_fields.contains(&sf.field_id));
                // Do NOT filter conditions - they should be preserved even if field is disabled
            }

            let schema_id = selected_schema_id.get();
            spawn_local(async move {
                let request = UpdateDashboardConfigRequest {
                    id: id.clone(),
                    name,
                    description: None,
                    config: current_config,
                };

                match api::update_config(&id, request).await {
                    Ok(_) => {
                        log!("Config updated successfully");
                        if let Some(sid) = schema_id {
                            if let Ok(resp) = api::list_configs(Some(&sid)).await {
                                set_saved_configs.set(resp.configs);
                            }
                        }
                    }
                    Err(e) => log!("Failed to update config: {}", e),
                }
            });
        }
    });

    // Handle schema selection change
    let on_schema_change = Callback::new(move |schema_id: String| {
        log!("Schema selected: {}", schema_id);
    });

    let display_title = title
        .clone()
        .unwrap_or_else(|| "Универсальный дашборд".to_string());
    let display_subtitle = subtitle.clone();

    view! {
        <div class="page">
            <div class="page__header">
                <div class="page__header-left" style="display: flex; flex-direction: row; gap: 32px;">
                    <h1 class="page__title">{display_title}</h1>
                    {display_subtitle.as_ref().map(|s| view! {
                        <p class="page__subtitle">{s.clone()}</p>
                    })}
                    <Show when=move || !fixed_schema>
                        <SchemaPicker
                            schemas=schemas
                            selected=selected_schema_id
                            on_change=on_schema_change
                        />
                    </Show>
                    <Show when=move || !saved_configs.get().is_empty() && selected_schema_id.get().is_some()>
                        <div class="config-picker-wrapper">
                            <Select value=selected_config_picker>
                                <option value="">"-- Выбрать настройку --"</option>
                                <For
                                    each=move || saved_configs.get()
                                    key=|cfg| cfg.id.clone()
                                    children=move |cfg: SavedDashboardConfigSummary| {
                                        let id = cfg.id.clone();
                                        let name = cfg.name.clone();
                                        view! {
                                            <option value=id>{name}</option>
                                        }
                                    }
                                />
                            </Select>
                        </div>
                    </Show>
                </div>
                <div class="page__header-right">
                    <Show when=move || selected_schema_id.get().is_some()>
                        <Button
                            appearance=ButtonAppearance::Primary
                            on_click=move |_| execute_query(())
                            disabled=move || {
                                config.get().selected_fields.is_empty()
                                    && config.get().groupings.is_empty()
                            }
                        >
                            "Обновить"
                        </Button>
                    </Show>
                </div>
            </div>

            // Tabs + Content
            {move || {
                if selected_schema_id.get().is_some() {
                    view! {
                        <TabsContainer
                            active_tab=active_tab
                            schema=schema
                            response=response
                            saved_configs=saved_configs
                            generated_sql=generated_sql
                            config=config
                            loading=loading
                            error=error
                            current_config_id=current_config_id.read_only()
                            show_only_selected=show_only_selected
                            is_loading_config=is_loading_config
                            set_config=set_config
                            set_show_save_dialog=set_show_save_dialog
                            set_generated_sql=set_generated_sql
                            load_config=load_config
                            delete_config=delete_config
                            on_save_current=update_current_config
                        />
                    }.into_any()
                } else {
                    ().into_any()
                }
            }}

            <SaveConfigDialog
                show=show_save_dialog
                _config=config.get()
                on_save=Callback::new(save_config)
                on_cancel=Callback::new(move |_| set_show_save_dialog.set(false))
            />
        </div>
    }
}
