use crate::dashboards::d406_wb_sales_funnel::api;
use crate::shared::page_frame::PageFrame;
use chrono::{Datelike, Utc};
use contracts::dashboards::d406_wb_sales_funnel::{
    FunnelDateAxis, WbSalesFunnelConversions, WbSalesFunnelMetrics, WbSalesFunnelResponse,
    WbSalesFunnelRow,
};
use leptos::prelude::*;
use leptos::task::spawn_local;

fn num(value: i64) -> String {
    value.to_string()
}

/// Показ метрики показов с учётом доступности источника: `N/A`, если источника нет
/// (напр. нет подписки «Джем»/рекламы) — «недоступно» ≠ «ноль».
fn num_avail(value: i64, available: bool) -> String {
    if available {
        value.to_string()
    } else {
        "N/A".to_string()
    }
}

fn money(value: f64) -> String {
    format!("{value:.0}")
}

/// Конверсия в % (или прочерк, если знаменатель = 0).
fn pct(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{v:.1}%"),
        None => "—".to_string(),
    }
}

fn first_day_of_month() -> String {
    let today = Utc::now().date_naive();
    format!("{:04}-{:02}-01", today.year(), today.month())
}

fn today() -> String {
    Utc::now().date_naive().format("%Y-%m-%d").to_string()
}

fn product_label(row: &WbSalesFunnelRow) -> String {
    if let Some(name) = row.product_name.as_ref().filter(|s| !s.trim().is_empty()) {
        name.clone()
    } else if let Some(nm) = row.nm_id {
        format!("nm {nm}")
    } else {
        "—".to_string()
    }
}

#[component]
fn MetricRow(
    #[prop(into)] label: String,
    metrics: WbSalesFunnelMetrics,
    conversions: WbSalesFunnelConversions,
    #[prop(optional, into)] date: Option<String>,
    #[prop(optional)] is_total: bool,
) -> impl IntoView {
    let row_class = if is_total {
        "d406-row d406-row--total"
    } else {
        "d406-row"
    };
    view! {
        <tr class=row_class>
            <td class="d406-date">{date.unwrap_or_default()}</td>
            <td class="d406-name">{label}</td>
            <td class="d406-n">{num_avail(metrics.show_total_count, metrics.show_total_available)}</td>
            <td class="d406-n">{num_avail(metrics.show_paid_count, metrics.show_paid_available)}</td>
            <td class="d406-n">{num_avail(metrics.show_free_count, metrics.show_free_available)}</td>
            <td class="d406-n">{num(metrics.open_count)}</td>
            <td class="d406-n">{num(metrics.cart_count)}</td>
            <td class="d406-n">{num(metrics.order_count)}</td>
            <td class="d406-n">{num(metrics.buyout_count)}</td>
            <td class="d406-n">{num(metrics.cancel_count)}</td>
            <td class="d406-n">{num(metrics.return_count)}</td>
            <td class="d406-money">{money(metrics.order_sum)}</td>
            <td class="d406-c">{pct(conversions.open_to_cart)}</td>
            <td class="d406-c">{pct(conversions.cart_to_order)}</td>
            <td class="d406-c">{pct(conversions.order_to_buyout)}</td>
            <td class="d406-c">{pct(conversions.cancel_rate)}</td>
        </tr>
    }
}

#[component]
pub fn WbSalesFunnelDashboard() -> impl IntoView {
    let date_from = RwSignal::new(first_day_of_month());
    let date_to = RwSignal::new(today());
    let connection_mp_ref = RwSignal::new(String::new());
    let nm_id = RwSignal::new(String::new());
    let axis = RwSignal::new(FunnelDateAxis::Cohort);
    let data = RwSignal::new(None::<WbSalesFunnelResponse>);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let load = move || {
        let df = date_from.get_untracked();
        let dt = date_to.get_untracked();
        let conn = connection_mp_ref.get_untracked();
        let nm = nm_id.get_untracked();
        let ax = axis.get_untracked();
        loading.set(true);
        error.set(None);
        spawn_local(async move {
            match api::get_wb_sales_funnel(&df, &dt, &conn, &nm, ax).await {
                Ok(response) => {
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
        <PageFrame page_id="d406_wb_sales_funnel--dashboard" category="dashboard" class="page--wide">
            <style>
                ".d406-shell{display:flex;flex-direction:column;gap:12px;height:100%}
                .d406-toolbar{display:flex;gap:10px;align-items:end;flex-wrap:wrap;padding:10px 0}
                .d406-field{display:flex;flex-direction:column;gap:4px;min-width:150px}
                .d406-field label{font-size:12px;color:var(--color-text-secondary)}
                .d406-field input,.d406-field select{height:32px;border:1px solid var(--color-border);border-radius:6px;padding:0 8px;background:var(--color-surface);color:var(--color-text-primary)}
                .d406-btn{height:32px;border:1px solid var(--color-border);border-radius:6px;background:var(--color-surface);color:var(--color-text-primary);padding:0 12px;cursor:pointer}
                .d406-note{font-size:12px;color:var(--color-text-tertiary)}
                .d406-table-wrap{overflow:auto;border:1px solid var(--color-border-light,var(--color-border));border-radius:8px;background:var(--color-surface)}
                .d406-table{width:100%;border-collapse:collapse;font-size:13px;white-space:nowrap}
                .d406-table th{position:sticky;top:0;background:var(--color-surface);z-index:1;text-align:right;border-bottom:1px solid var(--color-border);padding:8px;color:var(--color-text-secondary);font-weight:600}
                .d406-table th:nth-child(1),.d406-table th:nth-child(2){text-align:left}
                .d406-table td{border-bottom:1px solid var(--color-border-light,var(--color-border));padding:7px 8px}
                .d406-date{white-space:nowrap;color:var(--color-text-secondary)}
                .d406-name{min-width:200px;overflow:hidden;text-overflow:ellipsis;max-width:320px}
                .d406-n,.d406-money,.d406-c{text-align:right;font-variant-numeric:tabular-nums}
                .d406-c{color:var(--color-text-secondary)}
                .d406-row--total{background:color-mix(in srgb,var(--color-brand,#2563eb) 8%,transparent);font-weight:700}
                .d406-row--total td{border-bottom:2px solid var(--color-border)}
                .d406-state{padding:18px;color:var(--color-text-secondary)}
                .d406-group{border-right:1px solid var(--color-border)}"
            </style>
            <div class="d406-shell">
                <div>
                    <h1 style="margin:0;font-size:20px;">"Воронка продаж WB"</h1>
                    <div class="d406-note">
                        "Показы → переходы → корзина → заказы → выкупы. Показы: платные (реклама a026) + бесплатные (органика a040)."
                    </div>
                </div>

                <div class="d406-toolbar">
                    <div class="d406-field">
                        <label>"Период с"</label>
                        <input
                            type="date"
                            prop:value=move || date_from.get()
                            on:input=move |ev| date_from.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="d406-field">
                        <label>"Период по"</label>
                        <input
                            type="date"
                            prop:value=move || date_to.get()
                            on:input=move |ev| date_to.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="d406-field">
                        <label>"Кабинет"</label>
                        <input
                            placeholder="connection_mp_ref"
                            prop:value=move || connection_mp_ref.get()
                            on:input=move |ev| connection_mp_ref.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="d406-field">
                        <label>"nm_id"</label>
                        <input
                            placeholder="все"
                            prop:value=move || nm_id.get()
                            on:input=move |ev| nm_id.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="d406-field">
                        <label title="Когорта — по дате заказа (винтаж); Событие — по дате транзакции события">
                            "Ось дат"
                        </label>
                        <select
                            on:change=move |ev| {
                                let v = event_target_value(&ev);
                                axis.set(if v == "event" { FunnelDateAxis::Event } else { FunnelDateAxis::Cohort });
                            }
                        >
                            <option value="cohort">"Когорта (дата заказа)"</option>
                            <option value="event">"Событие (дата транзакции)"</option>
                        </select>
                    </div>
                    <button class="d406-btn" on:click=move |_| load() disabled=move || loading.get()>
                        {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                    </button>
                </div>

                <div class="d406-note">
                    "Когортная ось: выкупы/возвраты a012 привязываются к дате заказа (a015 по srid); если заказ не найден — фолбэк на дату продажи. Показы: N/A означает «источник недоступен» (нет «Джем»/рекламы), а не ноль."
                </div>

                {move || error.get().map(|message| view! {
                    <div class="d406-state">{message}</div>
                })}

                {move || data.get().map(|response| {
                    let rows = response.rows.clone();
                    let totals = response.totals.clone();
                    let totals_conv = response.totals_conversions.clone();
                    view! {
                        <div class="d406-table-wrap">
                            <table class="d406-table">
                                <thead>
                                    <tr>
                                        <th>"Дата"</th>
                                        <th>"Товар"</th>
                                        <th title="Всего показов = платные + бесплатные">"Показы"</th>
                                        <th title="Рекламные показы (a026)">"Платн."</th>
                                        <th title="Органические показы (a040)">"Беспл."</th>
                                        <th>"Переходы"</th>
                                        <th>"Корзина"</th>
                                        <th title="Фактические заказы (a015)">"Заказы"</th>
                                        <th>"Выкупы"</th>
                                        <th>"Отмены"</th>
                                        <th>"Возвраты"</th>
                                        <th>"Сумма заказов"</th>
                                        <th title="Корзина / переходы">"→корзина"</th>
                                        <th title="Заказы / корзина">"→заказ"</th>
                                        <th title="Выкупы / заказы">"→выкуп"</th>
                                        <th title="Отмены / заказы">"отмены"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    <MetricRow
                                        label="Итого за период".to_string()
                                        metrics=totals
                                        conversions=totals_conv
                                        is_total=true
                                    />
                                    {if rows.is_empty() {
                                        view! {
                                            <tr><td class="d406-state" colspan="16">"Нет данных за выбранный период."</td></tr>
                                        }.into_any()
                                    } else {
                                        rows.into_iter().map(|row| {
                                            let label = product_label(&row);
                                            view! {
                                                <MetricRow
                                                    label=label
                                                    metrics=row.metrics
                                                    conversions=row.conversions
                                                    date=row.date
                                                />
                                            }
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
