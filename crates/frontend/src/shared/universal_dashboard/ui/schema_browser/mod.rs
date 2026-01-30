//! Schema browser - view and test pivot schemas

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::logging::log;
use contracts::shared::universal_dashboard::{
    SchemaInfo, SchemaSource, SchemaValidationResult, ValidateAllSchemasResponse,
};

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
    let (single_result, set_single_result) = signal(None::<SchemaValidationResult>);
    let (all_results, set_all_results) = signal(None::<ValidateAllSchemasResponse>);
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
        set_single_result.set(None);
        set_all_results.set(None);

        spawn_local(async move {
            match api::validate_schema(&schema_id).await {
                Ok(result) => {
                    set_single_result.set(Some(result));
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
        set_single_result.set(None);
        set_all_results.set(None);

        spawn_local(async move {
            match api::validate_all_schemas().await {
                Ok(result) => {
                    set_all_results.set(Some(result));
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
        <div class="schema-browser">
            <div class="schema-browser-header">
                <h1>"Схемы данных"</h1>
                <p class="text-muted">
                    "Просмотр и тестирование схем для сводных таблиц"
                </p>
            </div>

            <Show
                when=move || !loading.get()
                fallback=|| view! {
                    <div class="loading-state">
                        <div class="spinner"></div>
                        <p>"Загрузка схем..."</p>
                    </div>
                }
            >
                {move || error.get().map(|e| view! {
                    <div class="error-banner">
                        <p>{e}</p>
                    </div>
                })}

                <div class="schema-browser-content">
                    <div class="schema-browser-main">
                        <SchemaList
                            schemas=schemas
                            on_validate=on_validate
                            on_validate_all=on_validate_all
                            validating=validating
                            validating_all=validating_all
                        />
                    </div>
                    <div class="schema-browser-side">
                        <ValidationPanel
                            single_result=single_result
                            all_results=all_results
                        />
                    </div>
                </div>
            </Show>
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
    on_validate_all: Callback<()>,
    #[prop(into)] validating: Signal<Option<String>>,
    #[prop(into)] validating_all: Signal<bool>,
) -> impl IntoView {
    view! {
        <div class="schema-list">
            <div class="schema-list-header">
                <h2>"Схемы данных"</h2>
                <button
                    class="btn btn-primary"
                    on:click=move |_| on_validate_all.run(())
                    disabled=move || validating_all.get()
                >
                    {move || if validating_all.get() { "Тестирование..." } else { "Тестировать все" }}
                </button>
            </div>

            <table class="schema-table">
                <thead>
                    <tr>
                        <th>"ID"</th>
                        <th>"Название"</th>
                        <th>"Источник"</th>
                        <th>"Таблица"</th>
                        <th>"Действия"</th>
                    </tr>
                </thead>
                <tbody>
                    {move || {
                        schemas.get().into_iter().map(|schema| {
                            let id_for_btn = schema.id.clone();
                            let id_for_check = schema.id.clone();
                            let id_for_text = schema.id.clone();

                            view! {
                                <tr>
                                    <td class="schema-id">{schema.id.clone()}</td>
                                    <td>{schema.name.clone()}</td>
                                    <td>
                                        <span class=match schema.source {
                                            SchemaSource::Custom => "badge badge-custom",
                                            SchemaSource::Auto => "badge badge-auto",
                                        }>
                                            {match schema.source {
                                                SchemaSource::Custom => "Кастомная",
                                                SchemaSource::Auto => "Авто",
                                            }}
                                        </span>
                                    </td>
                                    <td class="schema-table-name">{schema.table_name.clone()}</td>
                                    <td>
                                        <button
                                            class="btn btn-sm btn-outline"
                                            on:click=move |_| on_validate.run(id_for_btn.clone())
                                            disabled=move || validating.get().as_ref() == Some(&id_for_check)
                                        >
                                            {move || {
                                                if validating.get().as_ref() == Some(&id_for_text) {
                                                    "..."
                                                } else {
                                                    "Тест"
                                                }
                                            }}
                                        </button>
                                    </td>
                                </tr>
                            }
                        }).collect_view()
                    }}
                </tbody>
            </table>
        </div>
    }
}

// ============================================================================
// ValidationPanel - Display validation results
// ============================================================================

#[component]
fn ValidationPanel(
    #[prop(into)] single_result: Signal<Option<SchemaValidationResult>>,
    #[prop(into)] all_results: Signal<Option<ValidateAllSchemasResponse>>,
) -> impl IntoView {
    view! {
        <div class="validation-panel">
            {move || {
                single_result.get().map(|result| {
                    view! {
                        <div class="validation-result single-result">
                            <h3>"Результат: " <span class="schema-name">{result.schema_name.clone()}</span></h3>
                            <ValidationCard result=result />
                        </div>
                    }
                })
            }}

            {move || {
                all_results.get().map(|response| {
                    view! {
                        <div class="validation-result all-results">
                            <h3>"Результаты тестирования всех схем"</h3>
                            <div class="validation-summary">
                                <span class="summary-item">"Всего: " <strong>{response.total_schemas}</strong></span>
                                <span class="summary-item valid">"Валидных: " <strong>{response.valid_count}</strong></span>
                                <span class="summary-item invalid">"С ошибками: " <strong>{response.invalid_count}</strong></span>
                                <span class="summary-item time">"Время: " <strong>{response.total_time_ms}"мс"</strong></span>
                            </div>
                            <div class="validation-results-list">
                                {response.results.into_iter().map(|r| {
                                    view! { <ValidationCard result=r /> }
                                }).collect_view()}
                            </div>
                        </div>
                    }
                })
            }}

            {move || {
                (single_result.get().is_none() && all_results.get().is_none()).then(|| view! {
                    <div class="validation-empty">
                        <p>"Нажмите \"Тест\" для проверки схемы"</p>
                    </div>
                })
            }}
        </div>
    }
}

// ============================================================================
// ValidationCard - Single validation result card
// ============================================================================

#[component]
fn ValidationCard(result: SchemaValidationResult) -> impl IntoView {
    let status_class = if result.is_valid { "status-valid" } else { "status-invalid" };
    let has_warnings = !result.warnings.is_empty();

    view! {
        <div class=format!("validation-card {}", status_class)>
            <div class="card-header">
                <span class="card-title">{result.schema_id.clone()}</span>
                <span class="card-status">{if result.is_valid { "OK" } else { "ОШИБКА" }}</span>
            </div>
            <div class="card-body">
                <div class="card-meta">
                    <span class="meta-item">"Время: " {result.execution_time_ms}"мс"</span>
                    {result.row_count.map(|count| view! {
                        <span class="meta-item">"Строк: " {count}</span>
                    })}
                </div>

                {(!result.errors.is_empty()).then(|| view! {
                    <div class="card-errors">
                        <strong>"Ошибки:"</strong>
                        <ul>
                            {result.errors.iter().map(|e| view! { <li class="error-item">{e.clone()}</li> }).collect_view()}
                        </ul>
                    </div>
                })}

                {has_warnings.then(|| view! {
                    <div class="card-warnings">
                        <strong>"Предупреждения:"</strong>
                        <ul>
                            {result.warnings.iter().map(|w| view! { <li class="warning-item">{w.clone()}</li> }).collect_view()}
                        </ul>
                    </div>
                })}
            </div>
        </div>
    }
}
