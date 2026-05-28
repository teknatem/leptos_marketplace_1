use crate::dashboards::d402_wb_order_flow::api;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use contracts::dashboards::d402_wb_order_flow::WbOrderFlowResponse;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

// ── Палитра ───────────────────────────────────────────────────────────
// Используем CSS-переменные дизайн-системы там, где возможно (адаптация
// под светлую/тёмную темы). Акценты этапов — фиксированные оттенки,
// которые остаются читаемыми на любом фоне.

const SURFACE: &str = "var(--color-surface)";
const SURFACE_ALT: &str = "var(--color-surface-alt, var(--color-bg-secondary))";
const BORDER: &str = "var(--color-border-light, var(--color-border))";
const BORDER_STRONG: &str = "var(--color-border)";

const T_PRIMARY: &str = "var(--color-text-primary)";
const T_SECONDARY: &str = "var(--color-text-secondary)";
const T_MUTED: &str = "var(--color-text-tertiary)";

// Акценты этапов: насыщенные средние тона, читаются и на белом, и на тёмном.
const CA: &str = "#7c3aed"; // реклама (violet-600)
const CO: &str = "#16a34a"; // заказ   (green-600)
const CS: &str = "#2563eb"; // поставка (blue-600)
const CP: &str = "#15803d"; // продажа / доход (green-700)
const CE: &str = "#dc2626"; // расход / возврат (red-600)
const CN: &str = "#94a3b8"; // финансовый отчёт — нейтральный серый (slate-400)
const CW: &str = "#f59e0b"; // заявка на возврат — янтарный (amber-500)

// Прозрачные подложки под точкой/бордюром карточки — используем rgba,
// чтобы акцент мягко проявлялся на любом фоне.
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

/// Возвращает (julian-day, текст `DD.MM.YYYY`) для строки в формате
/// `DD.MM.YYYY` или `YYYY-MM-DD`.
fn parse_event_date(s: &str) -> Option<(i64, String)> {
    if s.is_empty() {
        return None;
    }
    if let Some(j) = jdn_from_dmy(s) {
        return Some((j, s.to_string()));
    }
    let p: Vec<&str> = s.split('-').collect();
    if p.len() == 3 && p[0].len() == 4 {
        let formatted = format!("{}.{}.{}", p[2], p[1], p[0]);
        return jdn_from_dmy(&formatted).map(|j| (j, formatted));
    }
    None
}

fn advert_status_text(s: i32) -> Option<&'static str> {
    match s {
        4 => Some("Готова"),
        7 => Some("Завершена"),
        9 => Some("Активна"),
        11 => Some("Пауза"),
        _ => None,
    }
}

fn claim_status_text(s: Option<i32>) -> &'static str {
    match s {
        Some(1) => "Открыта",
        Some(2) => "На рассмотрении",
        Some(3) => "Одобрена",
        Some(4) => "Отклонена",
        Some(5) => "Закрыта",
        _ => "Статус неизвестен",
    }
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

/// Строка `Лейбл: 1500 ₽` без приведения знака. Заметные, крупные лейблы.
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

/// Знаковая денежная строка: отрицательные → красным с префиксом `−`,
/// положительные — обычным цветом без знака.
fn detail_row_money(label: &'static str, value: f64) -> AnyView {
    let (text, color) = if value < -0.001 {
        (format!("−{}", rub(value.abs())), CE.to_string())
    } else if value > 0.001 {
        (rub(value), T_PRIMARY.to_string())
    } else {
        (rub(0.0), T_MUTED.to_string())
    };
    detail_row(label, text, color)
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
    /// > 0 — доход, < 0 — расход, None — событие без денежного движения.
    amount: Option<f64>,
    links: Vec<(String, &'static str)>,
}

fn build_events(data: &WbOrderFlowResponse) -> Vec<TimelineEvent> {
    let mut events: Vec<TimelineEvent> = Vec::new();

    // 1. Реклама
    for adv in &data.advert_campaigns {
        if let Some((jd, dt)) = parse_event_date(&adv.document_date) {
            let mut title = adv.campaign_name.clone().unwrap_or_default();
            if title.is_empty() {
                title = if adv.advert_id > 0 {
                    format!("Кампания #{}", adv.advert_id)
                } else {
                    "Рекламная кампания".to_string()
                };
            }
            let subtitle = adv
                .campaign_status
                .and_then(advert_status_text)
                .map(|s| s.to_string());
            let mut links: Vec<(String, &'static str)> = vec![(
                format!("a026_wb_advert_daily_details_{}", adv.registrator_ref),
                "День",
            )];
            if adv.advert_id > 0 {
                links.push((
                    format!("a030_wb_advert_campaign_details_{}", adv.advert_id),
                    "Кампания",
                ));
            }
            events.push(TimelineEvent {
                sort_key: jd * 10,
                julian_day: jd,
                date_text: dt,
                accent: CA,
                type_label: "Реклама",
                title,
                subtitle,
                details: vec![],
                amount: Some(-adv.allocated_cost.abs()),
                links,
            });
        }
    }

    // 2. Заказ — price_with_disc, spp, finished_price
    if let Some(o) = &data.order {
        if let Some(date) = &o.document_date {
            if let Some((jd, dt)) = parse_event_date(date) {
                let mut title = o.supplier_article.clone().unwrap_or_default();
                if let Some(q) = o.qty {
                    if !title.is_empty() {
                        title.push_str(&format!(" · {:.0} шт", q));
                    } else {
                        title = format!("{:.0} шт", q);
                    }
                }
                let mut details: Vec<AnyView> = Vec::new();
                if let Some(v) = o.price_with_disc {
                    details.push(detail_row("Цена со скидкой", rub(v), T_PRIMARY.to_string()));
                }
                if let Some(v) = o.spp {
                    details.push(detail_row("СПП", rub(v), T_PRIMARY.to_string()));
                }
                if let Some(v) = o.finished_price {
                    details.push(detail_row("Цена клиента", rub(v), T_PRIMARY.to_string()));
                }
                events.push(TimelineEvent {
                    sort_key: jd * 10 + 1,
                    julian_day: jd,
                    date_text: dt,
                    accent: CO,
                    type_label: "Заказ",
                    title,
                    subtitle: None,
                    details,
                    amount: None,
                    links: vec![(format!("a015_wb_orders_details_{}", o.id), "Заказ")],
                });
            }
        }
    }

    // 3. Поставка — одно событие, имя + дата закрытия в подзаголовке
    if let Some(s) = &data.supply {
        if let Some(date) = &s.created_at_wb {
            if let Some((jd, dt)) = parse_event_date(date) {
                let title = s.supply_id.clone();
                let mut sub_parts: Vec<String> = Vec::new();
                if let Some(name) = &s.supply_name {
                    if !name.is_empty() {
                        sub_parts.push(name.clone());
                    }
                }
                if let Some(closed) = &s.closed_at_wb {
                    if !closed.is_empty() {
                        let closed_text = parse_event_date(closed)
                            .map(|p| p.1)
                            .unwrap_or_else(|| closed.clone());
                        sub_parts.push(format!("закрыта {}", closed_text));
                    }
                }
                let subtitle = (!sub_parts.is_empty()).then(|| sub_parts.join(" · "));
                events.push(TimelineEvent {
                    sort_key: jd * 10 + 2,
                    julian_day: jd,
                    date_text: dt,
                    accent: CS,
                    type_label: "Поставка",
                    title,
                    subtitle,
                    details: vec![],
                    amount: None,
                    links: vec![(format!("a029_wb_supply_details_{}", s.id), "Поставка")],
                });
            }
        }
    }

    // 4. Продажа / Возврат (a012). Возврат — красный, со ссылкой.
    for s in &data.sales {
        if let Some((jd, dt)) = parse_event_date(&s.sale_dt) {
            let (accent, type_label, link_label) = if s.is_customer_return {
                (CE, "Возврат", "Возврат")
            } else {
                (CP, "Продажа", "Продажа")
            };
            let mut title = s.status_norm.clone();
            if title.is_empty() {
                title = s.event_type.clone();
            }

            // Поля документа — берём _fact если документ зафиксирован, иначе _plan.
            let payout = if s.is_fact {
                s.supplier_payout_fact
            } else {
                s.supplier_payout_plan
            };
            let profit = if s.is_fact {
                s.profit_fact
            } else {
                s.profit_plan
            };

            let mut details: Vec<AnyView> = Vec::new();
            if let Some(v) = s.finished_price {
                details.push(detail_row_money("Цена клиента", v));
            }
            if let Some(v) = s.dealer_price_ut {
                details.push(detail_row_money("Дилерская цена", v));
            }
            if let Some(v) = s.cost_of_production {
                details.push(detail_row_money("Себестоимость", v));
            }
            if let Some(v) = payout {
                let lbl = if s.is_fact {
                    "Выплата WB (факт)"
                } else {
                    "Выплата WB"
                };
                details.push(detail_row_money(lbl, v));
            }
            if let Some(v) = profit {
                let lbl = if s.is_fact {
                    "Прибыль (факт)"
                } else {
                    "Прибыль"
                };
                details.push(detail_row_money(lbl, v));
            }

            events.push(TimelineEvent {
                sort_key: jd * 10 + 4,
                julian_day: jd,
                date_text: dt,
                accent,
                type_label,
                title,
                subtitle: None,
                details,
                amount: None,
                links: vec![(format!("a012_wb_sales_details_{}", s.id), link_label)],
            });
        }
    }

    // 5. Финансовый отчёт (p903) — каждая строка как событие, со ссылкой.
    for r in &data.p903_rows {
        if let Some((jd, dt)) = parse_event_date(&r.rr_dt) {
            let title = r.supplier_oper_name.clone().unwrap_or_default();
            let oper_l = title.to_lowercase();
            let is_return = oper_l.contains("возврат");

            let mut amount = r.ppvz_for_pay.unwrap_or(0.0)
                - r.rebill_logistic_cost.unwrap_or(0.0).abs()
                - r.storage_fee.unwrap_or(0.0).abs()
                - r.penalty.unwrap_or(0.0).abs()
                + r.additional_payment.unwrap_or(0.0);

            if is_return && amount > 0.001 {
                amount = -amount;
            }

            events.push(TimelineEvent {
                sort_key: jd * 10 + 5,
                julian_day: jd,
                date_text: dt,
                accent: CN,
                type_label: "Фин. отчёт",
                title,
                subtitle: None,
                details: vec![],
                amount: if amount.abs() < 0.001 {
                    None
                } else {
                    Some(amount)
                },
                links: vec![(
                    format!("p903_wb_finance_report_details_id_{}", r.id),
                    "Фин. отчёт",
                )],
            });
        }
    }

    // 6. Заявки на возврат (a032) — событие без денег, цена в деталях.
    for c in &data.claims {
        if let Some((jd, dt)) = parse_event_date(&c.dt) {
            let status_text = claim_status_text(c.status).to_string();
            let title = if c.is_archive {
                format!("{} · архив", status_text)
            } else {
                status_text
            };

            let subtitle = c
                .user_comment
                .as_deref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| {
                    let cnt = s.chars().count();
                    if cnt > 100 {
                        let head: String = s.chars().take(100).collect();
                        format!("{head}…")
                    } else {
                        s.to_string()
                    }
                });

            let mut details: Vec<AnyView> = Vec::new();
            if let Some(p) = c.price {
                details.push(detail_row("Цена возврата", rub(p), T_PRIMARY.to_string()));
            }
            details.push(detail_row(
                "Заявка №",
                c.claim_id.clone(),
                T_PRIMARY.to_string(),
            ));
            if let Some(upd) = c.dt_update.as_deref().filter(|s| !s.is_empty()) {
                details.push(detail_row(
                    "Обновлена",
                    upd.to_string(),
                    T_SECONDARY.to_string(),
                ));
            }

            events.push(TimelineEvent {
                sort_key: jd * 10 + 6,
                julian_day: jd,
                date_text: dt,
                accent: CW,
                type_label: "Заявка на возврат",
                title,
                subtitle,
                details,
                amount: None,
                links: vec![(format!("a032_wb_returns_claims_details_{}", c.id), "Заявка")],
            });
        }
    }

    events.sort_by_key(|e| e.sort_key);
    events
}

// ─────────────────────────────────────────────────────────────────────
// Карточка-шапка: товар + базовая номенклатура + дилерская цена + итоги
// ─────────────────────────────────────────────────────────────────────

fn render_hero(data: &WbOrderFlowResponse, events: &[TimelineEvent]) -> AnyView {
    let sale = data
        .sales
        .iter()
        .find(|s| !s.is_customer_return)
        .or_else(|| data.sales.first());
    let order = data.order.as_ref();

    let art = sale
        .map(|s| s.supplier_article.clone())
        .filter(|s| !s.is_empty())
        .or_else(|| order.and_then(|o| o.supplier_article.clone()))
        .unwrap_or_default();

    let name = data
        .nomenclature_description
        .clone()
        .filter(|s| !s.is_empty())
        .or_else(|| sale.map(|s| s.name.clone()).filter(|s| !s.is_empty()))
        .or_else(|| order.and_then(|o| o.subject.clone()))
        .unwrap_or_default();

    let base_nom = data
        .base_nomenclature_description
        .clone()
        .filter(|s| !s.is_empty());
    let brand = order.and_then(|o| o.brand.clone()).unwrap_or_default();
    let nm_id = order.and_then(|o| o.nm_id);
    let srid = data.srid.clone();

    let dealer = order
        .and_then(|o| o.dealer_price_ut)
        .or_else(|| sale.and_then(|s| s.dealer_price_ut));

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
            <div style="flex: 1; min-width: 280px;">
                // Артикул + наименование — крупным шрифтом
                <div style="display: flex; gap: 14px; align-items: baseline; flex-wrap: wrap;">
                    {(!art.is_empty()).then(|| view! {
                        <span style=format!("font-size: 18px; font-weight: 700; color: {T_PRIMARY};")>
                            {art}
                        </span>
                    })}
                    {(!name.is_empty()).then(|| view! {
                        <span style=format!("font-size: 16px; color: {T_PRIMARY}; font-weight: 500;")>
                            {name}
                        </span>
                    })}
                </div>

                // Базовая номенклатура
                {base_nom.map(|bn| view! {
                    <div style="display: flex; gap: 8px; align-items: baseline; margin-top: 6px;">
                        <span style=format!("font-size: 13px; color: {T_SECONDARY}; font-weight: 500;")>
                            {"Базовая:"}
                        </span>
                        <span style=format!("font-size: 14px; color: {T_PRIMARY}; font-weight: 600;")>
                            {bn}
                        </span>
                    </div>
                })}

                // Дилерская цена УТ
                {dealer.map(|v| view! {
                    <div style="display: flex; gap: 8px; align-items: baseline; margin-top: 6px;">
                        <span style=format!("font-size: 13px; color: {T_SECONDARY}; font-weight: 500;")>
                            {"Дилерская цена УТ:"}
                        </span>
                        <span style=format!("font-size: 14px; color: {T_PRIMARY}; font-weight: 700;
                                             font-variant-numeric: tabular-nums;")>{rub(v)}</span>
                    </div>
                })}

                // Бренд / nm / srid
                <div style="display: flex; gap: 12px; align-items: baseline; flex-wrap: wrap; margin-top: 8px;">
                    {(!brand.is_empty()).then(|| view! {
                        <span style=format!("font-size: 12px; color: {T_MUTED};")>{brand}</span>
                    })}
                    {nm_id.map(|n| view! {
                        <span style=format!("font-size: 12px; color: {T_MUTED}; font-variant-numeric: tabular-nums;")>
                            {"nm "}{n.to_string()}
                        </span>
                    })}
                    <span style=format!("font-size: 12px; color: {T_MUTED}; font-family: ui-monospace, monospace;")>
                        {"srid "}{srid}
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
    }.into_any()
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
            // Дата + дельта дней
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

            // Маркер
            <div style="display: flex; flex-direction: column; align-items: center;">
                <div style=format!("width: 2px; height: 13px; background: {line_top_color};")></div>
                <div style=format!("width: 12px; height: 12px; border-radius: 50%; background: {accent};
                                     flex-shrink: 0;")></div>
                <div style=format!("width: 2px; flex: 1; min-height: 6px; background: {line_bot_color};")></div>
            </div>

            // Карточка
            <div style=format!("background: {SURFACE}; border: 1px solid {BORDER};
                                 border-left: 3px solid {accent}; border-radius: 6px;
                                 padding: 8px 14px; margin-bottom: 5px;
                                 box-shadow: var(--shadow-sm);")>
                // Верхняя строка: chip + title слева, links + amount справа
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

                // Подзаголовок (если есть)
                {subtitle.map(|s| view! {
                    <div style=format!("font-size: 12px; color: {T_SECONDARY}; margin-top: 3px;")>
                        {s}
                    </div>
                })}

                // Детали (горизонтально, на одной строке с переносом)
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
pub fn WbOrderFlowDashboard(
    #[prop(optional, into)] initial_srid: Option<String>,
    #[prop(optional)] on_close: Option<Callback<()>>,
) -> impl IntoView {
    let (srid_input, set_srid_input) = signal(initial_srid.clone().unwrap_or_default());
    let (flow_data, set_flow_data) = signal(None::<WbOrderFlowResponse>);
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(None::<String>);

    let do_search = move || {
        let srid = srid_input.get_untracked().trim().to_string();
        if srid.is_empty() {
            return;
        }
        set_loading.set(true);
        set_error.set(None);
        set_flow_data.set(None);
        spawn_local(async move {
            match api::get_order_flow(&srid).await {
                Ok(d) => set_flow_data.set(Some(d)),
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    if let Some(srid) = initial_srid {
        if !srid.trim().is_empty() {
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
        <PageFrame page_id="d402_wb_order_flow" category="dashboard">
            // Полноценный заголовок страницы
            <div class="page__header"
                 style="display: grid; grid-template-columns: 1fr auto 1fr; align-items: center;">
                <div class="page__header-left">
                    <span class="page__icon">{icon("activity")}</span>
                    <h1 class="page__title">{"Схема заказа WB"}</h1>
                </div>
                // Центр: поле srid + кнопка Обновить
                <div style="display: flex; align-items: center; gap: 8px;">
                    <input
                        type="text"
                        placeholder="srid / document_no"
                        style=format!("
                            padding: 6px 12px;
                            background: {SURFACE}; border: 1px solid {BORDER_STRONG};
                            border-radius: 6px; color: {T_PRIMARY}; font-size: 13px;
                            outline: none; width: 60ch;
                            font-family: ui-monospace, monospace;
                        ")
                        prop:value=srid_input
                        on:input=move |ev| set_srid_input.set(event_target_value(&ev))
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
                // Справа: кнопка Закрыть
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

                    // Ошибка
                    {move || error.get().map(|e| view! {
                        <div style="background: var(--color-error-50);
                                     border: 1px solid var(--color-error-100);
                                     border-radius: 6px; padding: 10px 14px;
                                     color: var(--color-error); font-size: 13px;">
                            {"⚠ "}{e}
                        </div>
                    })}

                    // Контент
                    {move || {
                        let ctx = use_context::<AppGlobalContext>().expect("AppGlobalContext");
                        match flow_data.get() {
                            Some(data) => {
                                let events = build_events(&data);
                                let any_doc = data.order.is_some()
                                    || data.supply.is_some()
                                    || !data.sales.is_empty()
                                    || !data.advert_campaigns.is_empty()
                                    || !data.p903_rows.is_empty();
                                if !any_doc {
                                    view! {
                                        <div style=format!("text-align:center;padding:60px 0;
                                                              color:{T_MUTED};font-size:14px;")>
                                            <div style="font-size:32px;margin-bottom:10px;">{"🔍"}</div>
                                            {"Документы с srid «"}{data.srid}{"» не найдены"}
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
                                            {"Введите srid заказа и нажмите «Показать»"}
                                        </div>
                                        <div style=format!("font-size:12px;color:{T_MUTED};")>
                                            {"Лента событий: реклама · заказ · поставка · продажа · возврат · фин. отчёт"}
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
