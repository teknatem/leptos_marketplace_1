//! Каталог LLM-навыков (skills): read-only обзор реестра, который бэкенд отдаёт LLM.
//! Показывает, какие навыки есть, их инструменты, интенты-триггеры и для каких ролей доступны.

use crate::shared::api_utils::api_base;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_LIST;
use leptos::prelude::*;
use serde::Deserialize;

#[derive(Clone, Deserialize)]
struct SkillInfo {
    id: String,
    title: String,
    description: String,
    #[serde(default)]
    intents: Vec<String>,
    #[serde(default)]
    tools: Vec<String>,
    #[serde(default)]
    allowed_for: Vec<String>,
}

#[derive(Clone, Deserialize)]
struct Catalog {
    #[serde(default)]
    core_tools: Vec<String>,
    #[serde(default)]
    skills: Vec<SkillInfo>,
}

#[component]
#[allow(non_snake_case)]
pub fn LlmSkillList() -> impl IntoView {
    let data = RwSignal::new(None::<Catalog>);
    let error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_catalog().await {
                Ok(c) => {
                    data.set(Some(c));
                    error.set(None);
                }
                Err(e) => error.set(Some(e)),
            }
        });
    });

    view! {
        <PageFrame page_id="llm_skills--list" category=PAGE_CAT_LIST class="llm-skills-list">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Навыки LLM"</h1>
                    <span class="page__header-meta">
                        "Реестр навыков (skills), которые ассистент активирует под задачу"
                    </span>
                </div>
            </div>

            <div class="page__content">
                {move || error.get().map(|e| view! {
                    <div class="warning-box" style="background: var(--color-error-50); border-color: var(--color-error-100); margin-bottom: var(--spacing-md);">
                        <span class="warning-box__text" style="color: var(--color-error);">{e}</span>
                    </div>
                })}

                {move || data.get().map(|c| {
                    let core = c.core_tools.join(", ");
                    let cards = c.skills.into_iter().map(|s| {
                        let intents = s.intents.into_iter().map(|i| badge(i, "primary")).collect_view();
                        let roles = s.allowed_for.into_iter().map(|r| badge(r, "success")).collect_view();
                        let tools = s.tools.join(", ");
                        view! {
                            <div class="llm-skills__card">
                                <div class="llm-skills__card-head">
                                    <strong class="llm-skills__title">{s.title}</strong>
                                    <code class="llm-skills__id">{s.id}</code>
                                </div>
                                <div class="llm-skills__desc">{s.description}</div>
                                <div class="llm-skills__meta">
                                    <span class="llm-skills__label">"Триггеры (интенты):"</span>
                                    <span class="llm-skills__badges">{intents}</span>
                                </div>
                                <div class="llm-skills__meta">
                                    <span class="llm-skills__label">"Доступен ролям:"</span>
                                    <span class="llm-skills__badges">{roles}</span>
                                </div>
                                <div class="llm-skills__meta">
                                    <span class="llm-skills__label">"Инструменты:"</span>
                                    <span class="llm-skills__tools">{tools}</span>
                                </div>
                            </div>
                        }
                    }).collect_view();

                    view! {
                        <div class="llm-skills__core">
                            <strong>"Core (всегда активен): "</strong>
                            <span class="llm-skills__tools">{core}</span>
                        </div>
                        {cards}
                    }
                })}
            </div>
        </PageFrame>
    }
}

/// Бейдж в стиле страницы sys_users (глобальные классы `badge badge--*`).
fn badge(text: String, variant: &'static str) -> impl IntoView {
    view! { <span class=format!("badge badge--{variant}")>{text}</span> }
}

async fn fetch_catalog() -> Result<Catalog, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/llm-skills", api_base());
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
