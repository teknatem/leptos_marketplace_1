//! Multi-select component for MP connection accounts.
//!
//! Uses an `OverlayDrawer` (Thaw) to avoid all positioning / overflow issues.
//! Pattern is identical to `IndicatorPicker` in `a025_bi_dashboard`.
//!
//! # Usage
//! ```rust
//! let selected_ids = RwSignal::new(Vec::<String>::new());
//! view! { <ConnectionMpMultiSelect selected=selected_ids /> }
//! ```

use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::common::AggregateId;
use leptos::prelude::*;
use thaw::*;
use wasm_bindgen::JsCast;

use crate::shared::api_utils::api_base;

// ── internal data ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct MpOption {
    id: String,
    /// Display name shown in the picker (description; falls back to code).
    label: String,
}

async fn load_options() -> Result<Vec<MpOption>, String> {
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/connection_mp", api_base());
    let request =
        Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    let _ = request.headers().set("Accept", "application/json");

    let window = web_sys::window().ok_or("no window")?;
    let resp_val =
        wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
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
    let data: Vec<ConnectionMP> =
        serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data
        .into_iter()
        .map(|c| {
            let label = if c.base.description.trim().is_empty() {
                c.base.code.clone()
            } else {
                c.base.description.clone()
            };
            MpOption {
                id: c.base.id.as_string(),
                label,
            }
        })
        .collect())
}

// ── main component ────────────────────────────────────────────────────────────

/// Multi-select picker for Marketplace connections.
///
/// Shows selected connections as compact tags in an inline header button.
/// Opens an [`OverlayDrawer`] with search + checkbox list when clicked.
#[component]
#[allow(non_snake_case)]
pub fn ConnectionMpMultiSelect(
    /// RwSignal holding the currently-selected connection IDs.
    selected: RwSignal<Vec<String>>,
) -> impl IntoView {
    // Label cache: id → label, populated when the drawer opens.
    let all_options: RwSignal<Vec<MpOption>> = RwSignal::new(vec![]);
    let drawer_open: RwSignal<bool> = RwSignal::new(false);

    // Resolve labels for the header tags from the cached options.
    let label_for = move |id: &str| -> String {
        all_options.with(|opts| {
            opts.iter()
                .find(|o| o.id == id)
                .map(|o| o.label.clone())
                .unwrap_or_else(|| id.chars().take(8).collect())
        })
    };

    view! {
        // ── header button ─────────────────────────────────────────────────────
        <div
            class="mp-ms__header"
            on:click=move |_| drawer_open.set(true)
        >
            {move || {
                let sel = selected.get();
                if sel.is_empty() {
                    view! {
                        <span class="mp-ms__placeholder">"— все кабинеты —"</span>
                    }.into_any()
                } else {
                    let count = sel.len();
                    let tags = sel.iter().take(3).map(|id| {
                        let lbl = label_for(id);
                        view! { <span class="mp-ms__tag">{lbl}</span> }
                    }).collect_view();
                    view! {
                        <div class="mp-ms__tags">
                            {tags}
                            {(count > 3).then(|| view! {
                                <span class="mp-ms__tag mp-ms__tag--more">
                                    "+" {count - 3}
                                </span>
                            })}
                        </div>
                    }.into_any()
                }
            }}
            <span class="mp-ms__chevron">"▼"</span>
        </div>

        // ── Drawer ────────────────────────────────────────────────────────────
        <OverlayDrawer
            open=drawer_open
            position=DrawerPosition::Right
            size=DrawerSize::Small
            close_on_esc=true
        >
            <DrawerHeader>
                <DrawerHeaderTitle>
                    "Выбор кабинетов МП"
                </DrawerHeaderTitle>
            </DrawerHeader>
            <DrawerBody>
                <MpPicker
                    all_options=all_options
                    selected=selected
                    drawer_open=drawer_open
                />
            </DrawerBody>
        </OverlayDrawer>
    }
}

// ── MpPicker (inner drawer content) ──────────────────────────────────────────

#[component]
fn MpPicker(
    all_options: RwSignal<Vec<MpOption>>,
    selected: RwSignal<Vec<String>>,
    drawer_open: RwSignal<bool>,
) -> impl IntoView {
    let search_q: RwSignal<String> = RwSignal::new(String::new());
    let loading: RwSignal<bool> = RwSignal::new(false);
    let fetch_error: RwSignal<Option<String>> = RwSignal::new(None);

    // Pending selection — local copy while drawer is open; applied on confirm.
    let pending: RwSignal<Vec<String>> = RwSignal::new(vec![]);

    // Fetch options once on first open; reset pending + search on each open.
    Effect::new(move |was_open: Option<bool>| {
        let open = drawer_open.get();
        if open && was_open != Some(true) {
            // Reset UI state
            search_q.set(String::new());
            // Pre-populate pending from current selection
            pending.set(selected.get_untracked());

            if all_options.with_untracked(|o| o.is_empty()) {
                leptos::task::spawn_local(async move {
                    loading.set(true);
                    fetch_error.set(None);
                    match load_options().await {
                        Ok(opts) => all_options.set(opts),
                        Err(e) => fetch_error.set(Some(e)),
                    }
                    loading.set(false);
                });
            }
        }
        open
    });

    // Client-side filter — instant, no API call
    let filtered = Signal::derive(move || {
        let q = search_q.get().trim().to_lowercase();
        all_options.with(|opts| {
            if q.is_empty() {
                opts.clone()
            } else {
                opts.iter()
                    .filter(|o| o.label.to_lowercase().contains(&q))
                    .cloned()
                    .collect()
            }
        })
    });

    let on_confirm = move |_| {
        selected.set(pending.get_untracked());
        drawer_open.set(false);
    };

    let on_cancel = move |_| {
        drawer_open.set(false);
    };

    view! {
        <div class="mp-picker">
            // ── Search ────────────────────────────────────────────────────────
            <div class="mp-picker__search">
                <input
                    type="text"
                    class="form__input"
                    placeholder="Поиск по наименованию…"
                    prop:value=move || search_q.get()
                    on:input=move |ev: web_sys::Event| {
                        let val = ev.target().unwrap()
                            .unchecked_into::<web_sys::HtmlInputElement>()
                            .value();
                        search_q.set(val);
                    }
                />
            </div>

            // ── Summary bar ───────────────────────────────────────────────────
            <div class="mp-picker__summary">
                {move || {
                    let n     = pending.with(|v| v.len());
                    let shown = filtered.with(|v| v.len());
                    let total = all_options.with(|v| v.len());
                    if n == 0 {
                        view! {
                            <span class="mp-picker__summary-text">
                                {if shown == total {
                                    format!("Всего: {}", total)
                                } else {
                                    format!("Найдено: {} из {}", shown, total)
                                }}
                            </span>
                        }.into_any()
                    } else {
                        view! {
                            <span class="mp-picker__summary-text mp-picker__summary-text--active">
                                "Выбрано: " {n}
                            </span>
                            <button
                                class="mp-picker__clear-btn"
                                on:click=move |_| pending.set(vec![])
                            >
                                "Снять всё"
                            </button>
                        }.into_any()
                    }
                }}
            </div>

            // ── Loading / error ───────────────────────────────────────────────
            {move || loading.get().then(|| view! {
                <div class="mp-picker__loading">"Загрузка…"</div>
            })}
            {move || fetch_error.get().map(|e| view! {
                <div class="mp-picker__error">{e}</div>
            })}

            // ── List ──────────────────────────────────────────────────────────
            <div class="mp-picker__list">
                {move || {
                    let rows = filtered.get();
                    let sel  = pending.get();

                    if rows.is_empty() && !loading.get_untracked() {
                        return view! {
                            <div class="mp-picker__list-empty">"Нет кабинетов по запросу"</div>
                        }.into_any();
                    }

                    rows.into_iter().map(|opt| {
                        let is_sel  = sel.contains(&opt.id);
                        let opt_id  = opt.id.clone();
                        let row_cls = if is_sel {
                            "mp-picker__row mp-picker__row--selected"
                        } else {
                            "mp-picker__row"
                        };
                        view! {
                            <label class=row_cls>
                                <input
                                    type="checkbox"
                                    class="mp-picker__checkbox"
                                    prop:checked=is_sel
                                    on:change=move |_| {
                                        pending.update(|v| {
                                            if let Some(pos) = v.iter().position(|x| x == &opt_id) {
                                                v.remove(pos);
                                            } else {
                                                v.push(opt_id.clone());
                                            }
                                        });
                                    }
                                />
                                <span class="mp-picker__label">{opt.label.clone()}</span>
                            </label>
                        }
                    }).collect::<Vec<_>>().into_any()
                }}
            </div>

            // ── Footer ────────────────────────────────────────────────────────
            <div class="mp-picker__footer">
                <Button
                    appearance=ButtonAppearance::Primary
                    on_click=on_confirm
                >
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none"
                        stroke="currentColor" stroke-width="2.5"
                        stroke-linecap="round" stroke-linejoin="round">
                        <polyline points="20 6 9 17 4 12"/>
                    </svg>
                    " Применить"
                </Button>
                <Button appearance=ButtonAppearance::Secondary on_click=on_cancel>
                    "Отмена"
                </Button>
            </div>
        </div>
    }
}
