use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::icons::icon;
use contracts::domain::a019_llm_artifact::aggregate::{ArtifactType, LlmArtifact};
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use thaw::*;

async fn fetch_artifact(id: &str) -> Result<LlmArtifact, String> {
    let url = format!("{}/api/a019-llm-artifact/{}", api_base(), id);

    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch: {}", e))?;

    if response.status() != 200 {
        return Err(format!("Server error: {}", response.status()));
    }

    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    serde_json::from_str::<LlmArtifact>(&text)
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Извлечь session_id из query_params JSON поля артефакта DrilldownReport.
fn parse_session_id(query_params: &Option<String>) -> Option<String> {
    let params_str = query_params.as_deref()?;
    let v: serde_json::Value = serde_json::from_str(params_str).ok()?;
    v.get("session_id")?.as_str().map(|s| s.to_string())
}

#[component]
#[allow(non_snake_case)]
pub fn ArtifactCard(artifact_id: String) -> impl IntoView {
    let global_ctx = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    let artifact = RwSignal::new(None::<LlmArtifact>);
    let error = RwSignal::new(None::<String>);

    let artifact_id_clone = artifact_id.clone();
    Effect::new(move |_| {
        let artifact_id = artifact_id_clone.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_artifact(&artifact_id).await {
                Ok(a) => {
                    log!("✅ Artifact loaded for card: {}", a.base.description);
                    artifact.set(Some(a));
                    error.set(None);
                }
                Err(e) => {
                    log!("❌ Failed to load artifact for card: {}", e);
                    error.set(Some(e));
                }
            }
        });
    });

    // Кнопка "Открыть" для SqlQuery-артефакта
    let handle_open_sql = {
        let artifact_id = artifact_id.clone();
        move |_| {
            let key = format!("a019_llm_artifact_details_{}", artifact_id);
            global_ctx.open_tab(&key, "Артефакт");
        }
    };

    view! {
        <div style="margin-top: 12px; padding: 12px; border: 1px solid var(--colorNeutralStroke2); border-radius: 8px; background: var(--colorNeutralBackground2);">
            {move || {
                if let Some(err) = error.get() {
                    view! {
                        <div style="color: var(--color-error);">
                            {format!("Ошибка загрузки артефакта: {}", err)}
                        </div>
                    }.into_any()
                } else if let Some(a) = artifact.get() {
                    let description = a.base.description.clone();
                    let created_at = a.base.metadata.created_at.format("%d.%m.%Y %H:%M").to_string();

                    if a.artifact_type == ArtifactType::DrilldownReport {
                        // ── Drilldown Report карточка ──
                        let session_id = parse_session_id(&a.query_params);
                        let tab_key = session_id
                            .as_deref()
                            .map(|s| format!("drilldown__{}", s))
                            .unwrap_or_default();
                        let has_session = !tab_key.is_empty();

                        let comment_text = a.base.comment.clone().unwrap_or_default();
                        let has_comment = !comment_text.is_empty();

                        let desc_for_open = description.clone();
                        let handle_open_drilldown = move |_| {
                            if has_session {
                                global_ctx.open_tab(&tab_key, &desc_for_open);
                            }
                        };

                        view! {
                            <div>
                                <div style="display: flex; justify-content: space-between; align-items: center;">
                                    <div style="display: flex; align-items: center; gap: 8px;">
                                        {icon("bar-chart")}
                                        <strong>{description.clone()}</strong>
                                    </div>
                                    <Button
                                        size=ButtonSize::Small
                                        appearance=ButtonAppearance::Primary
                                        on_click=handle_open_drilldown
                                        disabled=move || !has_session
                                    >
                                        {icon("external-link")}
                                        " Открыть отчёт"
                                    </Button>
                                </div>

                                <Show when=move || has_comment>
                                    <div style="margin-top: 8px; color: var(--colorNeutralForeground3); font-size: 13px;">
                                        {comment_text.clone()}
                                    </div>
                                </Show>

                                <div style="margin-top: 8px; font-size: 11px; color: var(--colorNeutralForeground3);">
                                    {format!("Drilldown отчёт • Создан: {}", created_at)}
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        // ── SQL Query карточка ──
                        let sql_full = a.sql_query.clone();
                        let is_long = sql_full.len() > 120;
                        let sql_short = if is_long {
                            // Обрезаем по символам, не байтам
                            let cut: String = sql_full.chars().take(120).collect();
                            format!("{}…", cut)
                        } else {
                            sql_full.clone()
                        };
                        let comment_text = a.base.comment.clone().unwrap_or_default();
                        let has_comment = !comment_text.is_empty();
                        let execution_count = a.execution_count;
                        let sql_expanded = RwSignal::new(false);

                        view! {
                            <div>
                                <div style="display: flex; justify-content: space-between; align-items: center;">
                                    <div style="display: flex; align-items: center; gap: 8px;">
                                        {icon("file-code")}
                                        <strong>{description}</strong>
                                    </div>
                                    <Button
                                        size=ButtonSize::Small
                                        appearance=ButtonAppearance::Secondary
                                        on_click=handle_open_sql.clone()
                                    >
                                        {icon("external-link")}
                                        " Открыть"
                                    </Button>
                                </div>

                                <Show when=move || has_comment>
                                    <div style="margin-top: 8px; color: var(--colorNeutralForeground3); font-size: 13px;">
                                        {comment_text.clone()}
                                    </div>
                                </Show>

                                // SQL блок: клик разворачивает/сворачивает полный запрос
                                <div
                                    style="margin-top: 8px; padding: 8px; background: var(--colorNeutralBackground1); border-radius: 4px; font-family: monospace; font-size: 12px; white-space: pre-wrap; overflow: hidden; cursor: pointer; user-select: text;"
                                    title=if is_long { "Нажмите чтобы развернуть / свернуть" } else { "" }
                                    on:click=move |_| { if is_long { sql_expanded.update(|v| *v = !*v); } }
                                >
                                    {move || {
                                        if sql_expanded.get() {
                                            sql_full.clone()
                                        } else {
                                            sql_short.clone()
                                        }
                                    }}
                                    {move || {
                                        if is_long {
                                            Some(view! {
                                                <span style="margin-left: 6px; font-size: 11px; color: var(--colorBrandForeground1); font-family: sans-serif;">
                                                    {move || if sql_expanded.get() { "▲ свернуть" } else { "▼ развернуть" }}
                                                </span>
                                            })
                                        } else {
                                            None
                                        }
                                    }}
                                </div>

                                <div style="margin-top: 8px; font-size: 11px; color: var(--colorNeutralForeground3);">
                                    {format!("Создан: {} • Выполнен {} раз", created_at, execution_count)}
                                </div>
                            </div>
                        }.into_any()
                    }
                } else {
                    view! { <div>"Загрузка артефакта..."</div> }.into_any()
                }
            }}
        </div>
    }
}
