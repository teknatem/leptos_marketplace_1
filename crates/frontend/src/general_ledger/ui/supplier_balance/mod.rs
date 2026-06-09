//! Отчёт «Баланс к перечислению поставщику (YM)».
//!
//! Показывает движение денежного счёта расчётов 7609 в контуре entity=ym за
//! период: Входящее сальдо → +Начислено → −Удержано → −Перечислено → Исходящее
//! (= доступно к перечислению), плюс справочно баланс кошелька баллов/промо (76YB).

use crate::general_ledger::api::fetch_supplier_balance;
use crate::shared::api_utils::api_base;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_LIST;
use chrono::{Datelike, Utc};
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::common::AggregateId;
use contracts::general_ledger::{SupplierBalanceQuery, SupplierBalanceResponse};
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

#[derive(Clone, Debug)]
struct CabinetOption {
    id: String,
    label: String,
}

async fn load_cabinet_options() -> Vec<CabinetOption> {
    let url = format!("{}/api/connection_mp", api_base());
    let Ok(resp) = Request::get(&url).send().await else {
        return vec![];
    };
    if !resp.ok() {
        return vec![];
    }
    let Ok(data) = resp.json::<Vec<ConnectionMP>>().await else {
        return vec![];
    };
    let mut opts: Vec<CabinetOption> = data
        .into_iter()
        .map(|c| {
            let label = if c.base.description.trim().is_empty() {
                c.base.code.clone()
            } else {
                c.base.description.clone()
            };
            CabinetOption {
                id: c.base.id.as_string(),
                label,
            }
        })
        .collect();
    opts.sort_by(|a, b| a.label.cmp(&b.label));
    opts
}

fn default_month_range() -> (String, String) {
    let now = Utc::now().date_naive();
    let (y, m) = (now.year(), now.month());
    let first = chrono::NaiveDate::from_ymd_opt(y, m, 1).expect("first");
    let last = if m == 12 {
        chrono::NaiveDate::from_ymd_opt(y + 1, 1, 1)
    } else {
        chrono::NaiveDate::from_ymd_opt(y, m + 1, 1)
    }
    .map(|d| d - chrono::Duration::days(1))
    .expect("last");
    (
        first.format("%Y-%m-%d").to_string(),
        last.format("%Y-%m-%d").to_string(),
    )
}

/// Денежный формат с разделением тысяч и 2 знаками.
fn money(value: f64) -> String {
    let negative = value < 0.0;
    let cents = (value.abs() * 100.0).round() as u64;
    let rub = cents / 100;
    let frac = cents % 100;
    let mut digits = rub.to_string();
    let mut grouped = String::new();
    while digits.len() > 3 {
        let split = digits.len() - 3;
        grouped = format!(" {}{}", &digits[split..], grouped);
        digits.truncate(split);
    }
    let int_part = format!("{digits}{grouped}");
    let sign = if negative { "−" } else { "" };
    format!("{sign}{int_part},{frac:02} ₽")
}

#[component]
fn BalanceRow(label: &'static str, value: f64, strong: bool) -> impl IntoView {
    let value_style = if strong {
        "font-weight:700;font-size:var(--font-size-lg);"
    } else {
        ""
    };
    let value_color = if value < 0.0 {
        "color:var(--color-error,#c0392b);"
    } else {
        ""
    };
    view! {
        <div style="display:flex;justify-content:space-between;align-items:baseline;padding:6px 0;border-bottom:1px dashed var(--color-neutral-stroke-2,#e5e5e5);">
            <span style="color:var(--color-text-secondary);">{label}</span>
            <span style=format!("{value_style}{value_color}font-variant-numeric:tabular-nums;")>
                {money(value)}
            </span>
        </div>
    }
}

#[component]
pub fn SupplierBalancePage() -> impl IntoView {
    let (df, dt) = default_month_range();
    let date_from = RwSignal::new(df);
    let date_to = RwSignal::new(dt);
    let cabinet = RwSignal::new(String::new());
    let cabinet_options = RwSignal::new(Vec::<CabinetOption>::new());

    let (result, set_result) = signal::<Option<SupplierBalanceResponse>>(None);
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);

    Effect::new(move |_| {
        spawn_local(async move {
            cabinet_options.set(load_cabinet_options().await);
        });
    });

    let run = move || {
        let query = SupplierBalanceQuery {
            date_from: date_from.get_untracked(),
            date_to: date_to.get_untracked(),
            connection_mp_ref: {
                let c = cabinet.get_untracked();
                (!c.trim().is_empty()).then_some(c)
            },
        };
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            match fetch_supplier_balance(&query).await {
                Ok(r) => set_result.set(Some(r)),
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    // Загрузить при первом рендере.
    Effect::new(move |prev: Option<()>| {
        if prev.is_none() {
            run();
        }
    });

    view! {
        <PageFrame page_id="supplier_balance--list" category=PAGE_CAT_LIST>
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Баланс к перечислению (YM)"</h1>
                </div>
            </div>

            <div class="page__content">
                <CardAnimated delay_ms=0 nav_id="supplier_balance_filters">
                    <Flex gap=FlexGap::Medium align=FlexAlign::End style="flex-wrap:wrap;">
                        <div style="width:170px;">
                            <Flex vertical=true gap=FlexGap::Small>
                                <Label>"Дата с"</Label>
                                <input
                                    class="form__input"
                                    type="date"
                                    prop:value=move || date_from.get()
                                    on:change=move |ev| date_from.set(event_target_value(&ev))
                                />
                            </Flex>
                        </div>
                        <div style="width:170px;">
                            <Flex vertical=true gap=FlexGap::Small>
                                <Label>"Дата по"</Label>
                                <input
                                    class="form__input"
                                    type="date"
                                    prop:value=move || date_to.get()
                                    on:change=move |ev| date_to.set(event_target_value(&ev))
                                />
                            </Flex>
                        </div>
                        <div style="width:320px;">
                            <Flex vertical=true gap=FlexGap::Small>
                                <Label>"Кабинет"</Label>
                                <select
                                    class="form__input"
                                    prop:value=move || cabinet.get()
                                    on:change=move |ev| cabinet.set(event_target_value(&ev))
                                >
                                    <option value="">"Все кабинеты"</option>
                                    {move || cabinet_options.get().into_iter().map(|c| {
                                        view! { <option value=c.id.clone()>{c.label.clone()}</option> }
                                    }).collect::<Vec<_>>()}
                                </select>
                            </Flex>
                        </div>
                        <Button appearance=ButtonAppearance::Primary on_click=move |_| run()>
                            "Рассчитать"
                        </Button>
                    </Flex>
                </CardAnimated>

                {move || {
                    if loading.get() {
                        return view! {
                            <div class="page__placeholder"><Spinner /> " Расчёт баланса..."</div>
                        }.into_any();
                    }
                    if let Some(err) = error.get() {
                        return view! {
                            <div class="alert alert--error">{format!("Ошибка: {err}")}</div>
                        }.into_any();
                    }
                    let Some(r) = result.get() else {
                        return view! { <></> }.into_any();
                    };
                    view! {
                        <div style="max-width:560px;margin-top:var(--spacing-md);">
                            <CardAnimated delay_ms=40 nav_id="supplier_balance_card">
                                <h4 class="details-section__title">
                                    {format!("Счёт {} · контур {}", r.account, r.entity)}
                                </h4>
                                <BalanceRow label="Входящее сальдо" value=r.opening_balance strong=false />
                                <BalanceRow label="+ Начислено (выручка/доходы)" value=r.accrued strong=false />
                                <BalanceRow label="− Удержано (комиссии/услуги/возвраты)" value=r.deductions strong=false />
                                <BalanceRow label="− Перечислено на расчётный счёт" value=-r.settled strong=false />
                                <div style="height:6px;"></div>
                                <BalanceRow label="= Доступно к перечислению (исходящее сальдо)" value=r.closing_balance strong=true />
                                <div style="margin-top:var(--spacing-md);padding-top:var(--spacing-sm);border-top:1px solid var(--color-neutral-stroke-2,#e5e5e5);">
                                    <BalanceRow label="Справочно: баланс баллов/промо (76YB)" value=r.points_balance strong=false />
                                </div>
                            </CardAnimated>
                        </div>
                    }.into_any()
                }}
            </div>
        </PageFrame>
    }
}
