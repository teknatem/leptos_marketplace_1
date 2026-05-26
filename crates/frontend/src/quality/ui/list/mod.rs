//! Реестр проверок контроля качества данных.
//!
//! Таблица всех зарегистрированных правил с кнопками «Запустить» (быстрый статус
//! inline) и «Детали» (открывает страницу `quality_check_details_<id>` с метриками,
//! долей соответствия, разрезами и drill-down).

use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use contracts::quality::{CheckResult, QualityCheckInfo};
use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct CheckState {
    running: bool,
    result: Option<CheckResult>,
    error: Option<String>,
}

fn quality_check_matches(check: &QualityCheckInfo, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let haystack = format!(
        "{} {} {} {} {}",
        check.code, check.id, check.name, check.category, check.description
    )
    .to_lowercase();
    haystack.contains(needle)
}

// ---------------------------------------------------------------------------
// QualityCheckList — main page
// ---------------------------------------------------------------------------

#[component]
#[allow(non_snake_case)]
pub fn QualityCheckList() -> impl IntoView {
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext context not found");

    let (checks, set_checks) = signal::<Vec<QualityCheckInfo>>(Vec::new());
    let (states, set_states) =
        signal::<std::collections::HashMap<String, CheckState>>(Default::default());
    let (loading, set_loading) = signal(false);
    let (load_error, set_load_error) = signal::<Option<String>>(None);
    let search = RwSignal::new(String::new());

    // --- fetch check list ---
    let fetch_checks = move || {
        set_loading.set(true);
        set_load_error.set(None);
        spawn_local(async move {
            let url = format!("{}/api/quality/checks", api_base());
            match Request::get(&url).send().await {
                Ok(resp) if resp.status() == 200 => match resp.json::<Vec<QualityCheckInfo>>().await
                {
                    Ok(data) => {
                        set_checks.set(data);
                        set_loading.set(false);
                    }
                    Err(e) => {
                        set_load_error.set(Some(format!("Ошибка разбора: {e}")));
                        set_loading.set(false);
                    }
                },
                Ok(resp) => {
                    set_load_error.set(Some(format!("HTTP {}", resp.status())));
                    set_loading.set(false);
                }
                Err(e) => {
                    log!("quality fetch error: {e:?}");
                    set_load_error.set(Some(format!("Ошибка запроса: {e}")));
                    set_loading.set(false);
                }
            }
        });
    };

    Effect::new(move |_| {
        fetch_checks();
    });

    // --- run a check (inline status badge) ---
    let run_check = move |check_id: String| {
        set_states.update(|m| {
            m.entry(check_id.clone()).or_default().running = true;
        });
        spawn_local(async move {
            let url = format!("{}/api/quality/checks/{}/run", api_base(), check_id);
            let res: Result<CheckResult, String> = match Request::post(&url).send().await {
                Ok(r) if r.status() == 200 => r.json::<CheckResult>().await.map_err(|e| e.to_string()),
                Ok(r) => Err(format!("HTTP {}", r.status())),
                Err(e) => Err(e.to_string()),
            };
            set_states.update(|m| {
                let s = m.entry(check_id.clone()).or_default();
                s.running = false;
                match res {
                    Ok(r) => {
                        s.result = Some(r);
                        s.error = None;
                    }
                    Err(e) => {
                        s.error = Some(e);
                    }
                }
            });
        });
    };

    view! {
        <PageFrame page_id="quality_checks--list" category="list">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Контроль качества данных"</h1>
                </div>
                <div class="navigator__search" style="max-width: 360px; flex: 1 1 260px;">
                    <span class="navigator__search-icon">{icon("search")}</span>
                    <input
                        class="navigator__search-input"
                        type="search"
                        placeholder="Поиск по проверкам..."
                        prop:value=move || search.get()
                        on:input=move |ev| search.set(event_target_value(&ev))
                    />
                    <Show when=move || !search.get().is_empty()>
                        <button
                            class="navigator__search-clear"
                            type="button"
                            title="Очистить"
                            on:click=move |_| search.set(String::new())
                        >
                            {icon("x")}
                        </button>
                    </Show>
                </div>
                <div class="page__header-right">
                    <thaw::Button
                        appearance=thaw::ButtonAppearance::Secondary
                        on_click=move |_| fetch_checks()
                        disabled=loading.get()
                    >
                        {icon("refresh")} " Обновить список"
                    </thaw::Button>
                </div>
            </div>

            {move || load_error.get().map(|e| view! {
                <div class="warning-box" style="margin: 10px;">{e}</div>
            })}

            {move || if loading.get() {
                view! { <div style="padding: 20px; color: var(--color-text-secondary);">"Загрузка..."</div> }.into_any()
            } else if checks.get().is_empty() {
                view! { <div style="padding: 20px; color: var(--color-text-secondary);">"Нет зарегистрированных проверок."</div> }.into_any()
            } else {
                view! {
                    <div class="page__content">
                        <table class="table__data table--striped">
                            <thead class="table__head">
                                <tr>
                                    <th class="table__header-cell" style="width: 90px;">"Код"</th>
                                    <th class="table__header-cell">"Название"</th>
                                    <th class="table__header-cell">"Категория"</th>
                                    <th class="table__header-cell">"Описание"</th>
                                    <th class="table__header-cell" style="width: 160px;">"Статус"</th>
                                    <th class="table__header-cell table__header-cell--center" style="width: 200px;">"Действие"</th>
                                </tr>
                            </thead>
                            <tbody>
                            {move || {
                                let needle = search.get().trim().to_lowercase();
                                let filtered = checks
                                    .get()
                                    .into_iter()
                                    .filter(|check| quality_check_matches(check, &needle))
                                    .collect::<Vec<_>>();

                                if filtered.is_empty() {
                                    return view! {
                                        <tr class="table__row">
                                            <td class="table__cell" colspan="6" style="padding: 20px; text-align: center; color: var(--color-text-secondary);">
                                                "Ничего не найдено"
                                            </td>
                                        </tr>
                                    }.into_any();
                                }

                                filtered.into_iter().map(|check| {
                                    let cid = check.id.clone();
                                    let cid_run = cid.clone();
                                    let cid_detail = cid.clone();
                                    let detail_code = check.code.clone();
                                    let detail_name = check.name.clone();
                                    let store = tabs_store;
                                    view! {
                                        <tr class="table__row">
                                            <td class="table__cell" style="font-family: monospace; font-size: 0.82rem; color: var(--color-text-secondary); white-space: nowrap;">{check.code.clone()}</td>
                                            <td class="table__cell" style="font-weight: 500;">
                                                <a
                                                    href="#"
                                                    class="table__link"
                                                    on:click=move |e| {
                                                        e.prevent_default();
                                                        store.open_tab(
                                                            &format!("quality_check_details_{cid_detail}"),
                                                            &format!("{} · {}", detail_code, detail_name),
                                                        );
                                                    }
                                                >
                                                    {check.name.clone()}
                                                </a>
                                            </td>
                                            <td class="table__cell"><span class="badge badge--secondary">{check.category.clone()}</span></td>
                                            <td class="table__cell" style="color: var(--color-text-secondary); font-size: 0.875rem;">{check.description.clone()}</td>
                                            <td class="table__cell">
                                                {move || {
                                                    let map = states.get();
                                                    let s = map.get(&cid).cloned().unwrap_or_default();
                                                    if s.running {
                                                        view! { <span style="color: var(--color-text-secondary); font-size: 0.8rem;">"⏳ Выполняется"</span> }.into_any()
                                                    } else if s.error.is_some() {
                                                        view! { <span class="badge badge--error" style="font-size: 0.75rem;">"Ошибка"</span> }.into_any()
                                                    } else if let Some(r) = &s.result {
                                                        let compliant = (r.population_total - r.violations_total).max(0);
                                                        let rate_title = format!("Соответствие {}", fmt_pct(r.compliance_rate()));
                                                        view! {
                                                            <div style="display: flex; gap: 4px; align-items: center;">
                                                                <span class="badge badge--success" style="font-size: 0.75rem;" title=rate_title>
                                                                    {format!("✓ {}", compliant)}
                                                                </span>
                                                                <span class="badge badge--error" style="font-size: 0.75rem;">
                                                                    {format!("⚠ {}", r.violations_total)}
                                                                </span>
                                                            </div>
                                                        }.into_any()
                                                    } else {
                                                        view! { <span style="color: var(--color-text-tertiary); font-size: 0.8rem;">"—"</span> }.into_any()
                                                    }
                                                }}
                                            </td>
                                            <td class="table__cell table__cell--center">
                                                <div style="display: flex; gap: 4px; justify-content: center;">
                                                    {move || {
                                                        let map = states.get();
                                                        let running = map.get(&cid_run).map(|s| s.running).unwrap_or(false);
                                                        let id = cid_run.clone();
                                                        view! {
                                                            <thaw::Button
                                                                appearance=thaw::ButtonAppearance::Secondary
                                                                size=thaw::ButtonSize::Small
                                                                disabled=running
                                                                on_click=move |_| run_check(id.clone())
                                                            >
                                                                {icon("play")} " Запустить"
                                                            </thaw::Button>
                                                        }
                                                    }}
                                                </div>
                                            </td>
                                        </tr>
                                    }
                                }).collect_view().into_any()
                            }}
                            </tbody>
                        </table>
                    </div>
                }.into_any()
            }}
        </PageFrame>
    }
}

/// Доля соответствия в процентах для бейджа статуса.
fn fmt_pct(rate: f64) -> String {
    format!("{:.2}%", rate * 100.0)
}
