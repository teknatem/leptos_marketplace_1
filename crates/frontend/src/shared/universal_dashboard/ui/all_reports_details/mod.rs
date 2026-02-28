//! All Reports Details - просмотр и редактирование настройки отчета

use crate::shared::page_frame::PageFrame;
use crate::shared::universal_dashboard::api;
use contracts::shared::universal_dashboard::{SavedDashboardConfig, UpdateDashboardConfigRequest};
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::{Button, ButtonAppearance, Card, Input, Space, Textarea};

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

#[component]
pub fn AllReportsDetails(
    /// Configuration ID
    config_id: String,
    /// Callback when close is requested
    on_close: Callback<()>,
) -> impl IntoView {
    // Data state
    let (config_data, set_config_data) = signal(None::<SavedDashboardConfig>);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(None::<String>);
    let (saving, set_saving) = signal(false);

    // Form state
    let name = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let (json_pretty, set_json_pretty) = signal(String::new());

    // Load config on mount
    let config_id_for_load = config_id.clone();
    Effect::new(move |_| {
        let id = config_id_for_load.clone();
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            match api::get_config(&id).await {
                Ok(config) => {
                    // Set form fields
                    name.set(config.name.clone());
                    description.set(config.description.clone().unwrap_or_default());

                    // Format JSON for display
                    match serde_json::to_string_pretty(&config.config) {
                        Ok(json_str) => set_json_pretty.set(json_str),
                        Err(e) => {
                            log!("Failed to serialize config to JSON: {}", e);
                            set_json_pretty
                                .set(format!("{{\"error\": \"Failed to serialize: {}\"}}", e));
                        }
                    }

                    set_config_data.set(Some(config));
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(format!("Ошибка загрузки настройки: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    });

    // Save changes
    let handle_save = move |_| {
        let current_config = config_data.get();
        if let Some(cfg) = current_config {
            set_saving.set(true);
            set_error.set(None);

            let request = UpdateDashboardConfigRequest {
                id: cfg.id.clone(),
                name: name.get(),
                description: if description.get().is_empty() {
                    None
                } else {
                    Some(description.get())
                },
                config: cfg.config.clone(), // Keep original config unchanged
            };

            let cfg_id = cfg.id.clone();
            spawn_local(async move {
                match api::update_config(&cfg_id, request).await {
                    Ok(_) => {
                        log!("Config updated successfully");
                        set_saving.set(false);
                        on_close.run(());
                    }
                    Err(e) => {
                        set_error.set(Some(format!("Ошибка сохранения: {}", e)));
                        set_saving.set(false);
                    }
                }
            });
        }
    };

    let handle_cancel = move |_| {
        on_close.run(());
    };

    view! {
        <PageFrame page_id="all_reports--detail" category="detail">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Настройка отчета"</h1>
                </div>
                <div class="page__header-right">
                    <Space>
                        <Button
                            appearance=ButtonAppearance::Primary
                            on_click=handle_save
                            disabled=move || loading.get() || saving.get()
                        >
                            {move || if saving.get() { "Сохранение..." } else { "Сохранить" }}
                        </Button>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            on_click=handle_cancel
                            disabled=move || saving.get()
                        >
                            "Отменить"
                        </Button>
                    </Space>
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
                        {move || config_data.get().map(|cfg| {
                            view! {
                                <div style="max-width: 1200px; padding: 24px;">
                                    // Basic Information Section
                                    <div style="margin-bottom: 32px;">
                                        <h2 style="margin-bottom: 16px; font-size: 18px; font-weight: 600;">
                                            "Основная информация"
                                        </h2>

                                        <div style="display: grid; gap: 16px;">
                                        <Card attr:style="width: 50%; margin: 0px;">
                                            // Name (editable)
                                            <div style="width: 100%;">
                                                <label style="display: block; margin-bottom: 6px; font-weight: 500; ">
                                                    "Наименование"
                                                </label>
                                                <Input
                                                    value=name
                                                    placeholder="Введите наименование"
                                                    attr:style="width: 80%;"
                                                />
                                            </div>

                                            // Description (editable)
                                            <div style="width: 100%;">
                                                <label style="display: block; margin-bottom: 6px; font-weight: 500; ">
                                                    "Описание"
                                                </label>
                                                <div style="height: 80px;">
                                                    <Textarea
                                                        value=description
                                                        placeholder="Введите описание"
                                                        attr:style="width: 80%;"
                                                    />
                                                </div>
                                            </div>
                                        </Card>

                                            // ID (read-only)
                                            <div>
                                                <label style="display: block; margin-bottom: 6px; font-weight: 500; color: var(--thaw-color-neutral-foreground-2);">
                                                    "ID"
                                                </label>
                                                <div style="padding: 8px 12px; background: var(--thaw-color-neutral-background-2); border-radius: 4px; font-family: monospace; font-size: 14px;">
                                                    {cfg.id.clone()}
                                                </div>
                                            </div>

                                            // Data Source (read-only)
                                            <div>
                                                <label style="display: block; margin-bottom: 6px; font-weight: 500; color: var(--thaw-color-neutral-foreground-2);">
                                                    "Источник данных"
                                                </label>
                                                <div style="padding: 8px 12px; background: var(--thaw-color-neutral-background-2); border-radius: 4px; font-family: monospace; font-size: 14px;">
                                                    {cfg.data_source.clone()}
                                                </div>
                                            </div>

                                            // Timestamps (read-only)
                                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 16px;">
                                                <div>
                                                    <label style="display: block; margin-bottom: 6px; font-weight: 500; color: var(--thaw-color-neutral-foreground-2);">
                                                        "Создано"
                                                    </label>
                                                    <div style="padding: 8px 12px; background: var(--thaw-color-neutral-background-2); border-radius: 4px; font-size: 14px;">
                                                        {format_datetime(&cfg.created_at)}
                                                    </div>
                                                </div>
                                                <div>
                                                    <label style="display: block; margin-bottom: 6px; font-weight: 500; color: var(--thaw-color-neutral-foreground-2);">
                                                        "Обновлено"
                                                    </label>
                                                    <div style="padding: 8px 12px; background: var(--thaw-color-neutral-background-2); border-radius: 4px; font-size: 14px;">
                                                        {format_datetime(&cfg.updated_at)}
                                                    </div>
                                                </div>
                                            </div>
                                        </div>
                                    </div>

                                    // JSON Configuration Section
                                    <div>
                                        <h2 style="margin-bottom: 16px; font-size: 18px; font-weight: 600;">
                                            "Конфигурация (JSON)"
                                        </h2>
                                        <div style="position: relative;">
                                            <pre style="
                                                padding: 16px;
                                                background: var(--thaw-color-neutral-background-2);
                                                border-radius: 4px;
                                                overflow-x: auto;
                                                max-height: 500px;
                                                overflow-y: auto;
                                                font-family: 'Courier New', Courier, monospace;
                                                font-size: 13px;
                                                line-height: 1.5;
                                                margin: 0;
                                            ">
                                                <code>{json_pretty.get()}</code>
                                            </pre>
                                        </div>
                                    </div>
                                </div>
                            }
                        })}
            </Show>
            </Show>
            </div>
        </PageFrame>
    }
}
