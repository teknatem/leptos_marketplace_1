//! All Reports List - список всех сохраненных отчетов (настроек дашбордов)

use contracts::shared::universal_dashboard::SavedDashboardConfigSummary;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;
use thaw::{Button, ButtonAppearance, Input, Table, TableBody, TableCell, TableCellLayout, TableHeader, TableHeaderCell, TableRow};
use uuid::Uuid;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::page_frame::PageFrame;
use crate::shared::universal_dashboard::api;

/// Format ISO datetime string to human-readable format (DD.MM.YYYY HH:MM)
fn format_datetime(iso: &str) -> String {
    if iso.len() >= 16 {
        let date_part = &iso[0..10]; // 2026-01-26
        let time_part = &iso[11..16]; // 21:57
        
        let parts: Vec<&str> = date_part.split('-').collect();
        if parts.len() == 3 {
            return format!("{}.{}.{} {}", parts[2], parts[1], parts[0], time_part);
        }
    }
    iso.to_string()
}

#[derive(Clone, Debug)]
enum SortField {
    Name,
    UpdatedAt,
}

#[derive(Clone, Debug)]
enum SortDirection {
    Asc,
    Desc,
}

#[component]
pub fn AllReportsList() -> impl IntoView {
    let ctx = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    
    // Data state
    let (configs, set_configs) = signal(Vec::<SavedDashboardConfigSummary>::new());
    let (schemas_map, set_schemas_map) = signal(HashMap::<String, String>::new());
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(None::<String>);
    
    // UI state
    let search_query = RwSignal::new(String::new());
    let sort_field = RwSignal::new(SortField::UpdatedAt);
    let sort_direction = RwSignal::new(SortDirection::Desc);
    
    // Load data on mount
    Effect::new(move |_| {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            
            // Load all configs
            let configs_result = api::list_configs(None).await;
            
            // Load schemas for name mapping
            let schemas_result = api::list_schemas().await;
            
            match (configs_result, schemas_result) {
                (Ok(configs_resp), Ok(schemas_resp)) => {
                    set_configs.set(configs_resp.configs);
                    
                    // Build schema id -> name map
                    let map: HashMap<String, String> = schemas_resp
                        .schemas
                        .into_iter()
                        .map(|s| (s.id, s.name))
                        .collect();
                    set_schemas_map.set(map);
                    
                    set_loading.set(false);
                }
                (Err(e), _) | (_, Err(e)) => {
                    set_error.set(Some(format!("Ошибка загрузки данных: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    });
    
    // Filtered and sorted configs
    let filtered_sorted_configs = Signal::derive(move || {
        let query = search_query.get().to_lowercase();
        let mut result: Vec<SavedDashboardConfigSummary> = configs
            .get()
            .into_iter()
            .filter(|cfg| {
                if query.is_empty() {
                    return true;
                }
                
                let name_match = cfg.name.to_lowercase().contains(&query);
                let desc_match = cfg
                    .description
                    .as_ref()
                    .map(|d| d.to_lowercase().contains(&query))
                    .unwrap_or(false);
                let source_match = cfg.data_source.to_lowercase().contains(&query);
                
                // Also search in human-readable schema name
                let schema_name_match = schemas_map
                    .get()
                    .get(&cfg.data_source)
                    .map(|name| name.to_lowercase().contains(&query))
                    .unwrap_or(false);
                
                name_match || desc_match || source_match || schema_name_match
            })
            .collect();
        
        // Sort
        let field = sort_field.get();
        let direction = sort_direction.get();
        
        result.sort_by(|a, b| {
            let cmp = match field {
                SortField::Name => a.name.cmp(&b.name),
                SortField::UpdatedAt => a.updated_at.cmp(&b.updated_at),
            };
            
            match direction {
                SortDirection::Asc => cmp,
                SortDirection::Desc => cmp.reverse(),
            }
        });
        
        result
    });
    
    // Toggle sort
    let toggle_sort = move |field: SortField| {
        let current_field = sort_field.get();
        let current_direction = sort_direction.get();
        
        if matches!((current_field, &field), (SortField::Name, SortField::Name) | (SortField::UpdatedAt, SortField::UpdatedAt)) {
            // Same field - toggle direction
            sort_direction.set(match current_direction {
                SortDirection::Asc => SortDirection::Desc,
                SortDirection::Desc => SortDirection::Asc,
            });
        } else {
            // Different field - set new field and default to Desc
            sort_field.set(field);
            sort_direction.set(SortDirection::Desc);
        }
    };
    
    // Open report in new tab
    let open_report = move |config: SavedDashboardConfigSummary| {
        let uuid = Uuid::new_v4();
        // Include schema_id (data_source) in the tab key for proper initialization
        let tab_key = format!("universal_dashboard_report_{}__{}__{}", uuid, config.data_source, config.id);
        let tab_title = format!("Отчет: {}", config.name);
        
        log!("Opening report: {} with key: {}", tab_title, tab_key);
        ctx.open_tab(&tab_key, &tab_title);
    };
    
    // Open details in new tab
    let open_details = move |config_id: String, config_name: String| {
        let tab_key = format!("all_reports_detail_{}", config_id);
        let tab_title = format!("Настройка: {}", config_name);
        
        log!("Opening details: {} with key: {}", tab_title, tab_key);
        ctx.open_tab(&tab_key, &tab_title);
    };
    
    view! {
        <PageFrame page_id="all_reports--list" category="list">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Все отчеты"</h1>
                </div>
                <div class="page__header-right">
                    <div style="width: 350px;">
                        <Input
                            value=search_query
                            placeholder="Поиск по названию, описанию или источнику..."
                        />
                    </div>
                </div>
            </div>
            
            <div class="page__content">
                <Show
                    when=move || !loading.get()
                    fallback=|| view! {
                        <div style="padding: 48px; text-align: center;">
                            <p>"Загрузка..."</p>
                        </div>
                    }
                >
                    <Show
                        when=move || error.get().is_none()
                        fallback=move || {
                            let err = error.get().unwrap_or_default();
                            view! {
                                <div style="padding: 24px; color: var(--thaw-color-error-foreground-1);">
                                    {err}
                                </div>
                            }
                        }
                    >
                        <Show
                            when=move || !filtered_sorted_configs.get().is_empty()
                            fallback=|| view! {
                                <div style="padding: 48px; text-align: center;">
                                    <p style="color: var(--thaw-color-neutral-foreground-2);">
                                        "Нет отчетов для отображения"
                                    </p>
                                </div>
                            }
                        >
                            <Table>
                                <TableHeader>
                                    <TableRow>
                                        <TableHeaderCell 
                                            resizable=true 
                                            min_width=200.0
                                        >
                                            <div 
                                                style="display: flex; align-items: center; gap: 4px; cursor: pointer;"
                                                on:click=move |_| toggle_sort(SortField::Name)
                                            >
                                                "Название"
                                                {move || {
                                                    if matches!(sort_field.get(), SortField::Name) {
                                                        match sort_direction.get() {
                                                            SortDirection::Asc => " ▲",
                                                            SortDirection::Desc => " ▼",
                                                        }
                                                    } else {
                                                        ""
                                                    }
                                                }}
                                            </div>
                                        </TableHeaderCell>
                                        <TableHeaderCell resizable=true min_width=300.0>
                                            "Описание"
                                        </TableHeaderCell>
                                        <TableHeaderCell resizable=true min_width=200.0>
                                            "Источник данных"
                                        </TableHeaderCell>
                                        <TableHeaderCell 
                                            min_width=150.0
                                        >
                                            <div 
                                                style="display: flex; align-items: center; gap: 4px; cursor: pointer;"
                                                on:click=move |_| toggle_sort(SortField::UpdatedAt)
                                            >
                                                "Обновлено"
                                                {move || {
                                                    if matches!(sort_field.get(), SortField::UpdatedAt) {
                                                        match sort_direction.get() {
                                                            SortDirection::Asc => " ▲",
                                                            SortDirection::Desc => " ▼",
                                                        }
                                                    } else {
                                                        ""
                                                    }
                                                }}
                                            </div>
                                        </TableHeaderCell>
                                        <TableHeaderCell min_width=100.0>
                                            "Действия"
                                        </TableHeaderCell>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    {move || {
                                        filtered_sorted_configs
                                            .get()
                                            .into_iter()
                                            .map(|cfg| {
                                                let config_id = cfg.id.clone();
                                                let config_name = cfg.name.clone();
                                                let config_description = cfg.description.clone().unwrap_or_default();
                                                let config_updated_at = cfg.updated_at.clone();
                                                let config_data_source = cfg.data_source.clone();
                                                
                                                // Get human-readable schema name
                                                let schema_name = schemas_map
                                                    .get()
                                                    .get(&config_data_source)
                                                    .cloned()
                                                    .unwrap_or_else(|| config_data_source.clone());
                                                
                                                let cfg_for_click = cfg.clone();
                                                let config_id_for_details = config_id.clone();
                                                let config_name_for_details = config_name.clone();
                                                
                                                view! {
                                                    <TableRow>
                                                        <TableCell>
                                                            <TableCellLayout>
                                                                <a
                                                                    href="#"
                                                                    style="
                                                                        color: var(--thaw-color-brand-foreground-1);
                                                                        text-decoration: none;
                                                                        font-weight: 600;
                                                                        cursor: pointer;
                                                                    "
                                                                    style:hover="text-decoration: underline;"
                                                                    on:click=move |e| {
                                                                        e.prevent_default();
                                                                        open_details(config_id_for_details.clone(), config_name_for_details.clone());
                                                                    }
                                                                >
                                                                    {config_name}
                                                                </a>
                                                            </TableCellLayout>
                                                        </TableCell>
                                                        <TableCell>
                                                            <TableCellLayout>
                                                                {config_description}
                                                            </TableCellLayout>
                                                        </TableCell>
                                                        <TableCell>
                                                            <TableCellLayout>
                                                                {schema_name}
                                                            </TableCellLayout>
                                                        </TableCell>
                                                        <TableCell>
                                                            <TableCellLayout>
                                                                {format_datetime(&config_updated_at)}
                                                            </TableCellLayout>
                                                        </TableCell>
                                                        <TableCell>
                                                            <TableCellLayout>
                                                                <Button
                                                                    size=thaw::ButtonSize::Small
                                                                    appearance=ButtonAppearance::Primary
                                                                    on_click=move |_| {
                                                                        open_report(cfg_for_click.clone());
                                                                    }
                                                                >
                                                                    "Сформировать отчет"
                                                                </Button>
                                                            </TableCellLayout>
                                                        </TableCell>
                                                    </TableRow>
                                                }
                                            })
                                            .collect_view()
                                    }}
                                </TableBody>
                            </Table>
                        </Show>
                    </Show>
                </Show>
            </div>
        </PageFrame>
    }
}
