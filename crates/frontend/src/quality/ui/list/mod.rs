//! Реестр проверок контроля качества данных.
//!
//! Таблица всех зарегистрированных правил с кнопками «Запустить» (быстрый статус
//! inline) и «Детали» (открывает страницу `quality_check_details_<id>` с метриками,
//! долей соответствия, разрезами и drill-down).

use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use contracts::quality::{
    CheckResult, QualityCheckInfo, QualityCheckOverview, QualityCheckReloadReport,
};
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

    let (checks, set_checks) = signal::<Vec<QualityCheckOverview>>(Vec::new());
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
            let url = format!("{}/api/quality/checks/overview", api_base());
            match Request::get(&url).send().await {
                Ok(resp) if resp.status() == 200 => {
                    match resp.json::<Vec<QualityCheckOverview>>().await {
                        Ok(data) => {
                            set_checks.set(data);
                            set_loading.set(false);
                        }
                        Err(e) => {
                            set_load_error.set(Some(format!("Ошибка разбора: {e}")));
                            set_loading.set(false);
                        }
                    }
                }
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

    let reload_checks = move || {
        set_loading.set(true);
        set_load_error.set(None);
        spawn_local(async move {
            let url = format!("{}/api/quality/checks/reload", api_base());
            match Request::post(&url).send().await {
                Ok(resp) if resp.status() == 200 => {
                    match resp.json::<QualityCheckReloadReport>().await {
                        Ok(report) if report.ok => fetch_checks(),
                        Ok(report) => {
                            set_load_error.set(Some(format!(
                                "Reload отклонён: {}",
                                report.diagnostics.join("; ")
                            )));
                            set_loading.set(false);
                        }
                        Err(error) => {
                            set_load_error.set(Some(format!("Ошибка разбора reload: {error}")));
                            set_loading.set(false);
                        }
                    }
                }
                Ok(resp) => {
                    set_load_error.set(Some(format!("Reload: HTTP {}", resp.status())));
                    set_loading.set(false);
                }
                Err(error) => {
                    set_load_error.set(Some(format!("Reload: {error}")));
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
                Ok(r) if r.status() == 200 => {
                    r.json::<CheckResult>().await.map_err(|e| e.to_string())
                }
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
                        on_click=move |_| reload_checks()
                        disabled=loading.get()
                    >
                        {icon("refresh")} " Перезагрузить правила"
                    </thaw::Button>
                </div>
            </div>

            {move || load_error.get().map(|e| view! {
                <div class="warning-box" style="margin: 10px;">{e}</div>
            })}

            <div style="margin: 10px 12px 0; padding: 12px 14px; border: 1px solid var(--color-border); border-radius: 8px; background: var(--color-surface); color: var(--color-text-secondary); font-size: 0.875rem; line-height: 1.45;">
                "Каждая карточка — отдельный read-only инвариант качества данных. Последний результат относится к текущей версии правила. Откройте название, чтобы увидеть популяцию, нарушения, разрезы и примеры; запуск обновляет историю."
            </div>

            {move || if loading.get() {
                view! { <div style="padding: 20px; color: var(--color-text-secondary);">"Загрузка..."</div> }.into_any()
            } else if checks.get().is_empty() {
                view! { <div style="padding: 20px; color: var(--color-text-secondary);">"Нет зарегистрированных проверок."</div> }.into_any()
            } else {
                view! {
                    <div class="page__content">
                        <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(360px, 1fr)); gap: 12px; padding: 12px; align-items: stretch;">
                            {move || {
                                let needle = search.get().trim().to_lowercase();
                                let filtered = checks
                                    .get()
                                    .into_iter()
                                    .filter(|check| quality_check_matches(&check.info, &needle))
                                    .collect::<Vec<_>>();

                                if filtered.is_empty() {
                                    return view! {
                                        <div style="grid-column: 1 / -1; padding: 20px; text-align: center; color: var(--color-text-secondary);">
                                            "Ничего не найдено"
                                        </div>
                                    }.into_any();
                                }

                                filtered.into_iter().map(|overview| {
                                    let check = overview.info;
                                    let cid = check.id.clone();
                                    let cid_run = cid.clone();
                                    let cid_detail = cid.clone();
                                    let detail_code = check.code.clone();
                                    let detail_name = check.name.clone();
                                    let latest = overview.latest_run.clone();
                                    let kind = if overview.kind == "regular" { "Регулярная" } else { "Доменная" };
                                    let store = tabs_store;
                                    view! {
                                        <article style="display: flex; flex-direction: column; min-height: 220px; border: 1px solid var(--color-border); border-radius: 10px; background: var(--color-surface); padding: 16px;">
                                            <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 10px;">
                                                <span style="font-family: monospace; font-size: 0.78rem; color: var(--color-text-secondary);">{check.code.clone()}</span>
                                                <span class="badge badge--secondary">{check.category.clone()}</span>
                                                <span style="font-size: 0.75rem; color: var(--color-text-tertiary); margin-left: auto;">{kind}</span>
                                            </div>
                                            <h2 style="font-size: 1rem; line-height: 1.35; margin: 0 0 8px;">
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
                                            </h2>
                                            <p style="color: var(--color-text-secondary); font-size: 0.875rem; line-height: 1.45; margin: 0 0 14px; flex: 1;">{check.description.clone()}</p>
                                            <div style="border-top: 1px solid var(--color-border); padding-top: 10px; display: flex; align-items: center; gap: 8px;">
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
                                                    } else if let Some(run) = &latest {
                                                        let when = run.started_at.format("%d.%m.%Y %H:%M").to_string();
                                                        if run.status == "completed" {
                                                            let violations = run.violations_total.unwrap_or(0);
                                                            let population = run.population_total.unwrap_or(0);
                                                            let badge = if violations == 0 { "badge badge--success" } else { "badge badge--error" };
                                                            let title = format!("Последний запуск: {when}");
                                                            view! {
                                                                <div title=title style="display: flex; gap: 6px; align-items: center; font-size: 0.78rem;">
                                                                    <span class=badge>{if violations == 0 { "✓ Нет нарушений".to_string() } else { format!("⚠ {violations} из {population}") }}</span>
                                                                    <span style="color: var(--color-text-tertiary);">{when}</span>
                                                                </div>
                                                            }.into_any()
                                                        } else if run.status == "failed" {
                                                            view! { <span class="badge badge--error" title=run.error.clone().unwrap_or_default()>"Ошибка последнего запуска"</span> }.into_any()
                                                        } else {
                                                            view! { <span style="color: var(--color-text-secondary); font-size: 0.8rem;">"⏳ Выполняется"</span> }.into_any()
                                                        }
                                                    } else {
                                                        view! { <span style="color: var(--color-text-tertiary); font-size: 0.8rem;">"Ещё не запускалась"</span> }.into_any()
                                                    }
                                                }}
                                                <div style="margin-left: auto;">
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
                                            </div>
                                        </article>
                                    }
                                }).collect_view().into_any()
                            }}
                        </div>
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
