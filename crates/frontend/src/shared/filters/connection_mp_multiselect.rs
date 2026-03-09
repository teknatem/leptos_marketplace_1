//! Inline multi-select component for MP connection accounts.
//!
//! Renders all available marketplace connections as clickable badge-buttons.
//! This is optimized for compact filter groups inside dashboard and drilldown views.

use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::common::AggregateId;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::shared::api_utils::api_base;

#[derive(Clone, Debug)]
struct MpOption {
    id: String,
    label: String,
}

async fn load_options() -> Result<Vec<MpOption>, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/connection_mp", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    let _ = request.headers().set("Accept", "application/json");

    let window = web_sys::window().ok_or("no window")?;
    let resp_val = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_val.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let text: String =
        wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
            .await
            .map_err(|e| format!("{e:?}"))?
            .as_string()
            .ok_or("bad text")?;
    let data: Vec<ConnectionMP> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;

    let mut options: Vec<MpOption> = data
        .into_iter()
        .map(|conn| {
            let label = if conn.base.description.trim().is_empty() {
                conn.base.code.clone()
            } else {
                conn.base.description.clone()
            };
            MpOption {
                id: conn.base.id.as_string(),
                label,
            }
        })
        .collect();
    options.sort_by(|a, b| a.label.cmp(&b.label));
    Ok(options)
}

#[component]
#[allow(non_snake_case)]
pub fn ConnectionMpMultiSelect(selected: RwSignal<Vec<String>>) -> impl IntoView {
    let all_options = RwSignal::new(Vec::<MpOption>::new());
    let loading = RwSignal::new(false);
    let fetch_error = RwSignal::new(None::<String>);
    let requested = RwSignal::new(false);

    Effect::new(move |_| {
        if requested.get() {
            return;
        }
        requested.set(true);
        leptos::task::spawn_local(async move {
            loading.set(true);
            fetch_error.set(None);
            match load_options().await {
                Ok(opts) => all_options.set(opts),
                Err(err) => fetch_error.set(Some(err)),
            }
            loading.set(false);
        });
    });

    let clear_all = move |_| {
        selected.set(vec![]);
    };

    view! {
        <div class="mp-ms">

            {move || {
                if loading.get() {
                    view! { <div class="mp-ms__state">"Загрузка кабинетов..."</div> }.into_any()
                } else if let Some(err) = fetch_error.get() {
                    view! { <div class="mp-ms__state mp-ms__state--error">{err}</div> }.into_any()
                } else if all_options.with(|opts| opts.is_empty()) {
                    view! { <div class="mp-ms__state">"Нет доступных кабинетов."</div> }.into_any()
                } else {
                    view! {
                        <div class="mp-ms__badges">
                            <button
                                type="button"
                                class=move || {
                                    if selected.with(|ids| ids.is_empty()) {
                                        "mp-ms__badge mp-ms__badge--selected mp-ms__badge--all"
                                    } else {
                                        "mp-ms__badge mp-ms__badge--all"
                                    }
                                }
                                on:click=clear_all
                            >
                                "Все"
                            </button>

                            {move || {
                                all_options.get().into_iter().map(|opt| {
                                    let option_id = opt.id.clone();
                                    let option_id_for_check = option_id.clone();
                                    let option_id_for_click = option_id.clone();
                                    let label = opt.label.clone();
                                    view! {
                                        <button
                                            type="button"
                                            class=move || {
                                                if selected.with(|ids| ids.contains(&option_id_for_check)) {
                                                    "mp-ms__badge mp-ms__badge--selected"
                                                } else {
                                                    "mp-ms__badge"
                                                }
                                            }
                                            on:click=move |_| {
                                                selected.update(|ids| {
                                                    if let Some(pos) = ids.iter().position(|id| id == &option_id_for_click) {
                                                        ids.remove(pos);
                                                    } else {
                                                        ids.push(option_id_for_click.clone());
                                                    }
                                                });
                                            }
                                        >
                                            {label}
                                        </button>
                                    }
                                }).collect_view()
                            }}
                        </div>
                    }.into_any()
                }
            }}
            <div class="mp-ms__toolbar">
                <span class="mp-ms__summary">
                    {move || {
                        let count = selected.with(|ids| ids.len());
                        if count == 0 {
                            "Все кабинеты".to_string()
                        } else {
                            format!("Выбрано: {}", count)
                        }
                    }}
                </span>

                <Show when=move || !selected.with(|ids| ids.is_empty())>
                    <button
                        type="button"
                        class="mp-ms__clear"
                        on:click=clear_all
                    >
                        "Сбросить"
                    </button>
                </Show>
            </div>

        </div>
    }
}
