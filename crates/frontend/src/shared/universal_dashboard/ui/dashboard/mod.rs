//! Universal Dashboard component
//!
//! Two-level view architecture:
//! - Level 1 (result): Full-page report display
//! - Level 2 (settings): Configuration tabs (Настройки, Сохранённые, SQL, JSON)
//!
//! Header buttons: [Настройки] [Обновить] [Сохранить CSV] [Закрыть]

mod json_tab;
pub mod result_tab;
mod saved_tab;
mod settings_tab;
mod sql_tab;
mod tabs_container;

use contracts::shared::universal_dashboard::{
    DashboardConfig, DashboardFilters, ExecuteDashboardRequest, ExecuteDashboardResponse,
    SaveDashboardConfigRequest, SavedDashboardConfigSummary, SchemaInfo,
    UpdateDashboardConfigRequest,
};
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::{Button, ButtonAppearance, Select, Space};

use super::pivot_table;
use super::SaveConfigDialog;
use super::SchemaPicker;
use crate::shared::universal_dashboard::api;
use tabs_container::TabsContainer;

/// Universal Dashboard that works with any schema
#[component]
pub fn UniversalDashboard(
    /// Optional: Initial schema to load on mount
    #[prop(optional)]
    initial_schema_id: Option<String>,
    /// Optional: Initial config to auto-load on mount
    #[prop(optional)]
    initial_config_id: Option<String>,
    /// Optional: If true, hides schema picker
    #[prop(default = false)]
    fixed_schema: bool,
    /// Optional: Custom title
    #[prop(optional)]
    title: Option<String>,
    /// Optional: Custom subtitle
    #[prop(optional)]
    subtitle: Option<String>,
    /// Optional: Callback to close the tab
    #[prop(optional)]
    on_close: Option<Callback<()>>,
) -> impl IntoView {
    // Is this an "automatic" open (from all_reports with both schema + config)?
    let is_auto_mode = initial_schema_id.is_some() && initial_config_id.is_some();

    // ═══════════════════════════════════════════════════════════════
    // STATE
    // ═══════════════════════════════════════════════════════════════

    // Schema
    let (schemas, set_schemas) = signal(Vec::<SchemaInfo>::new());
    let selected_schema_id = RwSignal::new(initial_schema_id.clone());
    let (schema, set_schema) = signal(None::<contracts::shared::universal_dashboard::DataSourceSchemaOwned>);

    // Config
    let (config, set_config) = signal(DashboardConfig {
        data_source: String::new(),
        selected_fields: vec![],
        groupings: vec![],
        display_fields: vec![],
        filters: DashboardFilters::default(),
        sort: Default::default(),
        enabled_fields: vec![],
    });

    // Result
    let (response, set_response) = signal(None::<ExecuteDashboardResponse>);
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(None::<String>);

    // Saved configs
    let (saved_configs, set_saved_configs) = signal(Vec::<SavedDashboardConfigSummary>::new());
    let (show_save_dialog, set_show_save_dialog) = signal(false);

    // Current loaded config metadata
    let current_config_id = RwSignal::new(None::<String>);
    let current_config_name = RwSignal::new(None::<String>);

    // Config picker in header
    let selected_config_picker = RwSignal::new(String::new());

    // Settings sub-tabs
    let active_tab = RwSignal::new("settings");
    let (generated_sql, set_generated_sql) =
        signal(None::<contracts::shared::universal_dashboard::GenerateSqlResponse>);

    // Filter state for settings table
    let show_only_selected = RwSignal::new(false);

    // Flag: prevents FieldRow Effects from treating config-load as user edits
    let is_loading_config = RwSignal::new(false);

    // Flag: initial config auto-loaded once
    let initial_config_loaded = RwSignal::new(false);

    // ═══════════════════════════════════════════════════════════════
    // VIEW MODE: "result" or "settings"
    // In auto mode, start with "result" (will show loading/result).
    // In interactive mode, start with "settings".
    // ═══════════════════════════════════════════════════════════════
    let view_mode = RwSignal::new(if is_auto_mode { "result" } else { "settings" });

    // ═══════════════════════════════════════════════════════════════
    // (1) LOAD CONFIG — single callback used everywhere
    // ═══════════════════════════════════════════════════════════════
    let load_config = Callback::new(move |config_id: String| {
        log!("[load_config] Loading: {}", config_id);
        is_loading_config.set(true);
        spawn_local(async move {
            match api::get_config(&config_id).await {
                Ok(saved_config) => {
                    log!("[load_config] OK: {} ({})", saved_config.name, saved_config.id);
                    current_config_id.set(Some(saved_config.id.clone()));
                    current_config_name.set(Some(saved_config.name.clone()));
                    set_config.set(saved_config.config);
                    selected_config_picker.set(saved_config.id);
                    // Brief delay so FieldRow Effects see is_loading_config=true and skip
                    spawn_local(async move {
                        gloo_timers::future::TimeoutFuture::new(50).await;
                        is_loading_config.set(false);
                    });
                }
                Err(e) => {
                    log!("[load_config] Failed: {}", e);
                    is_loading_config.set(false);
                }
            }
        });
    });

    // ═══════════════════════════════════════════════════════════════
    // (2) EXECUTE QUERY — runs dashboard query, switches to result view
    // ═══════════════════════════════════════════════════════════════
    let execute_query = Callback::new(move |_: ()| {
        set_loading.set(true);
        set_error.set(None);
        view_mode.set("result");

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
    });

    // ═══════════════════════════════════════════════════════════════
    // (3) LOAD SCHEMAS LIST — for SchemaPicker dropdown
    // ═══════════════════════════════════════════════════════════════
    Effect::new(move |_| {
        if !fixed_schema {
            spawn_local(async move {
                match api::list_schemas().await {
                    Ok(resp) => set_schemas.set(resp.schemas),
                    Err(e) => log!("[schemas] Failed: {}", e),
                }
            });
        }
    });

    // ═══════════════════════════════════════════════════════════════
    // (4) SCHEMA CHANGE → full reset, load schema, load configs, auto-load
    // ═══════════════════════════════════════════════════════════════
    let initial_config_id_for_effect = initial_config_id.clone();
    Effect::new(move |_| {
        let initial_config_id = initial_config_id_for_effect.clone();
        let schema_id_opt = selected_schema_id.get();
        log!("[schema] Changed: {:?}", schema_id_opt);

        if let Some(schema_id) = schema_id_opt {
            let sid = schema_id.clone();
            spawn_local(async move {
                // Step A: Load schema definition
                let schema_data = match api::get_schema(&sid).await {
                    Ok(resp) => resp.schema,
                    Err(e) => {
                        log!("[schema] Failed to load: {}", e);
                        return;
                    }
                };
                log!("[schema] Loaded: {} ({} fields)", sid, schema_data.fields.len());

                let all_field_ids: Vec<String> =
                    schema_data.fields.iter().map(|f| f.id.clone()).collect();

                // Full interface reset for new schema
                set_schema.set(Some(schema_data));
                set_config.set(DashboardConfig {
                    data_source: sid.clone(),
                    selected_fields: vec![],
                    groupings: vec![],
                    display_fields: vec![],
                    filters: DashboardFilters::default(),
                    sort: Default::default(),
                    enabled_fields: all_field_ids,
                });
                set_response.set(None);
                set_generated_sql.set(None);
                current_config_id.set(None);
                current_config_name.set(None);
                selected_config_picker.set(String::new());

                // Step B: Load saved configs for picker
                match api::list_configs(Some(&sid)).await {
                    Ok(resp) => {
                        log!("[schema] {} saved configs for {}", resp.configs.len(), sid);
                        set_saved_configs.set(resp.configs);
                    }
                    Err(e) => log!("[schema] Failed to load configs: {}", e),
                }

                // Step C: Auto-load initial config (one-time, "automatic" mode)
                if let Some(ref init_cid) = initial_config_id {
                    if !initial_config_loaded.get_untracked() {
                        initial_config_loaded.set(true);
                        log!("[schema] Auto-loading config: {}", init_cid);
                        load_config.run(init_cid.clone());

                        // Step D: Auto-execute query after config loads
                        // Wait for load_config to complete (is_loading_config goes false)
                        let init_cid_for_exec = init_cid.clone();
                        spawn_local(async move {
                            // Wait for config load to finish
                            gloo_timers::future::TimeoutFuture::new(200).await;
                            log!("[schema] Auto-executing query after config: {}", init_cid_for_exec);
                            execute_query.run(());
                        });
                    }
                }
            });
        } else {
            log!("[schema] Cleared");
            set_schema.set(None);
            set_saved_configs.set(vec![]);
        }
    });

    // ═══════════════════════════════════════════════════════════════
    // (5) CONFIG PICKER — load config on USER selection only
    // ═══════════════════════════════════════════════════════════════
    Effect::new(move |prev: Option<String>| {
        let selected = selected_config_picker.get();
        if prev.is_none() {
            return selected;
        }
        if is_loading_config.get_untracked() {
            return selected;
        }
        if !selected.is_empty() && Some(&selected) != prev.as_ref() {
            log!("[picker] User selected config: {}", selected);
            load_config.run(selected.clone());
        }
        selected
    });

    // ═══════════════════════════════════════════════════════════════
    // ACTIONS
    // ═══════════════════════════════════════════════════════════════

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

    let save_config = move |(name, description): (String, Option<String>)| {
        let mut current_config = config.get();
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
        }

        let schema_id = selected_schema_id.get();
        spawn_local(async move {
            match api::save_config(SaveDashboardConfigRequest {
                name: name.clone(),
                description: description.clone(),
                config: current_config,
            })
            .await
            {
                Ok(resp) => {
                    set_show_save_dialog.set(false);
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

    let update_current_config = Callback::new(move |_: ()| {
        let config_id = current_config_id.get();
        let config_name = current_config_name.get();
        if let (Some(id), Some(name)) = (config_id, config_name) {
            let mut current_config = config.get();
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
            }

            let schema_id = selected_schema_id.get();
            spawn_local(async move {
                match api::update_config(
                    &id,
                    UpdateDashboardConfigRequest {
                        id: id.clone(),
                        name,
                        description: None,
                        config: current_config,
                    },
                )
                .await
                {
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

    let on_schema_change = Callback::new(move |schema_id: String| {
        log!("Schema selected: {}", schema_id);
    });

    // ═══════════════════════════════════════════════════════════════
    // VIEW
    // ═══════════════════════════════════════════════════════════════

    let display_title = title
        .clone()
        .unwrap_or_else(|| "Универсальный дашборд".to_string());
    let display_subtitle = subtitle.clone();

    view! {
        <div class="page">
            // ── HEADER ──
            <div class="page__header">
                <div class="page__header-left" style="display: flex; flex-direction: row; gap: 16px; align-items: center;">
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
                                <option value="">"-- Настройка --"</option>
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
                        <Space>
                            // Настройки — toggle to settings view
                            <Button
                                appearance=move || if view_mode.get() == "settings" { ButtonAppearance::Primary } else { ButtonAppearance::Subtle }
                                on_click=move |_| view_mode.set("settings")
                            >
                                "Настройки"
                            </Button>
                            // Обновить — execute query and show result
                            <Button
                                appearance=ButtonAppearance::Primary
                                on_click=move |_| execute_query.run(())
                                disabled=loading
                            >
                                {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                            </Button>
                            // Сохранить CSV — export result
                            <Show when=move || response.get().is_some()>
                                <Button
                                    appearance=ButtonAppearance::Subtle
                                    on_click=move |_| {
                                        if let Some(resp) = response.get() {
                                            let csv = pivot_table::generate_csv(&resp);
                                            pivot_table::download_csv(&csv, "report.csv");
                                        }
                                    }
                                >
                                    "Сохранить CSV"
                                </Button>
                            </Show>
                            // Закрыть — close tab (if on_close provided)
                            {on_close.map(|close_cb| view! {
                                <Button
                                    appearance=ButtonAppearance::Subtle
                                    on_click=move |_| close_cb.run(())
                                >
                                    "Закрыть"
                                </Button>
                            })}
                        </Space>
                    </Show>
                </div>
            </div>

            // ── CONTENT: two-level view ──
            {move || {
                if selected_schema_id.get().is_none() {
                    return view! {
                        <div class="empty-state" style="padding: 48px; text-align: center;">
                            <p>"Выберите схему данных"</p>
                        </div>
                    }.into_any();
                }

                if view_mode.get() == "result" {
                    // Level 1: Full-page result
                    view! {
                        <div class="dashboard-result-view">
                            <result_tab::ResultTab loading=loading error=error response=response />
                        </div>
                    }.into_any()
                } else {
                    // Level 2: Settings tabs
                    view! {
                        <TabsContainer
                            active_tab=active_tab
                            schema=schema
                            saved_configs=saved_configs
                            generated_sql=generated_sql
                            config=config
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
