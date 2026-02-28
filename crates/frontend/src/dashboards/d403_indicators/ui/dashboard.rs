use crate::shared::api_utils::api_url;
use crate::shared::page_frame::PageFrame;
use crate::shared::components::date_range_picker::DateRangePicker;
use crate::shared::components::indicator_set::IndicatorSetView;
use chrono::{Datelike, Utc};
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::common::AggregateId;
use contracts::shared::indicators::*;
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::{HashMap, HashSet};
use thaw::*;

#[component]
pub fn IndicatorsDashboard() -> impl IntoView {
    let now = Utc::now().date_naive();
    let first_day = format!("{:04}-{:02}-01", now.year(), now.month());
    let last_day = format!("{:04}-{:02}-{:02}", now.year(), now.month(), now.day());

    let date_from = RwSignal::new(first_day);
    let date_to = RwSignal::new(last_day);
    let catalog = RwSignal::new(None::<IndicatorCatalogResponse>);
    let values = RwSignal::new(HashMap::<String, IndicatorValue>::new());
    let loading = RwSignal::new(false);
    let error_msg = RwSignal::new(None::<String>);

    let connections = RwSignal::new(Vec::<ConnectionMP>::new());
    let selected_connections: RwSignal<HashSet<String>> = RwSignal::new(HashSet::new());

    // Load connections + catalog on mount
    spawn_local({
        let catalog = catalog;
        let connections = connections;
        async move {
            // Load connections
            match Request::get(&api_url("/api/connection_mp"))
                .send()
                .await
            {
                Ok(resp) if resp.ok() => {
                    if let Ok(text) = resp.text().await {
                        if let Ok(conns) = serde_json::from_str::<Vec<ConnectionMP>>(&text) {
                            connections.set(conns);
                        }
                    }
                }
                _ => log!("Failed to load connections"),
            }

            // Load catalog
            match Request::get(&api_url("/api/indicators/meta"))
                .send()
                .await
            {
                Ok(resp) if resp.ok() => {
                    if let Ok(text) = resp.text().await {
                        if let Ok(cat) = serde_json::from_str::<IndicatorCatalogResponse>(&text) {
                            catalog.set(Some(cat));
                        }
                    }
                }
                _ => log!("Failed to load indicator catalog"),
            }
        }
    });

    // Compute indicators when filters change
    let load_indicators = move || {
        let cat = catalog.get();
        let Some(cat) = cat else { return };

        let all_ids: Vec<IndicatorId> = cat.indicators.iter().map(|m| m.id.clone()).collect();
        let df = date_from.get();
        let dt = date_to.get();
        let conn_refs: Vec<String> = selected_connections.get().into_iter().collect();

        loading.set(true);
        error_msg.set(None);

        spawn_local(async move {
            let req_body = ComputeIndicatorsRequest {
                indicator_ids: all_ids,
                context: IndicatorContext {
                    date_from: df,
                    date_to: dt,
                    organization_ref: None,
                    marketplace: None,
                    connection_mp_refs: conn_refs,
                    extra: HashMap::new(),
                },
            };

            let body_str = match serde_json::to_string(&req_body) {
                Ok(s) => s,
                Err(e) => {
                    error_msg.set(Some(format!("Serialize error: {e}")));
                    loading.set(false);
                    return;
                }
            };

            match Request::post(&api_url("/api/indicators/compute"))
                .header("Content-Type", "application/json")
                .body(body_str)
                .unwrap()
                .send()
                .await
            {
                Ok(resp) if resp.ok() => {
                    if let Ok(text) = resp.text().await {
                        match serde_json::from_str::<ComputeIndicatorsResponse>(&text) {
                            Ok(resp) => {
                                let map: HashMap<String, IndicatorValue> = resp
                                    .values
                                    .into_iter()
                                    .map(|v| (v.id.0.clone(), v))
                                    .collect();
                                values.set(map);
                            }
                            Err(e) => {
                                error_msg.set(Some(format!("Parse error: {e}")));
                            }
                        }
                    }
                }
                Ok(resp) => {
                    error_msg.set(Some(format!("HTTP {}", resp.status())));
                }
                Err(e) => {
                    error_msg.set(Some(format!("Network error: {e}")));
                }
            }
            loading.set(false);
        });
    };

    Effect::new(move |_| {
        let _ = date_from.get();
        let _ = date_to.get();
        let _ = catalog.get();
        let _ = selected_connections.get();
        load_indicators();
    });

    let on_date_change = Callback::new(move |(from, to): (String, String)| {
        date_from.set(from);
        date_to.set(to);
    });

    view! {
        <PageFrame page_id="d403_indicators--dashboard" category="dashboard">
            <div class="page__header">
                <h2 class="page__title">"Показатели"</h2>
            </div>

            <div class="indicator-dashboard__filters">
                <DateRangePicker
                    date_from=Signal::derive(move || date_from.get())
                    date_to=Signal::derive(move || date_to.get())
                    on_change=on_date_change
                />

                <div class="indicator-dashboard__cabinets">
                    <div style="font-size: var(--font-size-xs); color: var(--color-text-secondary); margin-bottom: 4px;">
                        "Кабинеты"
                    </div>
                    <CheckboxGroup value=selected_connections>
                        <div style="display: flex; flex-wrap: wrap; gap: 4px 12px;">
                            {move || connections.get().into_iter().map(|conn| {
                                let id = conn.base.id.as_string();
                                let label = conn.base.description.clone();
                                view! {
                                    <Checkbox value=id label=label />
                                }
                            }).collect_view()}
                        </div>
                    </CheckboxGroup>
                </div>
            </div>

            {move || error_msg.get().map(|msg| view! {
                <div class="alert alert--error" style="margin-bottom: var(--spacing-md);">
                    {msg}
                </div>
            })}

            {move || {
                if loading.get() && values.get().is_empty() {
                    Some(view! {
                        <div style="text-align:center; padding: var(--spacing-xl); color: var(--color-text-secondary);">
                            "Загрузка показателей..."
                        </div>
                    }.into_any())
                } else {
                    None
                }
            }}

            <div class="indicator-dashboard__sets">
                {move || {
                    let cat = catalog.get();
                    cat.map(|cat| {
                        cat.sets.into_iter().map(|set_meta| {
                            let ind_metas: Vec<IndicatorMeta> = cat.indicators.clone();
                            let vals_sig = Signal::derive(move || values.get());
                            view! {
                                <IndicatorSetView
                                    set_meta=set_meta
                                    indicator_metas=ind_metas
                                    values=vals_sig
                                />
                            }
                        }).collect_view()
                    })
                }}
            </div>
        </PageFrame>
    }
}
