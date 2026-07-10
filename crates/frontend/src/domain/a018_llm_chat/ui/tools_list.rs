//! Каталог LLM-инструментов (tools): read-only реестр инструментов, которые ассистент
//! может вызывать. Мастер-деталь: слева список (поиск + группировка по категории),
//! справа карточка выбранного инструмента (описание, навыки, core, JSON-схема параметров).

use crate::shared::api_utils::api_base;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_LIST;
use leptos::prelude::*;
use serde::Deserialize;

#[derive(Clone, Deserialize)]
struct ToolInfo {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    parameters: serde_json::Value,
    #[serde(default)]
    category: String,
    #[serde(default)]
    is_core: bool,
    #[serde(default)]
    skills: Vec<String>,
}

#[derive(Clone, Deserialize)]
struct ToolsCatalog {
    #[serde(default)]
    tools: Vec<ToolInfo>,
}

/// Человекочитаемые подписи категорий и порядок вывода групп.
const CATEGORIES: &[(&str, &str)] = &[
    ("meta", "Мета / навыки"),
    ("data", "Данные и запросы"),
    ("shared", "Базовые"),
    ("analyst", "Аналитика"),
    ("chart", "Графики"),
    ("table", "Таблицы"),
    ("plugin", "Плагины"),
    ("admin", "Администрирование"),
    ("kb", "База знаний"),
];

fn category_label(id: &str) -> String {
    CATEGORIES
        .iter()
        .find(|(cid, _)| *cid == id)
        .map(|(_, label)| label.to_string())
        .unwrap_or_else(|| id.to_string())
}

/// Бейдж в стиле страницы sys_users (глобальные классы `badge badge--*`).
fn badge(text: String, variant: &'static str) -> impl IntoView {
    view! { <span class=format!("badge badge--{variant}")>{text}</span> }
}

#[component]
#[allow(non_snake_case)]
pub fn LlmToolList() -> impl IntoView {
    let data = RwSignal::new(None::<ToolsCatalog>);
    let error = RwSignal::new(None::<String>);
    let selected = RwSignal::new(None::<String>);
    let filter = RwSignal::new(String::new());

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

    // Левая колонка: поиск + сгруппированный по категориям список.
    let list_col = move || {
        data.get().map(|c| {
            let needle = filter.get().to_lowercase();
            let sel = selected.get();
            let groups = CATEGORIES
                .iter()
                .filter_map(|(cid, _)| {
                    let cid = *cid;
                    let items: Vec<ToolInfo> = c
                        .tools
                        .iter()
                        .filter(|t| t.category == cid)
                        .filter(|t| {
                            needle.is_empty()
                                || t.name.to_lowercase().contains(&needle)
                                || t.description.to_lowercase().contains(&needle)
                        })
                        .cloned()
                        .collect();
                    if items.is_empty() {
                        return None;
                    }
                    let rows = items.into_iter().map(|t| {
                    let name = t.name.clone();
                    let is_active = sel.as_deref() == Some(name.as_str());
                    let skills_n = t.skills.len();
                    let name_click = name.clone();
                    view! {
                        <div
                            class="llm-tools__row"
                            class:llm-tools__row--active=is_active
                            on:click=move |_| selected.set(Some(name_click.clone()))
                        >
                            <span class="llm-tools__row-name">{name}</span>
                            <span class="llm-tools__row-badges">
                                {t.is_core.then(|| badge("core".to_string(), "warning"))}
                                {(skills_n > 0).then(|| badge(format!("{skills_n}"), "neutral"))}
                            </span>
                        </div>
                    }
                }).collect_view();
                    Some(view! {
                        <div class="llm-tools__group">
                            <div class="llm-tools__group-title">{category_label(cid)}</div>
                            {rows}
                        </div>
                    })
                })
                .collect_view();

            view! {
                <input
                    class="llm-tools__search"
                    type="text"
                    placeholder="Поиск инструмента..."
                    prop:value=move || filter.get()
                    on:input=move |ev| filter.set(event_target_value(&ev))
                />
                <div class="llm-tools__groups">{groups}</div>
            }
        })
    };

    // Правая колонка: карточка выбранного инструмента.
    let detail_col = move || {
        let sel = selected.get();
        let tool = sel.as_ref().and_then(|name| {
            data.get()
                .and_then(|c| c.tools.into_iter().find(|t| &t.name == name))
        });
        match tool {
            None => view! {
                <div class="llm-tools__placeholder">"Выберите инструмент из списка слева"</div>
            }
            .into_any(),
            Some(t) => {
                let skills = if t.skills.is_empty() {
                    view! { <span style="color:var(--colorNeutralForeground3);">"—"</span> }
                        .into_any()
                } else {
                    t.skills
                        .iter()
                        .cloned()
                        .map(|s| badge(s, "primary"))
                        .collect_view()
                        .into_any()
                };
                let params_table = render_params(&t.parameters);
                let raw = serde_json::to_string_pretty(&t.parameters)
                    .unwrap_or_else(|_| "{}".to_string());
                view! {
                    <div class="llm-tools__card">
                        <div class="llm-tools__card-head">
                            <code class="llm-tools__card-name">{t.name.clone()}</code>
                            <span class="llm-tools__card-cat">{category_label(&t.category)}</span>
                            {t.is_core.then(|| badge("core".to_string(), "warning"))}
                        </div>
                        <div class="llm-tools__card-skills">
                            <span class="llm-tools__label">"Навыки:"</span>
                            <span class="llm-tools__badges">{skills}</span>
                        </div>
                        <div class="llm-tools__card-desc">{t.description.clone()}</div>
                        <div class="llm-tools__label">"Параметры"</div>
                        {params_table}
                        <details class="llm-tools__raw">
                            <summary>"JSON Schema"</summary>
                            <pre>{raw}</pre>
                        </details>
                    </div>
                }
                .into_any()
            }
        }
    };

    view! {
        <PageFrame page_id="llm_tools--list" category=PAGE_CAT_LIST class="llm-tools-list">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Инструменты LLM"</h1>
                    <span class="page__header-meta">
                        "Реестр инструментов (tools), которые ассистент вызывает под задачу"
                    </span>
                </div>
            </div>

            <div class="page__content">
                {move || error.get().map(|e| view! {
                    <div class="warning-box" style="background: var(--color-error-50); border-color: var(--color-error-100); margin-bottom: var(--spacing-md);">
                        <span class="warning-box__text" style="color: var(--color-error);">{e}</span>
                    </div>
                })}

                <div class="llm-tools__split">
                    <div class="llm-tools__master">{list_col}</div>
                    <div class="llm-tools__detail">{detail_col}</div>
                </div>
            </div>
        </PageFrame>
    }
}

/// Отрисовать `properties` JSON-схемы таблицей: имя / тип / обяз. / описание.
fn render_params(schema: &serde_json::Value) -> AnyView {
    let props = schema.get("properties").and_then(|v| v.as_object());
    let required: Vec<String> = schema
        .get("required")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    match props {
        None => view! {
            <div class="llm-tools__noparams" style="color:var(--colorNeutralForeground3); font-size:13px; margin:6px 0 12px;">
                "Без параметров"
            </div>
        }.into_any(),
        Some(props) => {
            let rows = props.iter().map(|(name, spec)| {
                let ty = spec.get("type").and_then(|v| v.as_str()).unwrap_or("—").to_string();
                let desc = spec.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let req = required.contains(name);
                let name = name.clone();
                view! {
                    <tr>
                        <td><code>{name}</code>{req.then(|| view! { <span style="color:var(--color-error);"> "*"</span> })}</td>
                        <td><span class="llm-tools__type">{ty}</span></td>
                        <td class="llm-tools__pdesc">{desc}</td>
                    </tr>
                }
            }).collect_view();
            view! {
                <table class="llm-tools__params">
                    <thead>
                        <tr><th>"Имя"</th><th>"Тип"</th><th>"Описание"</th></tr>
                    </thead>
                    <tbody>{rows}</tbody>
                </table>
            }.into_any()
        }
    }
}

async fn fetch_catalog() -> Result<ToolsCatalog, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/llm-tools", api_base());
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
