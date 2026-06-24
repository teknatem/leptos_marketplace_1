//! Details-страница пакета контекста: показывает ровно ту информацию, которую
//! получает LLM (rendered_text), и ссылку на исходную страницу (агрегат/отчёт).

use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::icons::icon;
use crate::shared::markdown::Markdown;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_DETAIL;
use contracts::domain::a018_llm_chat::context::ContextPackageSummary;
use leptos::prelude::*;
use thaw::*;

#[component]
#[allow(non_snake_case)]
pub fn LlmContextDetails(id: String, on_close: Callback<()>) -> impl IntoView {
    let pkg = RwSignal::new(None::<ContextPackageSummary>);
    let error = RwSignal::new(None::<String>);
    let nav_ctx = use_context::<AppGlobalContext>();

    Effect::new({
        let id = id.clone();
        move |_| {
            let id = id.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match fetch_context_package(&id).await {
                    Ok(p) => {
                        pkg.set(Some(p));
                        error.set(None);
                    }
                    Err(e) => error.set(Some(e)),
                }
            });
        }
    });

    view! {
        <PageFrame page_id="a018_llm_context--detail" category=PAGE_CAT_DETAIL class="a018-llm-context-detail">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">
                        {move || pkg.get().map(|p| p.title).unwrap_or_else(|| "Контекст LLM".to_string())}
                    </h1>
                    <span class="page__header-meta">
                        {move || pkg.get().map(|p| format!("Тип страницы: {}", p.page_type)).unwrap_or_default()}
                    </span>
                </div>
                <div class="page__header-right">
                    <Button appearance=ButtonAppearance::Secondary on_click=move |_| on_close.run(())>
                        {icon("x")}
                        " Закрыть"
                    </Button>
                </div>
            </div>

            <div class="page__content">
                {move || error.get().map(|e| view! {
                    <div class="warning-box" style="background: var(--color-error-50); border-color: var(--color-error-100); margin-bottom: var(--spacing-md);">
                        <span class="warning-box__text" style="color: var(--color-error);">{e}</span>
                    </div>
                })}

                {move || pkg.get().map(|p| {
                    let page_key = p.page_key.clone();
                    let title = p.title.clone();
                    view! {
                        <div style="margin-bottom: 12px;">
                            <a
                                href="#"
                                style="display: inline-flex; align-items: center; gap: 6px; \
                                       color: var(--colorBrandForeground1); text-decoration: none; cursor: pointer; font-size: 13px;"
                                on:click=move |e| {
                                    e.prevent_default();
                                    if let Some(c) = nav_ctx {
                                        c.open_tab(&page_key, &title);
                                    }
                                }
                            >
                                {icon("link")}
                                " Открыть исходную страницу"
                            </a>
                        </div>
                        <div style="padding: 12px 14px; background: var(--colorNeutralBackground1); \
                                    border: 1px solid var(--colorNeutralStroke2); border-radius: 8px;">
                            <Markdown text=p.rendered_text.clone() />
                        </div>
                    }
                })}
            </div>
        </PageFrame>
    }
}

async fn fetch_context_package(context_id: &str) -> Result<ContextPackageSummary, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a018-llm-chat-context/{}", api_base(), context_id);
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
    serde_json::from_str(&text).map_err(|e| format!("{e}"))
}
