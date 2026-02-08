//! Settings tab - saved dashboard configurations

use super::super::view_model::SchemaDetailsVm;
use leptos::prelude::*;
use thaw::*;

/// Format ISO datetime string to readable format
fn format_datetime(iso_str: &str) -> String {
    // Parse ISO 8601 format: "2024-01-15T10:30:00Z"
    // Return format: "15.01.2024 10:30"

    if iso_str.len() < 19 {
        return iso_str.to_string();
    }

    // Extract date and time parts
    let date_part = &iso_str[0..10]; // "2024-01-15"
    let time_part = &iso_str[11..16]; // "10:30"

    // Split date by '-'
    let date_parts: Vec<&str> = date_part.split('-').collect();
    if date_parts.len() != 3 {
        return iso_str.to_string();
    }

    // Reformat to DD.MM.YYYY HH:MM
    format!(
        "{}.{}.{} {}",
        date_parts[2], date_parts[1], date_parts[0], time_part
    )
}

/// Settings tab component
#[component]
pub fn SettingsTab(vm: SchemaDetailsVm) -> impl IntoView {
    let saved_configs = vm.saved_configs;
    let configs_loading = vm.configs_loading;

    // Store VM for use in closures
    let vm_stored = StoredValue::new(vm);

    view! {
        <Flex vertical=true gap=FlexGap::Medium>
            <div style="font-size: var(--font-size-lg); font-weight: 600; color: var(--color-text-primary);">
                "Сохранённые конфигурации дашборда"
            </div>

            <Show
                when=move || !configs_loading.get()
                fallback=|| view! {
                    <Flex gap=FlexGap::Small style="align-items: center; padding: var(--spacing-lg);">
                        <Spinner />
                        <span>"Загрузка конфигураций..."</span>
                    </Flex>
                }
            >
                {move || {
                    let configs = saved_configs.get();

                    if configs.is_empty() {
                        view! {
                            <div style="padding: var(--spacing-xl); text-align: center; color: var(--color-text-secondary); background: var(--color-neutral-50); border-radius: var(--radius-md); border: 1px dashed var(--color-neutral-200);">
                                <p>"Нет сохранённых конфигураций для этой схемы"</p>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <Table>
                                <TableHeader>
                                    <TableRow>
                                        <TableHeaderCell>"Название"</TableHeaderCell>
                                        <TableHeaderCell>"Описание"</TableHeaderCell>
                                        <TableHeaderCell>"Создана"</TableHeaderCell>
                                        <TableHeaderCell>"Обновлена"</TableHeaderCell>
                                        <TableHeaderCell>"Действия"</TableHeaderCell>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    {configs.into_iter().map(|config| {
                                        let id_for_delete = config.id.clone();

                                        view! {
                                            <TableRow>
                                                <TableCell>
                                                    <span style="font-weight: 500;">
                                                        {config.name}
                                                    </span>
                                                </TableCell>
                                                <TableCell>
                                                    <span style="color: var(--color-text-secondary);">
                                                        {config.description.unwrap_or_else(|| "-".to_string())}
                                                    </span>
                                                </TableCell>
                                                <TableCell>
                                                    <span style="font-size: var(--font-size-sm); color: var(--color-text-secondary);">
                                                        {format_datetime(&config.created_at)}
                                                    </span>
                                                </TableCell>
                                                <TableCell>
                                                    <span style="font-size: var(--font-size-sm); color: var(--color-text-secondary);">
                                                        {format_datetime(&config.updated_at)}
                                                    </span>
                                                </TableCell>
                                                <TableCell>
                                                    <Button
                                                        appearance=ButtonAppearance::Subtle
                                                        size=ButtonSize::Small
                                                        on_click=move |_| {
                                                            vm_stored.get_value().delete_config(id_for_delete.clone());
                                                        }
                                                    >
                                                        "Удалить"
                                                    </Button>
                                                </TableCell>
                                            </TableRow>
                                        }
                                    }).collect_view()}
                                </TableBody>
                            </Table>
                        }.into_any()
                    }
                }}
            </Show>
        </Flex>
    }
}
