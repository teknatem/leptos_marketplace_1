//! BiDashboard view page — итоговое представление дашборда
//!
//! Загружает дашборд по ID, отображает дерево групп с indicator-картами в iframe-srcdoc.
//! Фильтр-панель позволяет переопределить global_filters на текущую сессию.

use crate::shared::api_utils::api_base;
use crate::shared::bi_card::{render_srcdoc, IndicatorCardParams};
use crate::shared::page_frame::PageFrame;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

// ── Local structs (mirror of contracts, for deserialization) ────────────────

#[derive(Clone, Debug, serde::Deserialize)]
struct DashboardItem {
    pub indicator_id: String,
    pub col_class: String,
    #[serde(default)]
    pub param_overrides: std::collections::HashMap<String, String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct DashboardGroup {
    pub id: String,
    pub title: String,
    pub sort_order: i32,
    #[serde(default)]
    pub items: Vec<DashboardItem>,
    #[serde(default)]
    pub subgroups: Vec<DashboardGroup>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct DashboardLayout {
    #[serde(default)]
    pub groups: Vec<DashboardGroup>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct GlobalFilter {
    pub key: String,
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct BiDashboardData {
    pub id: String,
    pub code: String,
    pub description: String,
    pub layout: DashboardLayout,
    pub global_filters: Vec<GlobalFilter>,
    pub rating: Option<u8>,
    pub status: String,
}

async fn fetch_dashboard(id: &str) -> Result<serde_json::Value, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a025-bi-dashboard/{}", api_base(), id);
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

fn get_app_theme() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let theme = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item("app_theme").ok().flatten())
            .unwrap_or_else(|| "dark".to_string());
        if theme == "light" { "light".to_string() } else { "dark".to_string() }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        "dark".to_string()
    }
}

/// Renders a single indicator card as iframe srcdoc.
/// Uses preview mode (static data) since execution engine is not yet implemented.
#[component]
fn IndicatorCard(
    item: DashboardItem,
    session_filters: RwSignal<Vec<(String, String)>>,
) -> impl IntoView {
    let theme = get_app_theme();

    // Build merged params: global filters → param_overrides (latter wins)
    let params_info = {
        let mut merged: std::collections::HashMap<String, String> = session_filters
            .get_untracked()
            .into_iter()
            .collect();
        for (k, v) in &item.param_overrides {
            merged.insert(k.clone(), v.clone());
        }
        merged
    };

    let filter_hint = params_info
        .iter()
        .map(|(k, v)| format!("{}: {}", k, v))
        .collect::<Vec<_>>()
        .join(", ");

    let col_class = item.col_class.clone();
    let indicator_id = item.indicator_id.clone();

    // In preview mode we show a placeholder srcdoc with the indicator ID and filters.
    // Future: fetch actual data from the indicator's data_spec and populate values.
    let srcdoc = render_srcdoc(&IndicatorCardParams {
        style_name: "classic".to_string(),
        theme: theme.clone(),
        name: format!("Индикатор {}", &indicator_id[..8.min(indicator_id.len())]),
        value: "—".to_string(),
        unit: String::new(),
        delta: String::new(),
        delta_dir: "flat".to_string(),
        status: "neutral".to_string(),
        chip: String::new(),
        col_class: col_class.clone(),
        progress: 0,
        spark_points: vec![],
        meta_1: filter_hint,
        meta_2: String::new(),
        hint: String::new(),
        footer_1: String::new(),
        footer_2: String::new(),
        custom_html: None,
        custom_css: None,
    });

    view! {
        <div class={format!("dashboard-card dashboard-card--{}", col_class)}>
            <iframe
                class="dashboard-card__iframe"
                sandbox="allow-same-origin"
                srcdoc=srcdoc
            />
        </div>
    }
}

/// Renders a group recursively (group title + items grid + subgroups)
fn render_group(
    group: DashboardGroup,
    depth: usize,
    session_filters: RwSignal<Vec<(String, String)>>,
) -> AnyView {
    let heading_class = if depth == 0 {
        "dashboard-group__title dashboard-group__title--root"
    } else {
        "dashboard-group__title dashboard-group__title--sub"
    };

    let subgroups_views: Vec<AnyView> = group
        .subgroups
        .into_iter()
        .map(|sub| render_group(sub, depth + 1, session_filters))
        .collect();

    view! {
        <div class="dashboard-group">
            <div class=heading_class>
                {group.title.clone()}
            </div>
            <div class="dashboard-group__grid">
                {group.items.into_iter().map(|item| {
                    view! {
                        <IndicatorCard item=item session_filters=session_filters />
                    }
                }).collect::<Vec<_>>()}
            </div>
            {subgroups_views}
        </div>
    }
    .into_any()
}

#[component]
pub fn BiDashboardView(id: String) -> impl IntoView {
    let loading: RwSignal<bool> = RwSignal::new(false);
    let error: RwSignal<Option<String>> = RwSignal::new(None);
    let dashboard: RwSignal<Option<BiDashboardData>> = RwSignal::new(None);

    // Session filter overrides (key, value pairs)
    let session_filters: RwSignal<Vec<(String, String)>> = RwSignal::new(vec![]);

    // Load dashboard data
    {
        let id = id.clone();
        leptos::task::spawn_local(async move {
            loading.set(true);
            match fetch_dashboard(&id).await {
                Ok(raw) => {
                    match serde_json::from_value::<BiDashboardData>(raw) {
                        Ok(data) => {
                            // Seed session filters from global_filters defaults
                            let seeds: Vec<(String, String)> = data
                                .global_filters
                                .iter()
                                .map(|f| (f.key.clone(), f.value.clone()))
                                .collect();
                            session_filters.set(seeds);
                            dashboard.set(Some(data));
                        }
                        Err(e) => error.set(Some(format!("Ошибка парсинга: {}", e))),
                    }
                }
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    }

    view! {
        <PageFrame page_id="a025_bi_dashboard--view" category="dashboard">
            {move || if loading.get() {
                view! { <div class="placeholder">"Загрузка дашборда..."</div> }.into_any()
            } else if let Some(e) = error.get() {
                view! {
                    <div class="warning-box">
                        <span class="warning-box__icon">"⚠"</span>
                        <span class="warning-box__text">{e}</span>
                    </div>
                }.into_any()
            } else if let Some(data) = dashboard.get() {
                let title = data.description.clone();
                let code = data.code.clone();
                let global_filters_def = data.global_filters.clone();
                let groups = data.layout.groups.clone();
                let groups_empty = groups.is_empty();

                view! {
                    // Header
                    <div class="page__header">
                        <div class="page__header-left">
                            <h1 class="page__title">{title}</h1>
                            <span class="text-muted" style="margin-left: 8px">{code}</span>
                        </div>
                    </div>

                    // Filter panel
                    {if !global_filters_def.is_empty() {
                        view! {
                            <div class="dashboard-filters">
                                {global_filters_def.into_iter().map(|filter| {
                                    let key = filter.key.clone();
                                    let label = filter.label.clone();
                                    let initial_value = filter.value.clone();
                                    let sf = session_filters.clone();

                                    view! {
                                        <div class="dashboard-filter">
                                            <label class="dashboard-filter__label">{label}</label>
                                            <input
                                                type="text"
                                                class="form__input form__input--sm"
                                                value=initial_value
                                                on:input=move |ev| {
                                                    let val = ev.target().unwrap()
                                                        .unchecked_into::<web_sys::HtmlInputElement>()
                                                        .value();
                                                    sf.update(|v| {
                                                        if let Some(entry) = v.iter_mut().find(|(k, _)| k == &key) {
                                                            entry.1 = val;
                                                        }
                                                    });
                                                }
                                            />
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }}

                    // Dashboard content
                    <div class="dashboard-content">
                        {groups.into_iter().map(|group| {
                            render_group(group, 0, session_filters)
                        }).collect::<Vec<_>>()}

                        {if groups_empty {
                            view! {
                                <div class="placeholder">
                                    "Дашборд пуст. Добавьте группы и индикаторы в редакторе."
                                </div>
                            }.into_any()
                        } else {
                            view! { <div></div> }.into_any()
                        }}
                    </div>
                }.into_any()
            } else {
                view! { <div class="placeholder">"Дашборд не найден"</div> }.into_any()
            }}
        </PageFrame>
    }
}
