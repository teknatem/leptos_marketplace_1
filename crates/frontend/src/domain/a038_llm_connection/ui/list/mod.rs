use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use contracts::domain::a038_llm_connection::aggregate::LlmConnection;
use leptos::prelude::*;
use thaw::*;

#[component]
#[allow(non_snake_case)]
pub fn LlmConnectionList() -> impl IntoView {
    let tabs_store =
        use_context::<AppGlobalContext>().expect("AppGlobalContext not found in context");
    let (items, set_items) = signal::<Vec<LlmConnection>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);

    let fetch = move || {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_connections().await {
                Ok(v) => {
                    set_items.set(v);
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    let handle_create_new = move || {
        tabs_store.open_tab("a038_llm_connection_details_new", "Новое подключение LLM");
    };

    let handle_edit = move |id: String, title: String| {
        let title = if title.trim().is_empty() {
            "Подключение LLM".to_string()
        } else {
            title
        };
        tabs_store.open_tab(&format!("a038_llm_connection_details_{id}"), &title);
    };

    let handle_delete = move |id: String| {
        let confirmed = {
            if let Some(win) = web_sys::window() {
                win.confirm_with_message("Удалить подключение LLM?")
                    .unwrap_or(false)
            } else {
                false
            }
        };
        if !confirmed {
            return;
        }

        wasm_bindgen_futures::spawn_local(async move {
            let _ = delete_connection(&id).await;
        });
        fetch();
    };

    fetch();

    view! {
        <PageFrame page_id="a038_llm_connection--list" category="list">
            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center>
                <h1 style="font-size: 24px; font-weight: bold;">{"Подключения LLM"}</h1>
                <Space>
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=move |_| handle_create_new()
                    >
                        {icon("plus")}
                        " Новое подключение"
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
                        <TableHeaderCell min_width=80.0>"Основное"</TableHeaderCell>
                        <TableHeaderCell min_width=100.0>"Действия"</TableHeaderCell>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {move || items.get().into_iter().map(|connection| {
                        let id = connection.to_string_id();
                        let id_for_link = id.clone();
                        let id_for_delete = id.clone();
                        let title = connection.base.description.clone();
                        view! {
                            <TableRow>
                                <TableCell>
                                    <TableCellLayout>
                                        <a
                                            href="#"
                                            style="color: var(--colorBrandForeground1); text-decoration: none; cursor: pointer;"
                                            on:click=move |e| {
                                                e.prevent_default();
                                                handle_edit(id_for_link.clone(), title.clone());
                                            }
                                        >
                                            {connection.base.description}
                                        </a>
                                    </TableCellLayout>
                                </TableCell>
                                <TableCell>
                                    <TableCellLayout>
                                        {connection.provider_type.as_str()}
                                    </TableCellLayout>
                                </TableCell>
                                <TableCell>
                                    <TableCellLayout>
                                        {connection.model_name}
                                    </TableCellLayout>
                                </TableCell>
                                <TableCell>
                                    <TableCellLayout>
                                        {if connection.is_primary { "Да" } else { "" }}
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
        </PageFrame>
    }
}

async fn fetch_connections() -> Result<Vec<LlmConnection>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a038-llm-connection", api_base());
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
    let data: Vec<LlmConnection> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}

async fn delete_connection(id: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("DELETE");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a038-llm-connection/{}", api_base(), id);
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
