//! Inline multi-select component for MP connection accounts.
//!
//! Renders marketplace connections as a compact trigger + popover checklist.

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
    let search_q = RwSignal::new(String::new());
    let open = RwSignal::new(false);
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

    let clear_all = Callback::new(move |_: ()| {
        selected.set(vec![]);
    });

    let filtered_options = Signal::derive(move || {
        let query = search_q.get().trim().to_lowercase();
        all_options.with(|options| {
            if query.is_empty() {
                options.clone()
            } else {
                options
                    .iter()
                    .filter(|option| option.label.to_lowercase().contains(&query))
                    .cloned()
                    .collect::<Vec<_>>()
            }
        })
    });

    let trigger_summary = Signal::derive(move || {
        if loading.get() {
            return "Загрузка кабинетов...".to_string();
        }

        let selected_ids = selected.get();
        if selected_ids.is_empty() {
            return "Все кабинеты".to_string();
        }

        let labels = all_options.with(|options| {
            selected_ids
                .iter()
                .filter_map(|id| {
                    options
                        .iter()
                        .find(|option| option.id == *id)
                        .map(|option| option.label.clone())
                })
                .collect::<Vec<_>>()
        });

        match labels.len() {
            0 => format!("Выбрано: {}", selected_ids.len()),
            1 => labels[0].clone(),
            2 => format!("{}, {}", labels[0], labels[1]),
            _ => format!("Выбрано: {}", selected_ids.len()),
        }
    });

    view! {
        <div class="mp-ms">
            <button
                type="button"
                class="mp-ms__trigger"
                aria-haspopup="dialog"
                aria-expanded=move || if open.get() { "true" } else { "false" }
                on:click=move |_| open.update(|current| *current = !*current)
            >
                <span
                    class="mp-ms__trigger-value"
                    class:mp-ms__trigger-value--placeholder=move || selected.with(|ids| ids.is_empty())
                >
                    {move || trigger_summary.get()}
                </span>
                <span class="mp-ms__trigger-meta">
                    <Show when=move || !selected.with(|ids| ids.is_empty())>
                        <span class="mp-ms__count">{move || selected.with(|ids| ids.len().to_string())}</span>
                    </Show>
                    <svg width="14" height="14" viewBox="0 0 20 20" fill="none" aria-hidden="true">
                        <path
                            d="M5 7.5L10 12.5L15 7.5"
                            stroke="currentColor"
                            stroke-width="1.7"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        />
                    </svg>
                </span>
            </button>

            <Show when=move || open.get()>
                <button
                    type="button"
                    class="mp-ms__backdrop"
                    aria-label="Закрыть выбор кабинетов"
                    on:click=move |_| open.set(false)
                />

                <div class="mp-ms__popover" role="dialog" aria-label="Выбор кабинетов">
                    <div class="mp-ms__search">
                        <input
                            type="text"
                            class="form__input"
                            placeholder="Поиск кабинета..."
                            prop:value=move || search_q.get()
                            on:input=move |ev: web_sys::Event| {
                                let value = ev
                                    .target()
                                    .unwrap()
                                    .unchecked_into::<web_sys::HtmlInputElement>()
                                    .value();
                                search_q.set(value);
                            }
                        />
                    </div>

                    <div class="mp-ms__summary">
                        {move || {
                            let shown = filtered_options.with(|options| options.len());
                            let total = all_options.with(|options| options.len());
                            let selected_count = selected.with(|ids| ids.len());

                            if selected_count == 0 {
                                view! {
                                    <span class="mp-ms__summary-text">
                                        {if shown == total {
                                            format!("Всего: {}", total)
                                        } else {
                                            format!("Найдено: {} из {}", shown, total)
                                        }}
                                    </span>
                                }.into_any()
                            } else {
                                view! {
                                    <>
                                        <span class="mp-ms__summary-text mp-ms__summary-text--active">
                                            {format!("Выбрано: {}", selected_count)}
                                        </span>
                                        <button
                                            type="button"
                                            class="mp-ms__clear"
                                            on:click=move |_| clear_all.run(())
                                        >
                                            "Сбросить"
                                        </button>
                                    </>
                                }.into_any()
                            }
                        }}
                    </div>

                    {move || {
                        if loading.get() {
                            view! { <div class="mp-ms__state">"Загрузка кабинетов..."</div> }.into_any()
                        } else if let Some(err) = fetch_error.get() {
                            view! { <div class="mp-ms__state mp-ms__state--error">{err}</div> }.into_any()
                        } else {
                            let options = filtered_options.get();

                            if options.is_empty() {
                                view! {
                                    <div class="mp-ms__list-empty">"Нет кабинетов по текущему запросу."</div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="mp-ms__list">
                                        <For
                                            each=move || filtered_options.get()
                                            key=|option| option.id.clone()
                                            children=move |option: MpOption| {
                                                let option_id = option.id.clone();
                                                let option_id_for_toggle = option.id.clone();
                                                let option_id_for_checked = option.id.clone();
                                                let option_label = option.label.clone();

                                                view! {
                                                    <label
                                                        class="mp-ms__row"
                                                        class:mp-ms__row--selected=move || {
                                                            selected.with(|ids| ids.contains(&option_id))
                                                        }
                                                    >
                                                        <input
                                                            type="checkbox"
                                                            class="mp-ms__checkbox"
                                                            prop:checked=move || {
                                                                selected.with(|ids| ids.contains(&option_id_for_checked))
                                                            }
                                                            on:change=move |_| {
                                                                selected.update(|ids| {
                                                                    if let Some(pos) = ids.iter().position(|id| id == &option_id_for_toggle) {
                                                                        ids.remove(pos);
                                                                    } else {
                                                                        ids.push(option_id_for_toggle.clone());
                                                                    }
                                                                });
                                                            }
                                                        />
                                                        <span class="mp-ms__row-label">{option_label}</span>
                                                    </label>
                                                }
                                            }
                                        />
                                    </div>
                                }.into_any()
                            }
                        }
                    }}

                    <div class="mp-ms__footer">
                        <button
                            type="button"
                            class="mp-ms__footer-btn"
                            on:click=move |_| clear_all.run(())
                        >
                            "Все кабинеты"
                        </button>
                        <button
                            type="button"
                            class="mp-ms__footer-btn"
                            on:click=move |_| open.set(false)
                        >
                            "Готово"
                        </button>
                    </div>
                </div>
            </Show>
        </div>
    }
}
