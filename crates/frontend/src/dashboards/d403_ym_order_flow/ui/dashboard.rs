use crate::dashboards::d403_ym_order_flow::api;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use contracts::dashboards::d403_ym_order_flow::YmOrderFlowResponse;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

// ── Палитра ───────────────────────────────────────────────────────────
// Те же оттенки, что и в d402 — единый визуальный язык лент истории.

const SURFACE: &str = "var(--color-surface)";
const SURFACE_ALT: &str = "var(--color-surface-alt, var(--color-bg-secondary))";
const BORDER: &str = "var(--color-border-light, var(--color-border))";
const BORDER_STRONG: &str = "var(--color-border)";

const T_PRIMARY: &str = "var(--color-text-primary)";
const T_SECONDARY: &str = "var(--color-text-secondary)";
const T_MUTED: &str = "var(--color-text-tertiary)";

const CP: &str = "#15803d"; // продажа / доход (green-700)
const CE: &str = "#dc2626"; // возврат / расход (red-600)
const CN: &str = "#94a3b8"; // платёж — нейтральный серый (slate-400)
const CO: &str = "#2563eb"; // заказ — синий (blue-600)

fn accent_tint(hex: &str) -> String {
    format!("color-mix(in srgb, {hex} 10%, transparent)")
}

// ── Helpers ───────────────────────────────────────────────────────────

fn rub(v: f64) -> String {
    if v.abs() >= 10_000.0 {
        format!("{:.0} ₽", v)
    } else {
        format!("{:.2} ₽", v)
    }
}

fn jdn_from_dmy(s: &str) -> Option<i64> {
    let p: Vec<&str> = s.split('.').collect();
    if p.len() != 3 {
        return None;
    }
    let d = p[0].parse::<i64>().ok()?;
    let m = p[1].parse::<i64>().ok()?;
    let y = p[2].parse::<i64>().ok()?;
    let a = (14 - m) / 12;
    let yy = y + 4800 - a;
    let mm = m + 12 * a - 3;
    Some(d + (153 * mm + 2) / 5 + 365 * yy + yy / 4 - yy / 100 + yy / 400 - 32045)
}

/// (julian-day, текст `DD.MM.YYYY`) из строки `DD.MM.YYYY` или `YYYY-MM-DD`
/// (с опциональным временем после даты).
fn parse_event_date(s: &str) -> Option<(i64, String)> {
    if s.is_empty() {
        return None;
    }
    if let Some(j) = jdn_from_dmy(s) {
        return Some((j, s.to_string()));
    }
    let date_part = s.split(['T', ' ']).next().unwrap_or(s);
    let p: Vec<&str> = date_part.split('-').collect();
    if p.len() == 3 && p[0].len() == 4 {
        let formatted = format!("{}.{}.{}", p[2], p[1], p[0]);
        return jdn_from_dmy(&formatted).map(|j| (j, formatted));
    }
    None
}

fn open_btn(key: String, label: &'static str, ctx: AppGlobalContext) -> impl IntoView {
    view! {
        <button
            style=format!("
                padding: 4px 10px; background: transparent;
                border: 1px solid {BORDER_STRONG}; border-radius: 5px;
                color: {T_SECONDARY}; font-size: 12px; cursor: pointer;
                white-space: nowrap;
            ")
            on:click=move |_| ctx.open_tab(&key, label)
        >{"↗ "}{label}</button>
    }
}

fn detail_row(label: &'static str, value: String, color: String) -> AnyView {
    view! {
        <div style="display: flex; gap: 10px; align-items: baseline;">
            <span style=format!("font-size: 13px; color: {T_SECONDARY}; font-weight: 500;")>
                {label}
            </span>
            <span style=format!("font-size: 14px; color: {color}; font-weight: 700;
                                 font-variant-numeric: tabular-nums;")>{value}</span>
        </div>
    }
    .into_any()
}

// ─────────────────────────────────────────────────────────────────────
// Модель события в ленте
// ─────────────────────────────────────────────────────────────────────

struct TimelineEvent {
    sort_key: i64,
    julian_day: i64,
    date_text: String,
    accent: &'static str,
    type_label: &'static str,
    title: String,
    subtitle: Option<String>,
    details: Vec<AnyView>,
    /// > 0 — доход, < 0 — расход, None — без денежного движения.
    amount: Option<f64>,
    links: Vec<(String, &'static str)>,
}

fn build_events(data: &YmOrderFlowResponse) -> Vec<TimelineEvent> {
    let mut events: Vec<TimelineEvent> = Vec::new();

    // 0. Заказ a013 — первое событие своего дня (как в d402).
    if let Some(o) = &data.order {
        if let Some((jd, dt)) = o.order_date.as_deref().and_then(parse_event_date) {
            let mut title = format!("Заказ {}", o.document_no);
            if o.qty.abs() > 0.0001 {
                title = format!("{} · {:.0} шт", title, o.qty);
            }
            let mut details: Vec<AnyView> = Vec::new();
            if let Some(s) = o.status.as_deref().filter(|s| !s.is_empty()) {
                details.push(detail_row("Статус", s.to_string(), T_PRIMARY.to_string()));
            }
            if let Some(d) = o.delivery_date.as_deref().filter(|s| !s.is_empty()) {
                details.push(detail_row(
                    "Доставка",
                    d.to_string(),
                    T_SECONDARY.to_string(),
                ));
            }
            if let Some(v) = o.items_total.filter(|v| v.abs() > 0.0001) {
                details.push(detail_row(
                    "Платёж покупателя",
                    rub(v),
                    T_PRIMARY.to_string(),
                ));
            }
            events.push(TimelineEvent {
                sort_key: jd * 10, // раньше реализации (+1) на тот же день
                julian_day: jd,
                date_text: dt,
                accent: CO,
                type_label: "Заказ",
                title,
                subtitle: None,
                details,
                amount: None,
                links: vec![(format!("a013_ym_order_details_{}", o.id), "Заказ")],
            });
        }
    }

    // 1. Реализация a034 — продажа или возврат.
    for r in &data.realizations {
        if let Some((jd, dt)) = parse_event_date(&r.document_date) {
            let (accent, type_label) = if r.is_return {
                (CE, "Возврат")
            } else {
                (CP, "Реализация")
            };
            let amount = if r.is_return {
                -r.revenue_amount.abs()
            } else {
                r.revenue_amount.abs()
            };

            let title = if r.offer_name.is_empty() {
                r.shop_sku.clone()
            } else {
                r.offer_name.clone()
            };

            let mut details: Vec<AnyView> = Vec::new();
            if r.quantity.abs() > 0.0001 {
                details.push(detail_row(
                    "Кол-во",
                    format!("{:.0}", r.quantity),
                    T_PRIMARY.to_string(),
                ));
            }
            if !r.shop_sku.is_empty() {
                details.push(detail_row(
                    "SKU",
                    r.shop_sku.clone(),
                    T_SECONDARY.to_string(),
                ));
            }

            events.push(TimelineEvent {
                sort_key: jd * 10 + 1,
                julian_day: jd,
                date_text: dt,
                accent,
                type_label,
                title,
                subtitle: None,
                details,
                amount: Some(amount),
                links: vec![(
                    format!("a034_ym_realization_details_{}", r.doc_id),
                    "Реализация",
                )],
            });
        }
    }

    // 2. Платежи p907 — каждая транзакция как событие.
    for p in &data.payments {
        let date_src = p.transaction_date.clone().unwrap_or_default();
        if let Some((jd, dt)) = parse_event_date(&date_src) {
            let title = p
                .offer_or_service_name
                .clone()
                .filter(|s| !s.is_empty())
                .or_else(|| p.transaction_type.clone())
                .unwrap_or_else(|| "Транзакция".to_string());

            let mut sub_parts: Vec<String> = Vec::new();
            if let Some(t) = p.transaction_type.as_deref().filter(|s| !s.is_empty()) {
                sub_parts.push(t.to_string());
            }
            if let Some(st) = p.payment_status.as_deref().filter(|s| !s.is_empty()) {
                sub_parts.push(st.to_string());
            }
            let subtitle = (!sub_parts.is_empty()).then(|| sub_parts.join(" · "));

            let mut details: Vec<AnyView> = Vec::new();
            if let Some(bank) = p.bank_sum.filter(|v| v.abs() > 0.0001) {
                details.push(detail_row("Сумма ПП", rub(bank), T_SECONDARY.to_string()));
            }
            if let Some(sku) = p.shop_sku.as_deref().filter(|s| !s.is_empty()) {
                details.push(detail_row("SKU", sku.to_string(), T_SECONDARY.to_string()));
            }
            if let Some(src) = p.transaction_source.as_deref().filter(|s| !s.is_empty()) {
                details.push(detail_row(
                    "Источник",
                    src.to_string(),
                    T_SECONDARY.to_string(),
                ));
            }

            events.push(TimelineEvent {
                sort_key: jd * 10 + 5,
                julian_day: jd,
                date_text: dt,
                accent: CN,
                type_label: "Платёж",
                title,
                subtitle,
                details,
                amount: p.transaction_sum.filter(|v| v.abs() > 0.0001),
                links: vec![(format!("p907_ym_payment_report_details_{}", p.id), "Платёж")],
            });
        }
    }

    // 3. Возвраты a016 — заявка/возврат по заказу (если есть).
    for r in &data.returns {
        let date_src = r.created_at_source.clone().unwrap_or_default();
        if let Some((jd, dt)) = parse_event_date(&date_src) {
            let type_label = if r.return_type == "UNREDEEMED" {
                "Невыкуп"
            } else {
                "Возврат YM"
            };
            let mut details: Vec<AnyView> = Vec::new();
            if r.amount.abs() > 0.0001 {
                details.push(detail_row("Сумма возврата", rub(r.amount), CE.to_string()));
            }
            if r.qty != 0 {
                details.push(detail_row(
                    "Кол-во",
                    format!("{}", r.qty),
                    T_PRIMARY.to_string(),
                ));
            }
            if !r.refund_status.is_empty() {
                details.push(detail_row(
                    "Статус",
                    r.refund_status.clone(),
                    T_SECONDARY.to_string(),
                ));
            }
            events.push(TimelineEvent {
                sort_key: jd * 10 + 3,
                julian_day: jd,
                date_text: dt,
                accent: CE,
                type_label,
                title: format!("Возврат №{}", r.return_id),
                subtitle: None,
                details,
                amount: None,
                links: vec![(format!("a016_ym_returns_details_{}", r.id), "Возврат")],
            });
        }
    }

    events.sort_by_key(|e| e.sort_key);
    events
}

// ─────────────────────────────────────────────────────────────────────
// Карточка-шапка: номер заказа + итоги доход/расход
// ─────────────────────────────────────────────────────────────────────

fn render_hero(data: &YmOrderFlowResponse, events: &[TimelineEvent]) -> AnyView {
    let order_no = data.order_no.clone();

    let income: f64 = events
        .iter()
        .filter_map(|e| e.amount)
        .filter(|a| *a > 0.0)
        .sum();
    let expense: f64 = events
        .iter()
        .filter_map(|e| e.amount)
        .filter(|a| *a < 0.0)
        .map(|a| -a)
        .sum();
    let net = income - expense;

    let total = |label: &'static str, val: f64, color: &'static str, sign: &'static str| {
        let txt = if val.abs() < 0.001 {
            "—".to_string()
        } else {
            format!("{sign}{}", rub(val))
        };
        view! {
            <div style="text-align: right;">
                <div style=format!("font-size: 11px; color: {T_SECONDARY}; text-transform: uppercase;
                                    letter-spacing: 0.08em; font-weight: 600;")>{label}</div>
                <div style=format!("font-size: 20px; font-weight: 700; color: {color};
                                    font-variant-numeric: tabular-nums; line-height: 1.2;")>{txt}</div>
            </div>
        }
    };

    let net_color = if net > 0.0 {
        CP
    } else if net < 0.0 {
        CE
    } else {
        T_SECONDARY
    };
    let net_sign = if net >= 0.0 { "+" } else { "−" };

    view! {
        <div style=format!("
            background: {SURFACE}; border: 1px solid {BORDER};
            border-radius: 10px; padding: 16px 20px;
            display: flex; gap: 28px; align-items: center; flex-wrap: wrap;
            box-shadow: var(--shadow-sm);
        ")>
            <div style="flex: 1; min-width: 220px;">
                <div style="display: flex; gap: 10px; align-items: baseline; flex-wrap: wrap;">
                    <span style=format!("font-size: 12px; color: {T_SECONDARY}; font-weight: 600;
                                         text-transform: uppercase; letter-spacing: 0.06em;")>
                        {"Заказ YM"}
                    </span>
                    <span style=format!("font-size: 18px; font-weight: 700; color: {T_PRIMARY};
                                         font-family: ui-monospace, monospace;")>
                        {order_no}
                    </span>
                </div>
            </div>
            <div style=format!("display: flex; gap: 28px; padding-left: 28px;
                                border-left: 1px solid {BORDER};")>
                {total("Доход",  income,    CP,        "+")}
                {total("Расход", expense,   CE,        "−")}
                {total("Итого",  net.abs(), net_color, net_sign)}
            </div>
        </div>
    }
    .into_any()
}

// ─────────────────────────────────────────────────────────────────────
// Одно событие
// ─────────────────────────────────────────────────────────────────────

fn render_event(
    event: TimelineEvent,
    ctx: AppGlobalContext,
    days_from_prev: Option<i64>,
    is_first: bool,
    is_last: bool,
) -> impl IntoView {
    let accent = event.accent;
    let accent_bg = accent_tint(accent);

    let amount_view = event.amount.map(|a| {
        let (color, sign) = if a >= 0.0 { (CP, "+") } else { (CE, "−") };
        view! {
            <span style=format!("font-size: 15px; font-weight: 700; color: {color};
                                 font-variant-numeric: tabular-nums; white-space: nowrap;
                                 flex-shrink: 0;")>
                {sign}{rub(a.abs())}
            </span>
        }
    });

    let TimelineEvent {
        date_text,
        type_label,
        title,
        subtitle,
        details,
        links,
        ..
    } = event;

    let line_top_color = if is_first {
        "transparent".to_string()
    } else {
        BORDER_STRONG.to_string()
    };
    let line_bot_color = if is_last {
        "transparent".to_string()
    } else {
        BORDER_STRONG.to_string()
    };

    let days_label = days_from_prev.map(|d| {
        if d <= 0 {
            "+0 дн".to_string()
        } else {
            format!("+{} дн", d)
        }
    });

    let links_view = (!links.is_empty()).then(|| {
        view! {
            <div style="display: flex; gap: 5px; flex-wrap: nowrap; flex-shrink: 0;">
                {links.into_iter().map(|(key, lbl)|
                    open_btn(key, lbl, ctx)
                ).collect_view()}
            </div>
        }
    });

    view! {
        <div style="display: grid; grid-template-columns: 100px 24px minmax(0, 1fr);
                     gap: 0 12px; align-items: stretch;">
            <div style="padding-top: 9px; text-align: right;
                         display: flex; flex-direction: column; gap: 1px;
                         align-items: flex-end;">
                <span style=format!("font-size: 13px; color: {T_PRIMARY}; font-weight: 600;
                                     font-variant-numeric: tabular-nums; white-space: nowrap;")>
                    {date_text}
                </span>
                {days_label.map(|d| view! {
                    <span style=format!("font-size: 11px; color: {T_MUTED};
                                         font-variant-numeric: tabular-nums; white-space: nowrap;")>
                        {d}
                    </span>
                })}
            </div>

            <div style="display: flex; flex-direction: column; align-items: center;">
                <div style=format!("width: 2px; height: 13px; background: {line_top_color};")></div>
                <div style=format!("width: 12px; height: 12px; border-radius: 50%; background: {accent};
                                     flex-shrink: 0;")></div>
                <div style=format!("width: 2px; flex: 1; min-height: 6px; background: {line_bot_color};")></div>
            </div>

            <div style=format!("background: {SURFACE}; border: 1px solid {BORDER};
                                 border-left: 3px solid {accent}; border-radius: 6px;
                                 padding: 8px 14px; margin-bottom: 5px;
                                 box-shadow: var(--shadow-sm);")>
                <div style="display: flex; gap: 12px; align-items: center;
                             justify-content: space-between; min-width: 0;">
                    <div style="display: flex; gap: 10px; align-items: baseline;
                                 min-width: 0; flex: 1;">
                        <span style=format!("display: inline-block; font-size: 11px; font-weight: 700;
                                              color: {accent}; text-transform: uppercase;
                                              letter-spacing: 0.06em; background: {accent_bg};
                                              padding: 2px 8px; border-radius: 4px;
                                              flex-shrink: 0;")>
                            {type_label}
                        </span>
                        {(!title.is_empty()).then(|| view! {
                            <span style=format!("font-size: 14px; color: {T_PRIMARY}; font-weight: 600;
                                                 overflow: hidden; text-overflow: ellipsis;
                                                 white-space: nowrap; min-width: 0;")>
                                {title}
                            </span>
                        })}
                    </div>
                    <div style="display: flex; gap: 10px; align-items: center; flex-shrink: 0;">
                        {links_view}
                        {amount_view}
                    </div>
                </div>

                {subtitle.map(|s| view! {
                    <div style=format!("font-size: 12px; color: {T_SECONDARY}; margin-top: 3px;")>
                        {s}
                    </div>
                })}

                {(!details.is_empty()).then(|| view! {
                    <div style="display: flex; flex-wrap: wrap; margin-top: 5px;
                                 column-gap: 20px; row-gap: 3px;">
                        {details.into_iter().collect_view()}
                    </div>
                })}
            </div>
        </div>
    }
}

// ─────────────────────────────────────────────────────────────────────
// Лента
// ─────────────────────────────────────────────────────────────────────

fn render_timeline(events: Vec<TimelineEvent>, ctx: AppGlobalContext) -> AnyView {
    if events.is_empty() {
        return view! {
            <div style=format!("text-align: center; padding: 32px 0; color: {T_MUTED};
                                 font-size: 13px; font-style: italic;")>
                {"События по этому заказу не найдены"}
            </div>
        }
        .into_any();
    }

    let mut prev_jd: Option<i64> = None;
    let mut diffs: Vec<Option<i64>> = Vec::with_capacity(events.len());
    for e in &events {
        let d = prev_jd.map(|p| e.julian_day - p);
        diffs.push(d);
        prev_jd = Some(e.julian_day);
    }

    let last_idx = events.len() - 1;
    view! {
        <div style="display: flex; flex-direction: column; gap: 0;">
            {events.into_iter().enumerate().map(|(i, e)| {
                let d = diffs[i];
                render_event(e, ctx, d, i == 0, i == last_idx)
            }).collect_view()}
        </div>
    }
    .into_any()
}

// ─────────────────────────────────────────────────────────────────────
// Главный компонент
// ─────────────────────────────────────────────────────────────────────

#[component]
pub fn YmOrderFlowDashboard(
    #[prop(optional, into)] initial_order_id: Option<String>,
    #[prop(optional)] on_close: Option<Callback<()>>,
) -> impl IntoView {
    let (order_input, set_order_input) = signal(initial_order_id.clone().unwrap_or_default());
    let (flow_data, set_flow_data) = signal(None::<YmOrderFlowResponse>);
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(None::<String>);

    let do_search = move || {
        let order_id = order_input.get_untracked().trim().to_string();
        if order_id.is_empty() {
            return;
        }
        set_loading.set(true);
        set_error.set(None);
        set_flow_data.set(None);
        spawn_local(async move {
            match api::get_ym_order_flow(&order_id).await {
                Ok(d) => set_flow_data.set(Some(d)),
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    if let Some(order_id) = initial_order_id {
        if !order_id.trim().is_empty() {
            let do_search_initial = do_search.clone();
            Effect::new(move |run_once: Option<()>| {
                if run_once.is_none() {
                    do_search_initial();
                }
            });
        }
    }

    let on_search = {
        let ds = do_search.clone();
        Callback::new(move |_: ()| ds())
    };
    let on_key = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" {
            do_search();
        }
    };

    view! {
        <PageFrame page_id="d403_ym_order_flow" category="dashboard">
            <div class="page__header"
                 style="display: grid; grid-template-columns: 1fr auto 1fr; align-items: center;">
                <div class="page__header-left">
                    <span class="page__icon">{icon("list-ordered")}</span>
                    <h1 class="page__title">{"Вся история (YM)"}</h1>
                </div>
                <div style="display: flex; align-items: center; gap: 8px;">
                    <input
                        type="text"
                        placeholder="номер заказа"
                        style=format!("
                            padding: 6px 12px;
                            background: {SURFACE}; border: 1px solid {BORDER_STRONG};
                            border-radius: 6px; color: {T_PRIMARY}; font-size: 13px;
                            outline: none; width: 40ch;
                            font-family: ui-monospace, monospace;
                        ")
                        prop:value=order_input
                        on:input=move |ev| set_order_input.set(event_target_value(&ev))
                        on:keydown=on_key
                    />
                    <Button
                        appearance=ButtonAppearance::Primary
                        size=ButtonSize::Medium
                        on_click=move |_| on_search.run(())
                        disabled=Signal::derive(move || loading.get())
                    >
                        {move || if loading.get() { "Загрузка…" } else { "Обновить" }}
                    </Button>
                </div>
                <div class="page__header-right" style="justify-content: flex-end;">
                    {on_close.map(|cb| view! {
                        <Button
                            appearance=ButtonAppearance::Subtle
                            size=ButtonSize::Medium
                            on_click=move |_| cb.run(())
                        >
                            <span class="page-action-button__content">
                                <span class="page-action-button__icon page-action-button__icon--close">{icon("x")}</span>
                                <span class="page-action-button__text">"Закрыть"</span>
                            </span>
                        </Button>
                    })}
                </div>
            </div>

            <div class="page__content"
                 style="padding: 16px;
                         display: flex; flex-direction: column; align-items: center;">
                <div style="width: 100%; max-width: 1100px;
                             display: flex; flex-direction: column; gap: 14px;">

                    {move || error.get().map(|e| view! {
                        <div style="background: var(--color-error-50);
                                     border: 1px solid var(--color-error-100);
                                     border-radius: 6px; padding: 10px 14px;
                                     color: var(--color-error); font-size: 13px;">
                            {"⚠ "}{e}
                        </div>
                    })}

                    {move || {
                        let ctx = use_context::<AppGlobalContext>().expect("AppGlobalContext");
                        match flow_data.get() {
                            Some(data) => {
                                let events = build_events(&data);
                                let any_doc = data.order.is_some()
                                    || !data.realizations.is_empty()
                                    || !data.payments.is_empty()
                                    || !data.returns.is_empty();
                                if !any_doc {
                                    view! {
                                        <div style=format!("text-align:center;padding:60px 0;
                                                              color:{T_MUTED};font-size:14px;")>
                                            <div style="font-size:32px;margin-bottom:10px;">{"🔍"}</div>
                                            {"Документы по заказу «"}{data.order_no}{"» не найдены"}
                                        </div>
                                    }.into_any()
                                } else {
                                    let hero = render_hero(&data, &events);
                                    let timeline = render_timeline(events, ctx);
                                    view! {
                                        <div style="display: flex; flex-direction: column; gap: 14px;">
                                            {hero}
                                            {timeline}
                                        </div>
                                    }.into_any()
                                }
                            }
                            None => if loading.get() {
                                view! {
                                    <div style=format!("text-align:center;padding:60px 0;
                                                          color:{T_MUTED};font-size:14px;")>
                                        {"Загружаем данные…"}
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div style=format!("
                                        background: {SURFACE_ALT}; border: 1px dashed {BORDER};
                                        border-radius: 10px; padding: 60px 0; text-align: center;
                                    ")>
                                        <div style="font-size:36px;margin-bottom:12px;">{"📋"}</div>
                                        <div style=format!("font-size:14px;color:{T_SECONDARY};
                                                              margin-bottom:4px;")>
                                            {"Введите номер заказа и нажмите «Обновить»"}
                                        </div>
                                        <div style=format!("font-size:12px;color:{T_MUTED};")>
                                            {"Лента событий: заказ · реализация · возврат · платёж"}
                                        </div>
                                    </div>
                                }.into_any()
                            }
                        }
                    }}
                </div>
            </div>
        </PageFrame>
    }
}
