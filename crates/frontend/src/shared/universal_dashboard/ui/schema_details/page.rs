//! Main page component for Schema Details

use super::tabs::{FieldsTab, SettingsTab, SqlTab, TestTab};
use super::view_model::SchemaDetailsVm;
use leptos::prelude::*;
use thaw::*;

/// Main component for Schema Details
#[component]
pub fn SchemaDetails(schema_id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    // Create ViewModel
    let vm = SchemaDetailsVm::new();

    // Load schema data
    vm.load(schema_id);

    // Lazy loading for tabs
    let vm_effect = vm.clone();
    Effect::new(move |_| match vm_effect.active_tab.get() {
        "settings" if !vm_effect.configs_loaded.get() => vm_effect.load_configs(),
        "sql" if !vm_effect.sql_loaded.get() => vm_effect.generate_sql(),
        _ => {}
    });

    // Clone for closures
    let vm_header = vm.clone();
    let vm_tabs = vm.clone();
    let vm_content = vm.clone();
    let vm_loading = vm.clone();
    let vm_error = vm.clone();
    let vm_schema = vm.clone();

    view! {
        <div class="detail-form">
            // Header
            <Header vm=vm_header on_close=on_close />

            <div class="detail-form-content">
                {move || {
                    if vm_loading.loading.get() {
                        view! {
                            <Flex gap=FlexGap::Small style="align-items: center; padding: var(--spacing-4xl); justify-content: center;">
                                <Spinner />
                                <span>"Загрузка схемы..."</span>
                            </Flex>
                        }.into_any()
                    } else if let Some(err) = vm_error.error.get() {
                        view! {
                            <div style="padding: var(--spacing-sm); background: var(--color-error-50); border: 1px solid var(--color-error-100); border-radius: var(--radius-sm); color: var(--color-error); margin: var(--spacing-lg);">
                                <strong>"Ошибка: "</strong>{err}
                            </div>
                        }.into_any()
                    } else if vm_schema.schema.get().is_some() {
                        view! {
                            <div>
                                // Tab bar
                                <TabBar vm=vm_tabs.clone() />

                                // Tab content
                                <div style="padding-top: var(--spacing-sm);">
                                    <TabContent vm=vm_content.clone() />
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div>"Схема не найдена"</div> }.into_any()
                    }
                }}
            </div>
        </div>
    }
}

/// Header component with title and close button
#[component]
fn Header(vm: SchemaDetailsVm, on_close: Callback<()>) -> impl IntoView {
    let schema_name_fn = vm.schema_name();

    view! {
        <div class="page__header">
            <div class="page__header-left">
                <h1 class="page__title">{move || format!("Схема: {}", schema_name_fn())}</h1>
            </div>
            <div class="page__header-right">
                <Button
                    appearance=ButtonAppearance::Secondary
                    size=ButtonSize::Medium
                    on_click=move |_| on_close.run(())
                >
                    "Закрыть"
                </Button>
            </div>
        </div>
    }
}

/// Tab bar component
#[component]
fn TabBar(vm: SchemaDetailsVm) -> impl IntoView {
    let active_tab = vm.active_tab;

    view! {
        <div style="border-bottom: 1px solid var(--thaw-color-neutral-stroke-1); margin-bottom: 8px; padding-bottom: 4px;">
            <Space>
                <Button
                    appearance=move || if active_tab.get() == "fields" { ButtonAppearance::Primary } else { ButtonAppearance::Subtle }
                    on_click=move |_| active_tab.set("fields")
                >
                    "Поля"
                </Button>
                <Button
                    appearance=move || if active_tab.get() == "settings" { ButtonAppearance::Primary } else { ButtonAppearance::Subtle }
                    on_click=move |_| active_tab.set("settings")
                >
                    "Настройки"
                </Button>
                <Button
                    appearance=move || if active_tab.get() == "sql" { ButtonAppearance::Primary } else { ButtonAppearance::Subtle }
                    on_click=move |_| active_tab.set("sql")
                >
                    "SQL"
                </Button>
                <Button
                    appearance=move || if active_tab.get() == "test" { ButtonAppearance::Primary } else { ButtonAppearance::Subtle }
                    on_click=move |_| active_tab.set("test")
                >
                    "Тест"
                </Button>
            </Space>
        </div>
    }
}

/// Tab content component
#[component]
fn TabContent(vm: SchemaDetailsVm) -> impl IntoView {
    let active_tab = vm.active_tab;
    let schema = vm.schema;

    // Store VMs for reuse in closures
    let vm_stored = StoredValue::new(vm);

    view! {
        <Show
            when=move || schema.get().is_some()
            fallback=|| view! {
                <div>"Нет данных"</div>
            }
        >
            {move || schema.get().map(|s| {
                let vm_val = vm_stored.get_value();
                view! {
                    <div style=move || if active_tab.get() == "fields" { "" } else { "display: none;" }>
                        <FieldsTab schema=s.clone() />
                    </div>
                    <div style=move || if active_tab.get() == "settings" { "" } else { "display: none;" }>
                        <SettingsTab vm=vm_val.clone() />
                    </div>
                    <div style=move || if active_tab.get() == "sql" { "" } else { "display: none;" }>
                        <SqlTab vm=vm_val.clone() />
                    </div>
                    <div style=move || if active_tab.get() == "test" { "" } else { "display: none;" }>
                        <TestTab vm=vm_val.clone() />
                    </div>
                }
            })}
        </Show>
    }
}
