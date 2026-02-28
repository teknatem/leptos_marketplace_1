use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::icons::icon;
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use thaw::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmArtifactListItem {
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub chat_id: String,
    pub agent_id: String,
    pub artifact_type: String,
    pub status: String,
    pub created_at: String,
    pub execution_count: i32,
}

async fn fetch_artifacts() -> Result<Vec<LlmArtifactListItem>, String> {
    let url = format!("{}/api/a019-llm-artifact", api_base());

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

    serde_json::from_str::<Vec<LlmArtifactListItem>>(&text)
        .map_err(|e| format!("Failed to parse response: {}", e))
}

async fn delete_artifact(id: &str) -> Result<(), String> {
    let url = format!("{}/api/a019-llm-artifact/{}", api_base(), id);

    let response = Request::delete(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to delete: {}", e))?;

    if response.status() != 200 {
        return Err(format!("Server error: {}", response.status()));
    }

    Ok(())
}

#[component]
#[allow(non_snake_case)]
pub fn LlmArtifactList() -> impl IntoView {
    let global_ctx = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    let items = RwSignal::new(Vec::<LlmArtifactListItem>::new());
    let error = RwSignal::new(None::<String>);

    let fetch = move || {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_artifacts().await {
                Ok(artifacts) => {
                    log!("✅ Fetched {} artifacts", artifacts.len());
                    items.set(artifacts);
                    error.set(None);
                }
                Err(e) => {
                    log!("❌ Failed to fetch artifacts: {}", e);
                    error.set(Some(e));
                }
            }
        });
    };

    Effect::new(move |_| {
        fetch();
    });

    let handle_open_artifact = {
        let global_ctx = global_ctx.clone();
        move |id: String, description: String| {
            use crate::layout::tabs::{detail_tab_label, pick_identifier};
            use contracts::domain::a019_llm_artifact::ENTITY_METADATA as A019;
            let key = format!("a019_llm_artifact_detail_{}", id);
            let identifier = pick_identifier(None, None, Some(&description), &id);
            let title = detail_tab_label(A019.ui.element_name, identifier);
            global_ctx.open_tab(&key, &title);
        }
    };

    let handle_delete = move |id: String| {
        wasm_bindgen_futures::spawn_local(async move {
            match delete_artifact(&id).await {
                Ok(_) => {
                    log!("✅ Artifact deleted: {}", id);
                    fetch();
                }
                Err(e) => {
                    log!("❌ Failed to delete artifact: {}", e);
                    error.set(Some(format!("Ошибка удаления: {}", e)));
                }
            }
        });
    };

    view! {
        <div id="a019_llm_artifact--list" data-page-category="legacy" style="padding: 20px;">
            <div style="display: flex; justify-content: space-between; align-items: center;">
                <h1 style="font-size: 24px; font-weight: bold;">"Артефакты LLM"</h1>
                <Button appearance=ButtonAppearance::Secondary on_click=move |_| fetch()>
                    {icon("refresh")}
                    " Обновить"
                </Button>
            </div>

            <div style="margin-top: 16px;">
                {move || error.get().map(|e| view! {
                    <div style="padding: 12px; background: var(--color-error-50); border: 1px solid var(--color-error-100); border-radius: 8px; display: flex; align-items: center; gap: 8px;">
                        <span style="color: var(--color-error); font-size: 18px;">"⚠"</span>
                        <span style="color: var(--color-error);">{e}</span>
                    </div>
                })}
            </div>

            <Table attr:style="margin-top: 20px;">
                <TableHeader>
                    <TableRow>
                        <TableHeaderCell resizable=true min_width=150.0>"Код"</TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=250.0>"Название"</TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=200.0>"Комментарий"</TableHeaderCell>
                        <TableHeaderCell min_width=100.0>"Тип"</TableHeaderCell>
                        <TableHeaderCell min_width=100.0>"Статус"</TableHeaderCell>
                        <TableHeaderCell min_width=100.0>"Выполнений"</TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=150.0>"Создан"</TableHeaderCell>
                        <TableHeaderCell min_width=80.0>"Действия"</TableHeaderCell>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {move || {
                        items.get().into_iter().map(|item| {
                            let id = item.id.clone();
                            let id_for_link = id.clone();
                            let id_for_delete = id.clone();
                            let description_for_link = item.description.clone();
                            let handle_open = handle_open_artifact.clone();

                            let comment_short = item.comment.as_ref()
                                .map(|c: &String| if c.len() > 50 { format!("{}...", &c[..50]) } else { c.clone() })
                                .unwrap_or_else(|| "-".to_string());

                            let type_label = match item.artifact_type.as_str() {
                                "sql_query" => "SQL",
                                _ => "Unknown",
                            };

                            let status_label = match item.status.as_str() {
                                "draft" => "Черновик",
                                "active" => "Активен",
                                "deprecated" => "Устарел",
                                "failed" => "Ошибка",
                                _ => "Unknown",
                            };

                            view! {
                                <TableRow>
                                    <TableCell>
                                        <TableCellLayout>{item.code.clone()}</TableCellLayout>
                                    </TableCell>
                                    <TableCell>
                                        <TableCellLayout>
                                            <a
                                                href="#"
                                                style="color: var(--colorBrandForeground1); text-decoration: none; cursor: pointer;"
                                                on:click=move |e: web_sys::MouseEvent| {
                                                    e.prevent_default();
                                                    handle_open(id_for_link.clone(), description_for_link.clone());
                                                }
                                            >
                                                {item.description.clone()}
                                            </a>
                                        </TableCellLayout>
                                    </TableCell>
                                    <TableCell>
                                        <TableCellLayout>{comment_short}</TableCellLayout>
                                    </TableCell>
                                    <TableCell>
                                        <TableCellLayout>{type_label}</TableCellLayout>
                                    </TableCell>
                                    <TableCell>
                                        <TableCellLayout>{status_label}</TableCellLayout>
                                    </TableCell>
                                    <TableCell>
                                        <TableCellLayout>{item.execution_count}</TableCellLayout>
                                    </TableCell>
                                    <TableCell>
                                        <TableCellLayout>{item.created_at.clone()}</TableCellLayout>
                                    </TableCell>
                                    <TableCell>
                                        <TableCellLayout>
                                            <Button
                                                size=ButtonSize::Small
                                                appearance=ButtonAppearance::Subtle
                                                on_click=move |_| handle_delete(id_for_delete.clone())
                                            >
                                                {icon("delete")}
                                            </Button>
                                        </TableCellLayout>
                                    </TableCell>
                                </TableRow>
                            }
                        }).collect_view()
                    }}
                </TableBody>
            </Table>
        </div>
    }
}
