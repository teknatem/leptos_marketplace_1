use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::icons::icon;
use contracts::domain::a019_llm_artifact::aggregate::LlmArtifact;
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

    let handle_open = {
        let artifact_id = artifact_id.clone();
        move |_| {
            let key = format!("a019_llm_artifact_detail_{}", artifact_id);
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
                    let sql_preview = if a.sql_query.len() > 100 {
                        format!("{}...", &a.sql_query[..100])
                    } else {
                        a.sql_query.clone()
                    };
                    let comment_text = a.base.comment.clone().unwrap_or_default();
                    let has_comment = !comment_text.is_empty();
                    let description = a.base.description.clone();
                    let created_at = a.base.metadata.created_at.format("%d.%m.%Y %H:%M").to_string();
                    let execution_count = a.execution_count;

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
                                    on_click=handle_open.clone()
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

                            <div style="margin-top: 8px; padding: 8px; background: var(--colorNeutralBackground1); border-radius: 4px; font-family: monospace; font-size: 12px; white-space: pre-wrap; overflow: hidden;">
                                {sql_preview}
                            </div>

                            <div style="margin-top: 8px; font-size: 11px; color: var(--colorNeutralForeground3);">
                                {format!("Создан: {} • Выполнен {} раз", created_at, execution_count)}
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <div>"Загрузка артефакта..."</div> }.into_any()
                }
            }}
        </div>
    }
}
