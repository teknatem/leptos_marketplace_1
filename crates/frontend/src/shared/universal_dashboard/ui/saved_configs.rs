use leptos::prelude::*;
use leptos::ev::Event;
use thaw::*;
use wasm_bindgen::JsCast;
use contracts::shared::universal_dashboard::{SavedDashboardConfigSummary, DashboardConfig};
use crate::shared::modal_frame::ModalFrame;

/// Format ISO datetime string to human-readable format (DD.MM.YYYY HH:MM)
fn format_datetime(iso: &str) -> String {
    // Parse ISO format: 2026-01-26T21:57:17.656420+00:00
    if iso.len() >= 16 {
        let date_part = &iso[0..10]; // 2026-01-26
        let time_part = &iso[11..16]; // 21:57
        
        // Parse date components
        let parts: Vec<&str> = date_part.split('-').collect();
        if parts.len() == 3 {
            return format!("{}.{}.{} {}", parts[2], parts[1], parts[0], time_part);
        }
    }
    iso.to_string()
}

fn event_target_value(ev: &Event) -> String {
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|input: web_sys::HtmlInputElement| input.value())
        .unwrap_or_default()
}

fn event_target_textarea_value(ev: &Event) -> String {
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlTextAreaElement>().ok())
        .map(|textarea: web_sys::HtmlTextAreaElement| textarea.value())
        .unwrap_or_default()
}

#[component]
pub fn SavedConfigsList(
    /// List of saved configurations
    #[prop(into)]
    configs: Signal<Vec<SavedDashboardConfigSummary>>,
    /// Callback when a config is selected to load
    on_load: Callback<String>,
    /// Callback when a config is deleted
    on_delete: Callback<String>,
) -> impl IntoView {
    view! {
        <div class="saved-configs-list">
            <Show
                when=move || !configs.get().is_empty()
                fallback=|| view! {
                    <div class="empty-state" style="padding: 48px; text-align: center;">
                        <p style="color: var(--thaw-color-neutral-foreground-2);">"Нет сохранённых настроек"</p>
                    </div>
                }
            >
                <Table>
                    <TableHeader>
                        <TableRow>
                            <TableHeaderCell resizable=true min_width=200.0>"Название"</TableHeaderCell>
                            <TableHeaderCell resizable=true min_width=300.0>"Описание"</TableHeaderCell>
                            <TableHeaderCell min_width=150.0>"Обновлено"</TableHeaderCell>
                            <TableHeaderCell min_width=150.0>"Действия"</TableHeaderCell>
                        </TableRow>
                    </TableHeader>
                    <TableBody>
                        {move || {
                            configs
                                .get()
                                .iter()
                                .map(|cfg| {
                                    let config_id = cfg.id.clone();
                                    let config_id_for_delete = cfg.id.clone();
                                    let config_name = cfg.name.clone();
                                    let config_updated_at = cfg.updated_at.clone();
                                    let config_description = cfg.description.clone().unwrap_or_default();
                                    view! {
                                        <TableRow>
                                            <TableCell>
                                                <TableCellLayout>
                                                    <strong>{config_name}</strong>
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    {config_description}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    {format_datetime(&config_updated_at)}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout>
                                                    <Space>
                                                        <Button
                                                            size=ButtonSize::Small
                                                            appearance=ButtonAppearance::Primary
                                                            on_click=move |_| {
                                                                on_load.run(config_id.clone());
                                                            }
                                                        >
                                                            "Загрузить"
                                                        </Button>
                                                        <Button
                                                            size=ButtonSize::Small
                                                            appearance=ButtonAppearance::Subtle
                                                            on_click=move |_| {
                                                                on_delete.run(config_id_for_delete.clone());
                                                            }
                                                        >
                                                            "Удалить"
                                                        </Button>
                                                    </Space>
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
        </div>
    }
}

#[component]
pub fn SaveConfigDialog(
    /// Show/hide the dialog
    #[prop(into)]
    show: Signal<bool>,
    /// Current configuration to save
    _config: DashboardConfig,
    /// Callback when save is clicked
    on_save: Callback<(String, Option<String>)>,
    /// Callback when cancel is clicked
    on_cancel: Callback<()>,
) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());

    let handle_save = move |_| {
        let name_val = name.get();
        let desc_val = description.get();
        if !name_val.is_empty() {
            let desc_opt = if desc_val.is_empty() {
                None
            } else {
                Some(desc_val)
            };
            on_save.run((name_val, desc_opt));
            name.set(String::new());
            description.set(String::new());
        }
    };

    view! {
        <Show when=move || show.get() fallback=|| view! {}>
            <ModalFrame
                on_close=Callback::new(move |_| on_cancel.run(()))
                modal_style="max-width: 500px;".to_string()
            >
                <div style="padding: 24px;">
                    <h3 style="margin-top: 0; margin-bottom: 20px;">"Сохранить настройки"</h3>
                    <div style="margin-bottom: 16px;">
                        <label style="display: block; margin-bottom: 6px; font-weight: 500;">"Название"</label>
                        <input
                            type="text"
                            class="form-control"
                            prop:value=name
                            on:input=move |ev| {
                                name.set(event_target_value(&ev));
                            }
                        />
                    </div>
                    <div style="margin-bottom: 16px;">
                        <label style="display: block; margin-bottom: 6px; font-weight: 500;">"Описание (опционально)"</label>
                        <textarea
                            class="form-control"
                            prop:value=description
                            on:input=move |ev| {
                                description.set(event_target_textarea_value(&ev));
                            }
                        />
                    </div>
                    <div style="display: flex; justify-content: flex-end; margin-top: 24px;">
                        <Space>
                            <Button appearance=ButtonAppearance::Primary on_click=handle_save>
                                "Сохранить"
                            </Button>
                            <Button
                                appearance=ButtonAppearance::Secondary
                                on_click=move |_| {
                                    on_cancel.run(());
                                }
                            >
                                "Отмена"
                            </Button>
                        </Space>
                    </div>
                </div>
            </ModalFrame>
        </Show>
    }
}
