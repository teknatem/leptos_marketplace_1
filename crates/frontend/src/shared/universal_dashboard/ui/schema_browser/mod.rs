//! Schema browser - view and test pivot schemas

use contracts::shared::universal_dashboard::{SchemaInfo, SchemaSource, SchemaValidationResult};
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;
use thaw::*;

use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::page_header::PageHeader;
use crate::shared::universal_dashboard::api;

// ============================================================================
// SchemaBrowser - Main Component
// ============================================================================

/// Main schema browser component
#[component]
pub fn SchemaBrowser() -> impl IntoView {
    // State
    let (schemas, set_schemas) = signal(Vec::<SchemaInfo>::new());
    let (validating, set_validating) = signal(None::<String>);
    let (validating_all, set_validating_all) = signal(false);
    let (test_results, set_test_results) = signal(HashMap::<String, SchemaValidationResult>::new());
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(None::<String>);

    // Load schemas on mount
    Effect::new(move |_| {
        spawn_local(async move {
            match api::list_schemas().await {
                Ok(response) => {
                    set_schemas.set(response.schemas);
                    set_loading.set(false);
                }
                Err(e) => {
                    log!("Failed to load schemas: {}", e);
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    });

    // Validate single schema
    let on_validate = Callback::new(move |schema_id: String| {
        set_validating.set(Some(schema_id.clone()));

        spawn_local(async move {
            match api::validate_schema(&schema_id).await {
                Ok(result) => {
                    let id = result.schema_id.clone();
                    set_test_results.update(|results| {
                        results.insert(id, result);
                    });
                }
                Err(e) => {
                    log!("Failed to validate schema: {}", e);
                    set_error.set(Some(e));
                }
            }
            set_validating.set(None);
        });
    });

    // Validate all schemas
    let on_validate_all = Callback::new(move |_: ()| {
        set_validating_all.set(true);

        spawn_local(async move {
            match api::validate_all_schemas().await {
                Ok(response) => {
                    set_test_results.update(|results| {
                        for result in response.results {
                            let id = result.schema_id.clone();
                            results.insert(id, result);
                        }
                    });
                }
                Err(e) => {
                    log!("Failed to validate all schemas: {}", e);
                    set_error.set(Some(e));
                }
            }
            set_validating_all.set(false);
        });
    });

    view! {
        <div class="page">
            <PageHeader title="Схемы данных" subtitle="Просмотр и тестирование схем для сводных таблиц">
                <Button
                    appearance=ButtonAppearance::Primary
                    on_click=move |_| on_validate_all.run(())
                    disabled=move || validating_all.get()
                    loading=move || validating_all.get()
                >
                    {move || if validating_all.get() { "Тестирование..." } else { "Тестировать все" }}
                </Button>
            </PageHeader>

            <div class="page__content">
                <Show
                    when=move || !loading.get()
                    fallback=|| view! {
                        <Flex gap=FlexGap::Small style="align-items: center; padding: var(--spacing-4xl); justify-content: center;">
                            <Spinner />
                            <span>"Загрузка схем..."</span>
                        </Flex>
                    }
                >
                    {move || error.get().map(|e| view! {
                        <div style="padding: var(--spacing-md); background: var(--color-error-50); border: 1px solid var(--color-error-100); border-radius: var(--radius-sm); color: var(--color-error); margin-bottom: var(--spacing-md);">
                            <strong>"Ошибка: "</strong>{e}
                        </div>
                    })}

                    <SchemaList
                        schemas=schemas
                        on_validate=on_validate
                        test_results=test_results
                        validating=validating
                    />
                </Show>
            </div>
        </div>
    }
}

// ============================================================================
// SchemaList - Table of schemas with validation buttons
// ============================================================================

#[component]
fn SchemaList(
    #[prop(into)] schemas: Signal<Vec<SchemaInfo>>,
    on_validate: Callback<String>,
    #[prop(into)] test_results: Signal<HashMap<String, SchemaValidationResult>>,
    #[prop(into)] validating: Signal<Option<String>>,
) -> impl IntoView {
    let ctx =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    view! {
        <Table>
            <TableHeader>
                <TableRow>
                    <TableHeaderCell>"ID"</TableHeaderCell>
                    <TableHeaderCell>"Название"</TableHeaderCell>
                    <TableHeaderCell>"Источник"</TableHeaderCell>
                    <TableHeaderCell>"Таблица"</TableHeaderCell>
                    <TableHeaderCell>"Результат теста"</TableHeaderCell>
                    <TableHeaderCell>"Действия"</TableHeaderCell>
                </TableRow>
            </TableHeader>
            <TableBody>
                {move || {
                    schemas.get().into_iter().map(|schema| {
                        let id_for_btn = schema.id.clone();
                        let id_for_check = schema.id.clone();
                        let id_for_text = schema.id.clone();
                        let id_for_result = schema.id.clone();
                        let id_for_click = schema.id.clone();
                        let name_for_click = schema.name.clone();

                        view! {
                            <TableRow>
                                <TableCell>
                                    <span style="font-family: var(--font-mono); font-size: var(--font-size-xs); color: var(--color-text-secondary);">
                                        {schema.id.clone()}
                                    </span>
                                </TableCell>
                                <TableCell>
                                    <a
                                        href="#"
                                        style="color: var(--color-primary); text-decoration: none; font-weight: 500; cursor: pointer;"
                                        on:click=move |ev| {
                                            ev.prevent_default();
                                            let tab_key = format!("schema_details_{}", id_for_click);
                                            let tab_title = format!("Schema: {}", name_for_click);
                                            ctx.open_tab(&tab_key, &tab_title);
                                        }
                                    >
                                        {schema.name.clone()}
                                    </a>
                                </TableCell>
                                <TableCell>
                                    <Badge
                                        appearance=match schema.source {
                                            SchemaSource::Custom => BadgeAppearance::Filled,
                                            SchemaSource::Auto => BadgeAppearance::Tint,
                                        }
                                        color=match schema.source {
                                            SchemaSource::Custom => BadgeColor::Brand,
                                            SchemaSource::Auto => BadgeColor::Informative,
                                        }
                                    >
                                        {match schema.source {
                                            SchemaSource::Custom => "Кастомная",
                                            SchemaSource::Auto => "Авто",
                                        }}
                                    </Badge>
                                </TableCell>
                                <TableCell>
                                    <span style="font-family: var(--font-mono); font-size: var(--font-size-sm);">
                                        {schema.table_name.clone()}
                                    </span>
                                </TableCell>
                                <TableCell>
                                    {move || {
                                        test_results.get().get(&id_for_result).cloned().map(|result| {
                                            let is_valid = result.is_valid;
                                            let time_ms = result.execution_time_us as f64 / 1000.0;
                                            let row_count = result.row_count;

                                            view! {
                                                <Flex gap=FlexGap::Small align=FlexAlign::Center>
                                                    <span style=format!(
                                                        "font-size: 18px; font-weight: bold; color: {};",
                                                        if is_valid { "var(--color-success)" } else { "var(--color-error)" }
                                                    )>
                                                        {if is_valid { "✓" } else { "✗" }}
                                                    </span>
                                                    <span style="color: var(--color-text-secondary); font-size: var(--font-size-sm);">
                                                        {format!("{:.2}мс", time_ms)}
                                                    </span>
                                                    {row_count.map(|count| view! {
                                                        <span style="color: var(--color-text-secondary); font-size: var(--font-size-sm);">
                                                            {format!("{} строк", count)}
                                                        </span>
                                                    })}
                                                </Flex>
                                            }
                                        })
                                    }}
                                </TableCell>
                                <TableCell>
                                    <Button
                                        appearance=ButtonAppearance::Subtle
                                        size=ButtonSize::Small
                                        on_click=move |_| on_validate.run(id_for_btn.clone())
                                        disabled=move || validating.get().as_ref() == Some(&id_for_check)
                                        loading=move || validating.get().as_ref() == Some(&id_for_text)
                                    >
                                        "Тест"
                                    </Button>
                                </TableCell>
                            </TableRow>
                        }
                    }).collect_view()
                }}
            </TableBody>
        </Table>
    }
}

// ============================================================================
// ValidationCard - Single validation result card (used in schema details)
// ============================================================================

/// Reusable validation card component for displaying test results
#[component]
pub fn ValidationCard(result: SchemaValidationResult) -> impl IntoView {
    let has_warnings = !result.warnings.is_empty();

    view! {
        <div style="margin-bottom: var(--spacing-md); padding: var(--spacing-lg); border: 1px solid var(--color-neutral-200); border-radius: var(--radius-md); background: var(--color-bg-container);">
            <Flex vertical=true gap=FlexGap::Medium>
                // Header
                <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center>
                    <span style="font-weight: 600; font-size: var(--font-size-lg);">
                        {result.schema_id.clone()}
                    </span>
                    <Badge
                        appearance=BadgeAppearance::Filled
                        color=if result.is_valid { BadgeColor::Success } else { BadgeColor::Danger }
                    >
                        {if result.is_valid { "OK" } else { "ОШИБКА" }}
                    </Badge>
                </Flex>

                // Meta information
                <Flex gap=FlexGap::Large>
                    <span style="color: var(--color-text-secondary); font-size: var(--font-size-sm);">
                        "Время: " <strong>{format!("{:.2}мс", result.execution_time_us as f64 / 1000.0)}</strong>
                    </span>
                    {result.row_count.map(|count| view! {
                        <span style="color: var(--color-text-secondary); font-size: var(--font-size-sm);">
                            "Строк: " <strong>{count}</strong>
                        </span>
                    })}
                </Flex>

                // Errors
                {(!result.errors.is_empty()).then(|| view! {
                    <div>
                        <div style="font-weight: 600; margin-bottom: var(--spacing-sm); color: var(--color-error);">
                            "Ошибки:"
                        </div>
                        <ul style="margin: 0; padding-left: var(--spacing-lg); color: var(--color-error);">
                            {result.errors.iter().map(|e| view! {
                                <li style="margin-bottom: var(--spacing-xs);">{e.clone()}</li>
                            }).collect_view()}
                        </ul>
                    </div>
                })}

                // Warnings
                {has_warnings.then(|| view! {
                    <div>
                        <div style="font-weight: 600; margin-bottom: var(--spacing-sm); color: var(--color-warning);">
                            "Предупреждения:"
                        </div>
                        <ul style="margin: 0; padding-left: var(--spacing-lg); color: var(--color-warning);">
                            {result.warnings.iter().map(|w| view! {
                                <li style="margin-bottom: var(--spacing-xs);">{w.clone()}</li>
                            }).collect_view()}
                        </ul>
                    </div>
                })}
            </Flex>
        </div>
    }
}
