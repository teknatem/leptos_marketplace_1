use crate::dashboards::d404_wb_advert_report::api;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use chrono::{Datelike, Utc};
use contracts::dashboards::d404_wb_advert_report::{WbAdvertReportNode, WbAdvertReportResponse};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashSet;

fn money(value: f64) -> String {
    if value.abs() >= 10_000.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}

fn signed_class(value: f64) -> &'static str {
    if value < -0.005 {
        "d404-money d404-money--negative"
    } else if value > 0.005 {
        "d404-money d404-money--positive"
    } else {
        "d404-money d404-money--zero"
    }
}

fn first_day_of_month() -> String {
    let today = Utc::now().date_naive();
    format!("{:04}-{:02}-01", today.year(), today.month())
}

fn today() -> String {
    Utc::now().date_naive().format("%Y-%m-%d").to_string()
}

#[component]
fn MoneyCell(value: f64) -> impl IntoView {
    view! { <td class=signed_class(value)>{money(value)}</td> }
}

#[component]
fn ReportRow(node: WbAdvertReportNode, expanded: RwSignal<HashSet<String>>) -> AnyView {
    let tabs = expect_context::<AppGlobalContext>();
    let icon_id = node.id.clone();
    let children_id = node.id.clone();
    let primary_link = node.links.first().cloned();
    let links = node.links.clone();
    let has_children = !node.children.is_empty();
    let children = node.children.clone();
    let level_class = format!("d404-row d404-row--{}", node.level);
    let indent = match node.level.as_str() {
        "campaign" => 0,
        "nomenclature" => 22,
        _ => 44,
    };
    let is_open_icon = move || expanded.with(|set| set.contains(&icon_id));
    let is_open_children = move || expanded.with(|set| set.contains(&children_id));
    let toggle_id = node.id.clone();
    let toggle = move |_| {
        if has_children {
            expanded.update(|set| {
                if set.contains(&toggle_id) {
                    set.remove(&toggle_id);
                } else {
                    set.insert(toggle_id.clone());
                }
            });
        }
    };

    view! {
        <>
            <tr class=level_class>
                <td class="d404-name">
                    <button
                        class="d404-toggle"
                        class:d404-toggle--empty=move || !has_children
                        on:click=toggle
                    >
                        {move || {
                            if !has_children {
                                view! { <span></span> }.into_any()
                            } else if is_open_icon() {
                                icon("chevron-down")
                            } else {
                                icon("chevron-right")
                            }
                        }}
                    </button>
                    {if let Some(link) = primary_link {
                        let tab_key = link.tab_key.clone();
                        let title = node.label.clone();
                        let title_for_click = title.clone();
                        let tabs_for_click = tabs.clone();
                        view! {
                            <button
                                style=format!("padding-left: {indent}px")
                                class="d404-label d404-label--link"
                                on:click=move |_| tabs_for_click.open_tab(&tab_key, &title_for_click)
                            >
                                {title}
                            </button>
                        }.into_any()
                    } else {
                        view! {
                            <span style=format!("padding-left: {indent}px") class="d404-label">
                                {node.label.clone()}
                            </span>
                        }.into_any()
                    }}
                    <span class="d404-links">
                        {links.into_iter().map(|link| {
                            let tab_key_for_click = link.tab_key.clone();
                            let label = link.label.clone();
                            let label_for_click = label.clone();
                            let label_title = label.clone();
                            let tabs_for_click = tabs.clone();
                            view! {
                                <button
                                    class="d404-link"
                                    title=label_title
                                    on:click=move |_| {
                                        tabs_for_click.open_tab(&tab_key_for_click, &label_for_click)
                                    }
                                >
                                    {label}
                                </button>
                            }
                        }).collect_view()}
                    </span>
                </td>
                <MoneyCell value=node.accrued />
                <MoneyCell value=node.expensed />
                <MoneyCell value=node.balance />
                <MoneyCell value=node.expense_no_order />
            </tr>
            <Show when=move || has_children && is_open_children()>
                {children
                    .iter()
                    .cloned()
                    .map(|child| view! { <ReportRow node=child expanded=expanded /> })
                    .collect_view()}
            </Show>
        </>
    }
    .into_any()
}

#[component]
pub fn WbAdvertReportDashboard() -> impl IntoView {
    let date_from = RwSignal::new(first_day_of_month());
    let date_to = RwSignal::new(today());
    let campaign_code = RwSignal::new(String::new());
    let connection_mp_ref = RwSignal::new(String::new());
    let data = RwSignal::new(None::<WbAdvertReportResponse>);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let expanded = RwSignal::new(HashSet::<String>::new());

    let load = move || {
        let df = date_from.get_untracked();
        let dt = date_to.get_untracked();
        let cc = campaign_code.get_untracked();
        let conn = connection_mp_ref.get_untracked();
        loading.set(true);
        error.set(None);
        spawn_local(async move {
            match api::get_wb_advert_report(&df, &dt, &cc, &conn).await {
                Ok(response) => {
                    let root_ids = response
                        .campaigns
                        .iter()
                        .map(|node| node.id.clone())
                        .collect::<HashSet<_>>();
                    expanded.set(root_ids);
                    data.set(Some(response));
                    loading.set(false);
                }
                Err(message) => {
                    error.set(Some(message));
                    loading.set(false);
                }
            }
        });
    };

    Effect::new(move |_| load());

    view! {
        <PageFrame page_id="d404_wb_advert_report--dashboard" category="dashboard" class="page--wide">
            <style>
                ".d404-shell{display:flex;flex-direction:column;gap:12px;height:100%}
                .d404-toolbar{display:flex;gap:10px;align-items:end;flex-wrap:wrap;padding:10px 0}
                .d404-field{display:flex;flex-direction:column;gap:4px;min-width:160px}
                .d404-field label{font-size:12px;color:var(--color-text-secondary)}
                .d404-field input{height:32px;border:1px solid var(--color-border);border-radius:6px;padding:0 8px;background:var(--color-surface);color:var(--color-text-primary)}
                .d404-btn{height:32px;border:1px solid var(--color-border);border-radius:6px;background:var(--color-surface);color:var(--color-text-primary);padding:0 12px;cursor:pointer}
                .d404-summary{display:flex;gap:18px;flex-wrap:wrap;border:1px solid var(--color-border-light,var(--color-border));border-radius:8px;padding:10px;background:var(--color-surface)}
                .d404-summary span{font-size:12px;color:var(--color-text-secondary)}
                .d404-summary strong{font-size:14px;color:var(--color-text-primary);font-variant-numeric:tabular-nums}
                .d404-table-wrap{overflow:auto;border:1px solid var(--color-border-light,var(--color-border));border-radius:8px;background:var(--color-surface)}
                .d404-table{width:100%;border-collapse:collapse;font-size:13px}
                .d404-table th{position:sticky;top:0;background:var(--color-surface);z-index:1;text-align:right;border-bottom:1px solid var(--color-border);padding:8px;color:var(--color-text-secondary);font-weight:600}
                .d404-table th:first-child{text-align:left;min-width:420px}
                .d404-table td{border-bottom:1px solid var(--color-border-light,var(--color-border));padding:7px 8px}
                .d404-row--campaign{background:color-mix(in srgb,var(--color-brand,#2563eb) 7%,transparent);font-weight:700}
                .d404-row--nomenclature{font-weight:600}
                .d404-name{display:flex;align-items:center;gap:6px;white-space:nowrap}
                .d404-toggle{width:22px;height:22px;border:0;background:transparent;color:var(--color-text-secondary);display:inline-flex;align-items:center;justify-content:center;cursor:pointer}
                .d404-toggle--empty{cursor:default;visibility:hidden}
                .d404-label{overflow:hidden;text-overflow:ellipsis}
                .d404-label--link{border:0;background:transparent;color:var(--color-brand,#2563eb);cursor:pointer;text-align:left;font:inherit;text-decoration:none}
                .d404-label--link:hover{text-decoration:underline}
                .d404-links{display:inline-flex;gap:4px;margin-left:8px}
                .d404-link{height:22px;border:1px solid var(--color-border-light,var(--color-border));border-radius:5px;background:transparent;color:var(--color-text-secondary);font-size:11px;padding:0 6px;cursor:pointer}
                .d404-link:hover{color:var(--color-brand,#2563eb);border-color:var(--color-brand,#2563eb)}
                .d404-money{text-align:right;font-variant-numeric:tabular-nums}
                .d404-money--negative{color:#dc2626}
                .d404-money--positive{color:var(--color-text-primary)}
                .d404-money--zero{color:var(--color-text-tertiary)}
                .d404-state{padding:18px;color:var(--color-text-secondary)}"
            </style>
            <div class="d404-shell">
                <div>
                    <h1 style="margin:0;font-size:20px;">"Реклама WB"</h1>
                    <div style="color:var(--color-text-secondary);font-size:13px;">
                        "Кампания / Номенклатура / Заказ"
                    </div>
                </div>

                <div class="d404-toolbar">
                    <div class="d404-field">
                        <label>"Период с"</label>
                        <input
                            type="date"
                            prop:value=move || date_from.get()
                            on:input=move |ev| date_from.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="d404-field">
                        <label>"Период по"</label>
                        <input
                            type="date"
                            prop:value=move || date_to.get()
                            on:input=move |ev| date_to.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="d404-field">
                        <label>"Кампания"</label>
                        <input
                            placeholder="advert_id"
                            prop:value=move || campaign_code.get()
                            on:input=move |ev| campaign_code.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="d404-field">
                        <label>"Кабинет"</label>
                        <input
                            placeholder="connection_mp_ref"
                            prop:value=move || connection_mp_ref.get()
                            on:input=move |ev| connection_mp_ref.set(event_target_value(&ev))
                        />
                    </div>
                    <button class="d404-btn" on:click=move |_| load() disabled=move || loading.get()>
                        {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                    </button>
                </div>

                {move || error.get().map(|message| view! {
                    <div class="d404-state">{message}</div>
                })}

                {move || data.get().map(|response| {
                    let totals = response.totals.clone();
                    view! {
                        <div class="d404-summary">
                            <span>"Начислено: " <strong>{money(totals.accrued)}</strong></span>
                            <span>"Списано: " <strong>{money(totals.expensed)}</strong></span>
                            <span>"Остаток: " <strong>{money(totals.balance)}</strong></span>
                            <span>"РасходБезЗаказа: " <strong>{money(totals.expense_no_order)}</strong></span>
                        </div>
                        <div class="d404-table-wrap">
                            <table class="d404-table">
                                <thead>
                                    <tr>
                                        <th>"Кампания / Номенклатура / Заказ"</th>
                                        <th>"Начислено"</th>
                                        <th>"Списано"</th>
                                        <th>"Остаток"</th>
                                        <th>"РасходБезЗаказа"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {if response.campaigns.is_empty() {
                                        view! {
                                            <tr><td class="d404-state" colspan="5">"Нет данных за выбранный период."</td></tr>
                                        }.into_any()
                                    } else {
                                        response.campaigns.iter().cloned().map(|node| {
                                            view! { <ReportRow node=node expanded=expanded /> }
                                        }).collect_view().into_any()
                                    }}
                                </tbody>
                            </table>
                        </div>
                    }
                })}
            </div>
        </PageFrame>
    }
}
