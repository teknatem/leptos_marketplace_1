pub mod api;

use contracts::system::audit::{AuditReport, RoutePolicyDto, ViolationType};
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::{Badge, BadgeColor, Button, ButtonAppearance, Tab, TabList};

use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_SYSTEM;
use crate::system::auth::guard::RequireAdmin;

use self::api as audit_api;

#[component]
pub fn AuditPage() -> impl IntoView {
    view! {
        <RequireAdmin>
            <AuditPageInner />
        </RequireAdmin>
    }
}

#[component]
fn AuditPageInner() -> impl IntoView {
    let routes: RwSignal<Vec<RoutePolicyDto>> = RwSignal::new(Vec::new());
    let report: RwSignal<Option<AuditReport>> = RwSignal::new(None);
    let (error, set_error) = signal::<Option<String>>(None);
    let (loading, set_loading) = signal(true);
    let (filter_violations, set_filter_violations) = signal(false);
    let (route_search, set_route_search) = signal(String::new());
    let selected_tab: RwSignal<String> = RwSignal::new("health".to_string());

    let load_data = move || {
        set_loading.set(true);
        set_error.set(None);

        spawn_local(async move {
            let routes_res = audit_api::fetch_routes().await;
            let violations_res = audit_api::fetch_violations().await;

            match (routes_res, violations_res) {
                (Ok(r), Ok(v)) => {
                    routes.set(r);
                    report.set(Some(v));
                    set_loading.set(false);
                }
                (Err(e), _) | (_, Err(e)) => {
                    set_error.set(Some(format!("Ошибка загрузки аудита: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    Effect::new(move |_| {
        load_data();
    });

    view! {
        <PageFrame page_id="sys_audit" category=PAGE_CAT_SYSTEM>
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Аудит доступа"</h1>
                    {move || {
                        report.get().map(|r| {
                            let v = r.violations.len();
                            view! {
                                <Badge color={if v > 0 { BadgeColor::Danger } else { BadgeColor::Success }}>
                                    {if v > 0 { format!("{} нарушений", v) } else { "Нарушений нет".to_string() }}
                                </Badge>
                            }
                        })
                    }}
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| load_data()
                        disabled=Signal::derive(move || loading.get())
                    >
                        {icon("refresh")}
                        " Обновить"
                    </Button>
                </div>
            </div>

            <div class="page__content">
                {move || error.get().map(|e| view! { <div class="alert alert--error">{e}</div> })}

                {move || if loading.get() {
                    view! { <div class="page__loading">"Загрузка данных аудита..."</div> }.into_any()
                } else {
                    view! {
                        <TabList selected_value=selected_tab>
                            <Tab value="health".to_string()>"Состояние"</Tab>
                            <Tab value="routes".to_string()>"Маршруты"</Tab>
                            <Tab value="violations".to_string()>"Нарушения"</Tab>
                        </TabList>

                        <div style="margin-top:16px;">
                            {move || match selected_tab.get().as_str() {
                                "health" => report.get().map(|r| view! { <HealthTab report=r /> }.into_any())
                                    .unwrap_or_else(|| view! { <div>"Загрузка..."</div> }.into_any()),
                                "routes" => view! {
                                    <RoutesTab
                                        routes=routes
                                        filter_violations=filter_violations
                                        set_filter_violations=set_filter_violations
                                        route_search=route_search
                                        set_route_search=set_route_search
                                    />
                                }.into_any(),
                                _ => report.get().map(|r| view! { <ViolationsTab report=r /> }.into_any())
                                    .unwrap_or_else(|| view! { <div>"Загрузка..."</div> }.into_any()),
                            }}
                        </div>
                    }.into_any()
                }}
            </div>
        </PageFrame>
    }
}

// ============================================================================
// Health tab
// ============================================================================

#[component]
fn HealthTab(report: AuditReport) -> impl IntoView {
    let violation_count = report.violations.len();
    let health_ok = violation_count == 0;

    view! {
        <div style="display:flex;flex-direction:column;gap:16px;max-width:800px;">

            // Summary cards
            <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(160px,1fr));gap:12px;">
                <StatCard label="Всего маршрутов" value=report.total_routes.to_string() color="default" />
                <StatCard label="С проверкой scope" value=report.scoped_routes.to_string() color="info" />
                <StatCard label="Публичных" value=report.open_routes.to_string() color="success" />
                <StatCard label="Нарушений" value=violation_count.to_string()
                    color=if health_ok { "success" } else { "error" } />
            </div>

            // Overall health
            <div style=format!("padding:12px 16px;border-radius:8px;border-left:4px solid {};background:var(--color-surface-alt,var(--color-surface));",
                if health_ok { "var(--color-success,#16a34a)" } else { "var(--color-error,#dc2626)" })>
                <strong>
                    {if health_ok { "✓ Система прошла аудит без нарушений" } else { "✗ Обнаружены нарушения политики доступа" }}
                </strong>
                {if !health_ok { view! {
                    <div style="margin-top:4px;font-size:0.85em;color:var(--color-text-secondary);">
                        {format!("Перейдите на вкладку «Нарушения» для детальной информации.")}
                    </div>
                }.into_any() } else { view! {<></>}.into_any() }}
            </div>

            // Coverage per role
            <div>
                <h3 style="font-size:0.9em;font-weight:600;margin-bottom:8px;color:var(--color-text-secondary);">"Покрытие scope по ролям"</h3>
                <div style="display:flex;flex-direction:column;gap:6px;">
                    {report.role_coverage.iter().map(|cs| {
                        let pct = if cs.total > 0 { cs.covered * 100 / cs.total } else { 0 };
                        let color = if pct == 100 { "#16a34a" } else if pct >= 50 { "#d97706" } else { "#dc2626" };
                        view! {
                            <div style="display:flex;align-items:center;gap:12px;">
                                <span style="font-family:monospace;font-size:0.85em;min-width:80px;">{cs.role_code.clone()}</span>
                                <div style="flex:1;background:var(--color-border);border-radius:4px;height:8px;overflow:hidden;">
                                    <div style=format!("width:{}%;height:100%;background:{};transition:width 0.3s;", pct, color)></div>
                                </div>
                                <span style="font-size:0.82em;color:var(--color-text-secondary);">
                                    {format!("{}/{} ({}%)", cs.covered, cs.total, pct)}
                                </span>
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            </div>
        </div>
    }
}

#[component]
fn StatCard(label: &'static str, value: String, color: &'static str) -> impl IntoView {
    let border_color = match color {
        "info" => "var(--color-info-500,#0ea5e9)",
        "success" => "var(--color-success,#16a34a)",
        "error" => "var(--color-error,#dc2626)",
        "warning" => "var(--color-warning,#d97706)",
        _ => "var(--color-border)",
    };
    view! {
        <div style=format!("padding:12px 16px;border-radius:8px;border:1px solid {};background:var(--color-surface);text-align:center;", border_color)>
            <div style="font-size:1.6em;font-weight:700;">{value}</div>
            <div style="font-size:0.8em;color:var(--color-text-secondary);margin-top:2px;">{label}</div>
        </div>
    }
}

// ============================================================================
// Routes tab
// ============================================================================

#[component]
fn RoutesTab(
    routes: RwSignal<Vec<RoutePolicyDto>>,
    filter_violations: ReadSignal<bool>,
    set_filter_violations: WriteSignal<bool>,
    route_search: ReadSignal<String>,
    set_route_search: WriteSignal<String>,
) -> impl IntoView {
    fn mode_color(mode: &str) -> &'static str {
        match mode {
            "auto" => "var(--color-primary-600,#2563eb)",
            "read_only" => "var(--color-info-600,#0284c7)",
            "admin_only" => "var(--color-warning,#d97706)",
            "public" => "var(--color-success,#16a34a)",
            "auth_only" => "var(--color-error,#dc2626)",
            _ => "var(--color-text-secondary)",
        }
    }

    view! {
        <div style="display:flex;flex-direction:column;gap:12px;">
            <div style="display:flex;gap:8px;align-items:center;flex-wrap:wrap;">
                <input
                    type="text"
                    placeholder="Фильтр по пути..."
                    style="padding:6px 10px;border:1px solid var(--color-border);border-radius:4px;font-size:0.85em;min-width:240px;"
                    prop:value=move || route_search.get()
                    on:input=move |e| set_route_search.set(event_target_value(&e))
                />
                <label style="display:flex;align-items:center;gap:6px;font-size:0.85em;cursor:pointer;">
                    <input
                        type="checkbox"
                        prop:checked=move || filter_violations.get()
                        on:change=move |e| set_filter_violations.set(event_target_checked(&e))
                    />
                    "Только нарушения"
                </label>
                {move || {
                    let count = routes.get().len();
                    view! { <span style="font-size:0.8em;color:var(--color-text-secondary);">{format!("{} маршрутов", count)}</span> }
                }}
            </div>

            <div style="overflow-x:auto;">
                <table style="border-collapse:collapse;font-size:0.82em;width:100%;min-width:600px;">
                    <thead>
                        <tr style="background:var(--color-surface-alt,var(--color-surface));">
                            <th style="padding:6px 8px;border-bottom:2px solid var(--color-border);text-align:left;width:60px;">"Метод"</th>
                            <th style="padding:6px 8px;border-bottom:2px solid var(--color-border);text-align:left;">"Путь"</th>
                            <th style="padding:6px 8px;border-bottom:2px solid var(--color-border);text-align:left;width:160px;">"Scope"</th>
                            <th style="padding:6px 8px;border-bottom:2px solid var(--color-border);text-align:left;width:100px;">"Режим"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            let search = route_search.get().to_lowercase();
                            let only_violations = filter_violations.get();
                            routes.get().into_iter().filter(|r| {
                                if only_violations && !r.is_violation { return false; }
                                if !search.is_empty() && !r.path.to_lowercase().contains(&search) { return false; }
                                true
                            }).enumerate().map(|(i, r)| {
                                let row_bg = if r.is_violation {
                                    "background:rgba(220,38,38,0.05);"
                                } else if i % 2 == 0 {
                                    ""
                                } else {
                                    "background:var(--color-surface-alt,var(--color-surface));"
                                };
                                let mode_col = mode_color(&r.mode);
                                let method = r.method.clone();
                                let path = r.path.clone();
                                let scope = r.scope_id.clone().unwrap_or_else(|| "—".to_string());
                                let mode = r.mode.clone();
                                view! {
                                    <tr style=row_bg>
                                        <td style="padding:3px 8px;border-bottom:1px solid var(--color-border);font-family:monospace;font-size:0.9em;font-weight:600;">
                                            {method}
                                        </td>
                                        <td style="padding:3px 8px;border-bottom:1px solid var(--color-border);font-family:monospace;font-size:0.85em;">
                                            {path}
                                            {if r.is_violation { view! { <span style="color:var(--color-error,#dc2626);margin-left:4px;" title="Нарушение политики">"(!)"</span> }.into_any() } else { view! {<></>}.into_any() }}
                                        </td>
                                        <td style="padding:3px 8px;border-bottom:1px solid var(--color-border);font-family:monospace;font-size:0.82em;color:var(--color-text-secondary);">
                                            {scope}
                                        </td>
                                        <td style=format!("padding:3px 8px;border-bottom:1px solid var(--color-border);font-weight:600;color:{};", mode_col)>
                                            {mode}
                                        </td>
                                    </tr>
                                }
                            }).collect::<Vec<_>>()
                        }}
                    </tbody>
                </table>
            </div>
        </div>
    }
}

// ============================================================================
// Violations tab
// ============================================================================

#[component]
fn ViolationsTab(report: AuditReport) -> impl IntoView {
    fn violation_icon(vt: &ViolationType) -> &'static str {
        match vt {
            ViolationType::Unscoped => "[!]",
            ViolationType::UnknownScopeId => "[X]",
            ViolationType::OrphanScope => "[?]",
            ViolationType::OpenNoAuth => "[O]",
        }
    }

    fn violation_color(vt: &ViolationType) -> &'static str {
        match vt {
            ViolationType::Unscoped => "var(--color-warning,#d97706)",
            ViolationType::UnknownScopeId => "var(--color-error,#dc2626)",
            ViolationType::OrphanScope => "var(--color-info-600,#0284c7)",
            ViolationType::OpenNoAuth => "var(--color-error,#dc2626)",
        }
    }

    if report.violations.is_empty() {
        return view! {
            <div style="padding:24px;text-align:center;color:var(--color-success,#16a34a);font-size:1.1em;">
                "✓ Нарушений не обнаружено. Система алгебраически верна."
            </div>
        }.into_any();
    }

    view! {
        <div style="display:flex;flex-direction:column;gap:8px;">
            <p style="font-size:0.85em;color:var(--color-text-secondary);">
                {format!("Обнаружено {} нарушений:", report.violations.len())}
            </p>
            {report.violations.into_iter().map(|v| {
                let emo = violation_icon(&v.violation_type);
                let col = violation_color(&v.violation_type);
                let type_str = v.violation_type.as_str().to_string();
                view! {
                    <div style=format!("padding:10px 14px;border-radius:6px;border-left:4px solid {};background:var(--color-surface-alt,var(--color-surface));", col)>
                        <div style="display:flex;align-items:center;gap:8px;margin-bottom:2px;">
                            <span>{emo}</span>
                            <span style=format!("font-size:0.8em;font-weight:600;font-family:monospace;color:{};", col)>{type_str}</span>
                            <code style="font-size:0.85em;font-weight:600;">{v.subject.clone()}</code>
                        </div>
                        <div style="font-size:0.82em;color:var(--color-text-secondary);">{v.description.clone()}</div>
                    </div>
                }
            }).collect::<Vec<_>>()}
        </div>
    }.into_any()
}
