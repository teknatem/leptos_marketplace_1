use crate::domain::a017_llm_agent::ui::details::LlmAgentDetails;
use crate::shared::api_utils::api_base;
use crate::shared::icons::icon;
use crate::shared::modal_stack::ModalStackService;
use crate::shared::page_frame::PageFrame;
use contracts::domain::a017_llm_agent::aggregate::LlmAgent;
use leptos::prelude::*;
use thaw::*;

#[component]
#[allow(non_snake_case)]
pub fn LlmAgentList() -> impl IntoView {
    let modal_stack =
        use_context::<ModalStackService>().expect("ModalStackService not found in context");
    let (items, set_items) = signal::<Vec<LlmAgent>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);
    let show_modal = RwSignal::new(false);
    let (editing_id, set_editing_id) = signal::<Option<String>>(None);

    let fetch = move || {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_agents().await {
                Ok(v) => {
                    set_items.set(v);
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    let handle_create_new = move || {
        set_editing_id.set(None);
        show_modal.set(true);
    };

    let handle_edit = move |id: String| {
        set_editing_id.set(Some(id));
        show_modal.set(true);
    };

    let handle_delete = move |id: String| {
        let confirmed = {
            if let Some(win) = web_sys::window() {
                win.confirm_with_message("Удалить агента LLM?")
                    .unwrap_or(false)
            } else {
                false
            }
        };
        if !confirmed {
            return;
        }

        wasm_bindgen_futures::spawn_local(async move {
            let _ = delete_agent(&id).await;
        });
        fetch();
    };

    fetch();

    view! {
        <PageFrame page_id="a017_llm_agent--list" category="list">
            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center>
                <h1 style="font-size: 24px; font-weight: bold;">{"Агенты LLM"}</h1>
                <Space>
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=move |_| handle_create_new()
                    >
                        {icon("plus")}
                        " Новый агент"
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| fetch()
                    >
                        {icon("refresh")}
                        " Обновить"
                    </Button>
                </Space>
            </Flex>
            <div style="margin-top: 16px;">
            {move || error.get().map(|e| view! {
                <div style="padding: 12px; background: var(--color-error-50); border: 1px solid var(--color-error-100); border-radius: 8px; display: flex; align-items: center; gap: 8px;">
                    <span style="color: var(--color-error); font-size: 18px;">"⚠"</span>
                    <span style="color: var(--color-error);">{e}</span>
                </div>
            })}
            </div>

            <Table>
                <TableHeader>
                    <TableRow>
                        <TableHeaderCell resizable=true min_width=200.0>"Наименование"</TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=120.0>"Провайдер"</TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=150.0>"Модель"</TableHeaderCell>
                        <TableHeaderCell min_width=80.0>"Основной"</TableHeaderCell>
                        <TableHeaderCell min_width=100.0>"Действия"</TableHeaderCell>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {move || items.get().into_iter().map(|agent| {
                        let id = agent.to_string_id();
                        let id_for_link = id.clone();
                        let id_for_delete = id.clone();
                        view! {
                            <TableRow>
                                <TableCell>
                                    <TableCellLayout>
                                        <a
                                            href="#"
                                            style="color: var(--colorBrandForeground1); text-decoration: none; cursor: pointer;"
                                            on:click=move |e| {
                                                e.prevent_default();
                                                handle_edit(id_for_link.clone());
                                            }
                                        >
                                            {agent.base.description}
                                        </a>
                                    </TableCellLayout>
                                </TableCell>
                                <TableCell>
                                    <TableCellLayout>
                                        {agent.provider_type.as_str()}
                                    </TableCellLayout>
                                </TableCell>
                                <TableCell>
                                    <TableCellLayout>
                                        {agent.model_name}
                                    </TableCellLayout>
                                </TableCell>
                                <TableCell>
                                    <TableCellLayout>
                                        {if agent.is_primary { "Да" } else { "" }}
                                    </TableCellLayout>
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
                    }).collect_view()}
                </TableBody>
            </Table>

            <Show when=move || show_modal.get()>
                {move || {
                    let id_val = editing_id.get();
                    modal_stack.push_with_frame(
                        Some("max-width: min(1050px, 95vw); width: min(1050px, 95vw);".to_string()),
                        Some("llm-agent-modal".to_string()),
                        move |handle| {
                            let id_signal = Signal::derive({
                                let id_val = id_val.clone();
                                move || id_val.clone()
                            });

                            view! {
                                <LlmAgentDetails
                                    id=id_signal
                                    on_saved=Callback::new({
                                        let handle = handle.clone();
                                        move |_| {
                                            handle.close();
                                            fetch();
                                        }
                                    })
                                    on_cancel=Callback::new({
                                        let handle = handle.clone();
                                        move |_| handle.close()
                                    })
                                />
                            }.into_any()
                        },
                    );

                    show_modal.set(false);
                    set_editing_id.set(None);
                    view! { <></> }
                }}
            </Show>
        </PageFrame>
    }
}

async fn fetch_agents() -> Result<Vec<LlmAgent>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a017-llm-agent", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    let data: Vec<LlmAgent> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}

async fn delete_agent(id: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("DELETE");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a017-llm-agent/{}", api_base(), id);
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    Ok(())
}
