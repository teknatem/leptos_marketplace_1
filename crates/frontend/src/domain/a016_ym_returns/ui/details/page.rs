//! YM Returns Detail - Main Page Component

use super::model::YmReturnDetailDto;
use super::tabs::{GeneralTab, JsonTab, LinesTab, ProjectionsTab};
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;

#[component]
pub fn YmReturnDetail(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let (return_data, set_return_data) = signal::<Option<YmReturnDetailDto>>(None);
    let (raw_json_from_ym, set_raw_json_from_ym) = signal::<Option<String>>(None);
    let (projections, set_projections) = signal::<Option<serde_json::Value>>(None);
    let (projections_loading, set_projections_loading) = signal(false);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal::<Option<String>>(None);
    let (active_tab, set_active_tab) = signal("general");

    // Sort state for lines table
    let (lines_sort_column, set_lines_sort_column) = signal::<Option<&'static str>>(None);
    let (lines_sort_asc, set_lines_sort_asc) = signal(true);

    // Sort state for projections table
    let (proj_sort_column, set_proj_sort_column) = signal::<Option<&'static str>>(None);
    let (proj_sort_asc, set_proj_sort_asc) = signal(true);

    // Load return details
    Effect::new(move || {
        let id = id.clone();
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let url = format!("http://localhost:3000/api/a016/ym-returns/{}", id);

            match Request::get(&url).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status == 200 {
                        match response.text().await {
                            Ok(text) => {
                                match serde_json::from_str::<YmReturnDetailDto>(&text) {
                                    Ok(data) => {
                                        let raw_payload_ref =
                                            data.source_meta.raw_payload_ref.clone();
                                        let return_id = data.id.clone();
                                        set_return_data.set(Some(data));
                                        set_loading.set(false);

                                        // Async load raw JSON
                                        wasm_bindgen_futures::spawn_local(async move {
                                            let raw_url = format!(
                                                "http://localhost:3000/api/a016/raw/{}",
                                                raw_payload_ref
                                            );
                                            match Request::get(&raw_url).send().await {
                                                Ok(resp) => {
                                                    if resp.status() == 200 {
                                                        if let Ok(text) = resp.text().await {
                                                            if let Ok(json_value) =
                                                                serde_json::from_str::<
                                                                    serde_json::Value,
                                                                >(
                                                                    &text
                                                                )
                                                            {
                                                                if let Ok(formatted) =
                                                                    serde_json::to_string_pretty(
                                                                        &json_value,
                                                                    )
                                                                {
                                                                    set_raw_json_from_ym
                                                                        .set(Some(formatted));
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    log!("Failed to load raw JSON: {:?}", e);
                                                }
                                            }
                                        });

                                        // Async load projections
                                        wasm_bindgen_futures::spawn_local(async move {
                                            set_projections_loading.set(true);
                                            let projections_url = format!(
                                                "http://localhost:3000/api/a016/ym-returns/{}/projections",
                                                return_id
                                            );
                                            match Request::get(&projections_url).send().await {
                                                Ok(resp) => {
                                                    if resp.status() == 200 {
                                                        if let Ok(text) = resp.text().await {
                                                            if let Ok(proj_data) =
                                                                serde_json::from_str::<
                                                                    serde_json::Value,
                                                                >(
                                                                    &text
                                                                )
                                                            {
                                                                set_projections
                                                                    .set(Some(proj_data));
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    log!("Failed to load projections: {:?}", e);
                                                }
                                            }
                                            set_projections_loading.set(false);
                                        });
                                    }
                                    Err(e) => {
                                        log!("Failed to parse return: {:?}", e);
                                        set_error.set(Some(format!("Failed to parse: {}", e)));
                                        set_loading.set(false);
                                    }
                                }
                            }
                            Err(e) => {
                                log!("Failed to read response: {:?}", e);
                                set_error.set(Some(format!("Failed to read response: {}", e)));
                                set_loading.set(false);
                            }
                        }
                    } else {
                        set_error.set(Some(format!("Server error: {}", status)));
                        set_loading.set(false);
                    }
                }
                Err(e) => {
                    log!("Failed to fetch return: {:?}", e);
                    set_error.set(Some(format!("Failed to fetch: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    });

    view! {
        <div class="detail-form">
            <div class="detail-form-header">
                <div class="detail-form-header-left">
                    <h2>"Yandex Market Return"</h2>
                </div>
                <div class="detail-form-header-right">
                    <button class="button button--secondary" on:click=move |_| on_close.run(())>
                        "✕ Закрыть"
                    </button>
                </div>
            </div>

            <div class="detail-form-content">
                {move || {
                    if loading.get() {
                        view! {
                            <div style="text-align: center; padding: var(--space-2xl);">
                                <p style="font-size: var(--font-size-sm);">"Загрузка..."</p>
                            </div>
                        }
                            .into_any()
                    } else if let Some(err) = error.get() {
                        view! {
                            <div style="padding: var(--space-lg); background: var(--color-error-bg); border: 1px solid var(--color-error-border); border-radius: var(--radius-sm); color: var(--color-error-text); margin: var(--space-lg); font-size: var(--font-size-sm);">
                                <strong>"Ошибка: "</strong>
                                {err}
                            </div>
                        }
                            .into_any()
                    } else if let Some(data) = return_data.get() {
                        view! {
                            <div>
                                <div class="detail-tabs">
                                    <button
                                        class="detail-tab"
                                        class:active=move || active_tab.get() == "general"
                                        on:click=move |_| set_active_tab.set("general")
                                    >
                                        "Общие данные"
                                    </button>
                                    <button
                                        class="detail-tab"
                                        class:active=move || active_tab.get() == "lines"
                                        on:click=move |_| set_active_tab.set("lines")
                                    >
                                        "Товары"
                                    </button>
                                    <button
                                        class="detail-tab"
                                        class:active=move || active_tab.get() == "projections"
                                        on:click=move |_| set_active_tab.set("projections")
                                    >
                                        {move || {
                                            let count = projections
                                                .get()
                                                .as_ref()
                                                .map(|p| {
                                                    p["p904_sales_data"]
                                                        .as_array()
                                                        .map(|a| a.len())
                                                        .unwrap_or(0)
                                                })
                                                .unwrap_or(0);
                                            format!("Проекции ({})", count)
                                        }}

                                    </button>
                                    <button
                                        class="detail-tab"
                                        class:active=move || active_tab.get() == "json"
                                        on:click=move |_| set_active_tab.set("json")
                                    >
                                        "Raw JSON"
                                    </button>
                                </div>

                                <div style="padding-top: var(--space-lg);">
                                    {move || {
                                        let tab = active_tab.get();
                                        match tab.as_ref() {
                                            "general" => view! { <GeneralTab data=data.clone() /> }.into_any(),
                                            "lines" => {
                                                view! {
                                                    <LinesTab
                                                        lines=data.lines.clone()
                                                        sort_column=lines_sort_column.into()
                                                        set_sort_column=set_lines_sort_column
                                                        sort_asc=lines_sort_asc.into()
                                                        set_sort_asc=set_lines_sort_asc
                                                    />
                                                }
                                                    .into_any()
                                            }
                                            "projections" => {
                                                view! {
                                                    <ProjectionsTab
                                                        projections=projections.into()
                                                        projections_loading=projections_loading.into()
                                                        sort_column=proj_sort_column.into()
                                                        set_sort_column=set_proj_sort_column
                                                        sort_asc=proj_sort_asc.into()
                                                        set_sort_asc=set_proj_sort_asc
                                                    />
                                                }
                                                    .into_any()
                                            }
                                            "json" => view! { <JsonTab raw_json=raw_json_from_ym.into() /> }.into_any(),
                                            _ => view! { <div>"Unknown tab"</div> }.into_any(),
                                        }
                                    }}

                                </div>
                            </div>
                        }
                            .into_any()
                    } else {
                        view! { <div>"No data"</div> }.into_any()
                    }
                }}

            </div>
        </div>
    }
}
