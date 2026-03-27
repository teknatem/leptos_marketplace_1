//! ConnectionMpMultiSelect — выбор кабинетов МП через CheckboxGroup Thaw.

use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::common::AggregateId;
use gloo_net::http::Request;
use leptos::prelude::*;
use std::collections::HashSet;
use thaw::*;

use crate::shared::api_utils::api_base;

#[derive(Clone, Debug)]
struct MpOption {
    id: String,
    label: String,
}

async fn load_options() -> Result<Vec<MpOption>, String> {
    let url = format!("{}/api/connection_mp", api_base());
    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("{e}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let data: Vec<ConnectionMP> = resp.json().await.map_err(|e| format!("{e}"))?;

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

    let selected_set: RwSignal<HashSet<String>> =
        RwSignal::new(selected.get_untracked().into_iter().collect());

    // Sync HashSet → Vec (sorted for stable ordering)
    Effect::new(move |_| {
        let mut vec: Vec<String> = selected_set.get().into_iter().collect();
        vec.sort();
        selected.set(vec);
    });

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

    view! {
        {move || {
            if loading.get() {
                view! { <span class="form__label">"Загрузка кабинетов..."</span> }.into_any()
            } else if let Some(err) = fetch_error.get() {
                view! { <span style="color:var(--color-danger,#e53e3e)">{err}</span> }.into_any()
            } else if all_options.with(|opts| opts.is_empty()) {
                view! { <span class="form__label">"Нет доступных кабинетов"</span> }.into_any()
            } else {
                view! {
                    <CheckboxGroup value=selected_set>
                        <div style="display:flex;flex-wrap:wrap;gap:6px 12px">
                            <For
                                each=move || all_options.get()
                                key=|opt| opt.id.clone()
                                children=|opt: MpOption| view! {
                                    <Checkbox value=opt.id label=opt.label />
                                }
                            />
                        </div>
                    </CheckboxGroup>
                }.into_any()
            }
        }}
    }
}
