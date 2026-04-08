use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::components::date_range_picker::DateRangePicker;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::table::{TableCellMoney, TableCrosshairHighlight};
use crate::shared::date_utils::{format_date, format_datetime, format_datetime_space};
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator};
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_LIST;
use crate::shared::table_utils::{init_column_resize, was_just_resizing};
use chrono::{Datelike, Utc};
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::common::AggregateId;
use contracts::general_ledger::GeneralLedgerEntryDto;
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

use crate::general_ledger::api::{fetch_general_ledger, GeneralLedgerListQuery};

const TABLE_ID: &str = "general-ledger-table";
const COLUMN_WIDTHS_KEY: &str = "general_ledger_column_widths";

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

static ACCOUNT_OPTIONS: &[(&str, &str)] = &[
    ("", "Все"),
    ("62", "62 — Расчёты с покупателями"),
    ("44", "44 — Расходы на продажу"),
    ("4401", "4401 — Расходы МП"),
    ("41", "41 — Товары"),
    ("90", "90 — Продажи"),
    ("9001", "9001 — Выручка"),
    ("9002", "9002 — Себестоимость"),
    ("91", "91 — Прочие доходы/расходы"),
    ("76", "76 — Расчёты с прочими"),
    ("7609", "7609 — Расчёты с МП"),
];

static TURNOVER_OPTIONS: &[(&str, &str)] = &[
    ("", "Все"),
    ("qty_ordered", "qty_ordered — Количество заказано"),
    ("qty_sold", "qty_sold — Количество продано"),
    ("qty_returned", "qty_returned — Количество возвращено"),
    (
        "customer_revenue",
        "customer_revenue — Выручка от покупателя",
    ),
    ("customer_return", "customer_return — Возврат покупателя"),
    ("seller_payout", "seller_payout — Выплата продавцу"),
    ("mp_commission", "mp_commission — Комиссия МП"),
    (
        "mp_commission_adjustment",
        "mp_commission_adjustment — Корректировка комиссии",
    ),
    ("mp_acquiring", "mp_acquiring — Эквайринг МП"),
    ("mp_logistics", "mp_logistics — Логистика МП"),
    ("mp_storage", "mp_storage — Хранение МП"),
    ("mp_penalty", "mp_penalty — Штраф МП"),
    ("mp_penalty_storno", "mp_penalty_storno — Штраф МП (сторно)"),
    (
        "mp_rebill_logistic_cost",
        "mp_rebill_logistic_cost — Rebill logistics",
    ),
    (
        "mp_ppvz_reward",
        "mp_ppvz_reward — Возмещение за выдачу и возврат товаров на ПВЗ",
    ),
    ("item_cost", "item_cost — Себестоимость"),
    ("spp_discount", "spp_discount — Скидка СПП"),
    (
        "spp_discount_storno",
        "spp_discount_storno — Скидка СПП (сторно возврат)",
    ),
    ("wb_coinvestment", "wb_coinvestment — Соинвестирование WB"),
    (
        "wb_coinvestment_storno",
        "wb_coinvestment_storno — Соинвестирование WB (сторно возврат)",
    ),
    (
        "advertising_allocated",
        "advertising_allocated — Реклама (распределённая)",
    ),
    ("advertising", "advertising — Реклама"),
    ("acceptance", "acceptance — Приёмка"),
    (
        "adjustment_income",
        "adjustment_income — Корректировка (доход)",
    ),
    (
        "voluntary_return_compensation",
        "voluntary_return_compensation — Компенсация добр. возврата",
    ),
    (
        "adjustment_expense",
        "adjustment_expense — Корректировка (расход)",
    ),
    (
        "commission_percent",
        "commission_percent — Процент комиссии",
    ),
];

#[derive(Clone, Debug)]
struct GeneralLedgerListState {
    entries: Vec<GeneralLedgerEntryDto>,
    date_from: String,
    date_to: String,
    registrator_ref: String,
    registrator_type: String,
    layer: String,
    turnover_code: String,
    connection_mp_ref: String,
    debit_account: String,
    credit_account: String,
    sort_field: String,
    sort_ascending: bool,
    page: usize,
    page_size: usize,
    total_count: usize,
    total_pages: usize,
    is_loaded: bool,
}

impl Default for GeneralLedgerListState {
    fn default() -> Self {
        let now = Utc::now().date_naive();
        let year = now.year();
        let month = now.month();
        let month_start =
            chrono::NaiveDate::from_ymd_opt(year, month, 1).expect("invalid month start");
        let month_end = if month == 12 {
            chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
                .map(|d| d - chrono::Duration::days(1))
                .expect("invalid month end")
        } else {
            chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
                .map(|d| d - chrono::Duration::days(1))
                .expect("invalid month end")
        };

        Self {
            entries: Vec::new(),
            date_from: month_start.format("%Y-%m-%d").to_string(),
            date_to: month_end.format("%Y-%m-%d").to_string(),
            registrator_ref: String::new(),
            registrator_type: String::new(),
            layer: String::new(),
            turnover_code: String::new(),
            connection_mp_ref: String::new(),
            debit_account: String::new(),
            credit_account: String::new(),
            sort_field: "entry_date".to_string(),
            sort_ascending: false,
            page: 0,
            page_size: 100,
            total_count: 0,
            total_pages: 0,
            is_loaded: false,
        }
    }
}

fn short_id(value: &str) -> &str {
    if value.len() >= 8 {
        &value[..8]
    } else {
        value
    }
}

fn p903_tab_key_from_ref(value: &str) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(format!(
            "p903_wb_finance_report_details_id_{}",
            urlencoding::encode(value)
        ))
    }
}

fn p903_tab_label(value: &str) -> String {
    format!("WB Finance {}", short_id(value))
}

fn registrator_tab_key(registrator_type: &str, id: &str) -> Option<String> {
    match registrator_type {
        "a012_wb_sales" => Some(format!("a012_wb_sales_details_{id}")),
        "a013_ym_order" => Some(format!("a013_ym_order_details_{id}")),
        "a014_ozon_transactions" => Some(format!("a014_ozon_transactions_details_{id}")),
        "a015_wb_orders" => Some(format!("a015_wb_orders_details_{id}")),
        "a016_ym_returns" => Some(format!("a016_ym_returns_details_{id}")),
        "a026_wb_advert_daily" => Some(format!("a026_wb_advert_daily_details_{id}")),
        "a021_production_output" => Some(format!("a021_production_output_details_{id}")),
        "a022_kit_variant" => Some(format!("a022_kit_variant_details_{id}")),
        "a023_purchase_of_goods" => Some(format!("a023_purchase_of_goods_details_{id}")),
        "p903_wb_finance_report" => p903_tab_key_from_ref(id),
        _ => None,
    }
}

fn registrator_tab_label(registrator_type: &str, id: &str) -> String {
    match registrator_type {
        "a012_wb_sales" => format!("WB Sale {}", short_id(id)),
        "a013_ym_order" => format!("YM Order {}", short_id(id)),
        "a014_ozon_transactions" => format!("OZON Transaction {}", short_id(id)),
        "a015_wb_orders" => format!("WB Order {}", short_id(id)),
        "a016_ym_returns" => format!("YM Return {}", short_id(id)),
        "a026_wb_advert_daily" => format!("WB Ads {}", short_id(id)),
        "p903_wb_finance_report" => p903_tab_label(id),
        _ => format!("{registrator_type} • {}", short_id(id)),
    }
}

fn registrator_display(registrator_ref: &str, registrator_type: &str) -> String {
    format!("{registrator_type} • {}", short_id(registrator_ref))
}

fn format_journal_datetime(value: &str) -> String {
    if value.contains('T') {
        format_datetime(value)
    } else if value.contains(' ') {
        format_datetime_space(value)
    } else {
        format_date(value)
    }
}

fn format_optional_number(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "—".to_string())
}

#[component]
pub fn GeneralLedgerPage() -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let state = RwSignal::new(GeneralLedgerListState::default());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (is_filter_expanded, set_is_filter_expanded) = signal(false);

    let registrator_type_input = RwSignal::new(state.get_untracked().registrator_type.clone());
    let registrator_ref_input = RwSignal::new(state.get_untracked().registrator_ref.clone());
    let layer_input = RwSignal::new(state.get_untracked().layer.clone());
    let turnover_code_input = RwSignal::new(state.get_untracked().turnover_code.clone());
    let connection_mp_ref_input = RwSignal::new(state.get_untracked().connection_mp_ref.clone());
    let debit_account_input = RwSignal::new(state.get_untracked().debit_account.clone());
    let credit_account_input = RwSignal::new(state.get_untracked().credit_account.clone());

    let cabinet_options = RwSignal::new(Vec::<CabinetOption>::new());
    Effect::new(move |_| {
        spawn_local(async move {
            let opts = load_cabinet_options().await;
            cabinet_options.set(opts);
        });
    });

    let open_detail = move |id: String| {
        tabs_store.open_tab(
            &format!("general_ledger_details_{id}"),
            &format!("Главная книга • {}", short_id(&id)),
        );
    };

    let open_registrator = move |registrator_type: String, registrator_ref: String| {
        if let Some(key) = registrator_tab_key(&registrator_type, &registrator_ref) {
            tabs_store.open_tab(
                &key,
                &registrator_tab_label(&registrator_type, &registrator_ref),
            );
        }
    };

    let load_entries = move || {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let query = state.with_untracked(|s| GeneralLedgerListQuery {
                date_from: (!s.date_from.is_empty()).then(|| s.date_from.clone()),
                date_to: (!s.date_to.is_empty()).then(|| s.date_to.clone()),
                registrator_ref: (!s.registrator_ref.is_empty()).then(|| s.registrator_ref.clone()),
                registrator_type: (!s.registrator_type.is_empty())
                    .then(|| s.registrator_type.clone()),
                layer: (!s.layer.is_empty()).then(|| s.layer.clone()),
                turnover_code: (!s.turnover_code.is_empty()).then(|| s.turnover_code.clone()),
                connection_mp_ref: (!s.connection_mp_ref.is_empty())
                    .then(|| s.connection_mp_ref.clone()),
                debit_account: (!s.debit_account.is_empty()).then(|| s.debit_account.clone()),
                credit_account: (!s.credit_account.is_empty()).then(|| s.credit_account.clone()),
                sort_by: Some(s.sort_field.clone()),
                sort_desc: !s.sort_ascending,
                limit: s.page_size,
                offset: s.page * s.page_size,
            });

            match fetch_general_ledger(&query).await {
                Ok(response) => {
                    state.update(|s| {
                        s.entries = response.entries;
                        s.total_count = response.total;
                        s.total_pages = response.total_pages;
                        s.page = response.page;
                        s.page_size = response.page_size;
                        s.is_loaded = true;
                    });
                }
                Err(err) => set_error.set(Some(err)),
            }

            set_loading.set(false);
        });
    };

    Effect::new(move |_| {
        if !state.with_untracked(|s| s.is_loaded) {
            load_entries();
        }
    });

    let resize_initialized = StoredValue::new(false);
    Effect::new(move |_| {
        if !resize_initialized.get_value() {
            resize_initialized.set_value(true);
            spawn_local(async move {
                gloo_timers::future::TimeoutFuture::new(100).await;
                init_column_resize(TABLE_ID, COLUMN_WIDTHS_KEY);
            });
        }
    });

    let active_filters_count = Signal::derive(move || {
        let s = state.get();
        let mut count = 0;
        if !s.date_from.is_empty() || !s.date_to.is_empty() {
            count += 1;
        }
        if !s.registrator_type.is_empty() {
            count += 1;
        }
        if !s.registrator_ref.is_empty() {
            count += 1;
        }
        if !s.turnover_code.is_empty() {
            count += 1;
        }
        if !s.connection_mp_ref.is_empty() {
            count += 1;
        }
        if !s.layer.is_empty() {
            count += 1;
        }
        if !s.debit_account.is_empty() {
            count += 1;
        }
        if !s.credit_account.is_empty() {
            count += 1;
        }
        count
    });

    let apply_filters = move || {
        state.update(|s| {
            s.registrator_type = registrator_type_input.get_untracked().trim().to_string();
            s.registrator_ref = registrator_ref_input.get_untracked().trim().to_string();
            s.layer = layer_input.get_untracked().trim().to_string();
            s.turnover_code = turnover_code_input.get_untracked().trim().to_string();
            s.connection_mp_ref = connection_mp_ref_input.get_untracked().trim().to_string();
            s.debit_account = debit_account_input.get_untracked().trim().to_string();
            s.credit_account = credit_account_input.get_untracked().trim().to_string();
            s.page = 0;
        });
        load_entries();
    };

    let reset_filters = move || {
        let defaults = GeneralLedgerListState::default();
        registrator_type_input.set(String::new());
        registrator_ref_input.set(String::new());
        layer_input.set(String::new());
        turnover_code_input.set(String::new());
        connection_mp_ref_input.set(String::new());
        debit_account_input.set(String::new());
        credit_account_input.set(String::new());
        state.update(|s| {
            s.date_from = defaults.date_from;
            s.date_to = defaults.date_to;
            s.registrator_ref.clear();
            s.registrator_type.clear();
            s.layer.clear();
            s.turnover_code.clear();
            s.connection_mp_ref.clear();
            s.debit_account.clear();
            s.credit_account.clear();
            s.sort_field = defaults.sort_field;
            s.sort_ascending = defaults.sort_ascending;
            s.page = 0;
        });
        load_entries();
    };

    let toggle_sort = move |field: &'static str| {
        if was_just_resizing() {
            return;
        }

        state.update(|s| {
            if s.sort_field == field {
                s.sort_ascending = !s.sort_ascending;
            } else {
                s.sort_field = field.to_string();
                s.sort_ascending = true;
            }
            s.page = 0;
        });
        load_entries();
    };

    let go_to_page = move |page: usize| {
        state.update(|s| s.page = page);
        load_entries();
    };

    let change_page_size = move |page_size: usize| {
        state.update(|s| {
            s.page_size = page_size;
            s.page = 0;
        });
        load_entries();
    };

    view! {
        <PageFrame page_id="general_ledger--list" category=PAGE_CAT_LIST>
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Главная книга"</h1>
                    <Badge>{move || state.get().total_count.to_string()}</Badge>
                </div>
                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| {
                            tabs_store.open_tab("general_ledger_turnovers", "Обороты GL");
                        }
                    >
                        "Обороты GL"
                    </Button>

                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| {
                            tabs_store.open_tab("general_ledger_report", "Отчёт GL");
                        }
                    >
                        "Отчёт GL"
                    </Button>

                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| load_entries()
                        disabled=Signal::derive(move || loading.get())
                    >
                        {icon("refresh")}
                        {move || if loading.get() { " Загрузка..." } else { " Обновить" }}
                    </Button>
                </div>
            </div>

            <div class="page__content">
                {move || error.get().map(|err| view! {
                    <div class="alert alert--error">{err}</div>
                })}

                <div class="filter-panel">
                    <div class="filter-panel-header">
                        <div
                            class="filter-panel-header__left"
                            on:click=move |_| set_is_filter_expanded.update(|value| *value = !*value)
                        >
                            <svg
                                width="16"
                                height="16"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                class=move || {
                                    if is_filter_expanded.get() {
                                        "filter-panel__chevron filter-panel__chevron--expanded"
                                    } else {
                                        "filter-panel__chevron"
                                    }
                                }
                            >
                                <polyline points="6 9 12 15 18 9"></polyline>
                            </svg>
                            {icon("filter")}
                            <span class="filter-panel__title">"Фильтры"</span>
                            {move || {
                                let count = active_filters_count.get();
                                if count > 0 {
                                    view! { <span class="filter-panel__badge">{count}</span> }.into_any()
                                } else {
                                    view! { <></> }.into_any()
                                }
                            }}
                        </div>

                        <div class="filter-panel-header__center">
                            <PaginationControls
                                current_page=Signal::derive(move || state.get().page)
                                total_pages=Signal::derive(move || state.get().total_pages)
                                total_count=Signal::derive(move || state.get().total_count)
                                page_size=Signal::derive(move || state.get().page_size)
                                on_page_change=Callback::new(go_to_page)
                                on_page_size_change=Callback::new(change_page_size)
                                page_size_options=vec![50, 100, 200, 500]
                            />
                        </div>

                        <div class="filter-panel-header__right">
                            <Button
                                appearance=ButtonAppearance::Primary
                                on_click=move |_| apply_filters()
                                disabled=Signal::derive(move || loading.get())
                            >
                                "Найти"
                            </Button>
                        </div>
                    </div>

                    <Show when=move || is_filter_expanded.get()>
                        <div class="filter-panel-content">
                            <Flex vertical=true gap=FlexGap::Small>
                                <Flex gap=FlexGap::Small align=FlexAlign::End style="flex-wrap: wrap;">
                                    <DateRangePicker
                                        date_from=Signal::derive(move || state.with(|s| s.date_from.clone()))
                                        date_to=Signal::derive(move || state.with(|s| s.date_to.clone()))
                                        on_change=Callback::new(move |(from, to)| {
                                            state.update(|s| {
                                                s.date_from = from;
                                                s.date_to = to;
                                                s.page = 0;
                                            });
                                            load_entries();
                                        })
                                        label="Период".to_string()
                                    />

                                    <div style="width: 220px;">
                                        <Flex vertical=true gap=FlexGap::Small>
                                            <Label>"Тип регистратора"</Label>
                                            <Input value=registrator_type_input placeholder="a015_wb_orders" />
                                        </Flex>
                                    </div>

                                    <div style="width: 260px;">
                                        <Flex vertical=true gap=FlexGap::Small>
                                            <Label>"Регистратор"</Label>
                                            <Input value=registrator_ref_input placeholder="a015_wb_orders:uuid" />
                                        </Flex>
                                    </div>
                                </Flex>

                                <Flex gap=FlexGap::Small align=FlexAlign::End style="flex-wrap: wrap;">
                                    <div style="width: 160px;">
                                        <Flex vertical=true gap=FlexGap::Small>
                                            <Label>"Layer"</Label>
                                            <Select value=layer_input>
                                                <option value="">"Все"</option>
                                                <option value="oper">"oper"</option>
                                                <option value="fact">"fact"</option>
                                                <option value="plan">"plan"</option>
                                            </Select>
                                        </Flex>
                                    </div>

                                    <div style="width: 340px;">
                                        <Flex vertical=true gap=FlexGap::Small>
                                            <Label>"Вид оборота"</Label>
                                            <Select value=turnover_code_input>
                                                {TURNOVER_OPTIONS.iter().map(|(val, label)| {
                                                    let v = val.to_string();
                                                    let l = label.to_string();
                                                    view! { <option value=v>{l}</option> }
                                                }).collect::<Vec<_>>()}
                                            </Select>
                                        </Flex>
                                    </div>

                                    <div style="width: 220px;">
                                        <Flex vertical=true gap=FlexGap::Small>
                                            <Label>"Счет Дт"</Label>
                                            <Select value=debit_account_input>
                                                {ACCOUNT_OPTIONS.iter().map(|(val, label)| {
                                                    let v = val.to_string();
                                                    let l = label.to_string();
                                                    view! { <option value=v>{l}</option> }
                                                }).collect::<Vec<_>>()}
                                            </Select>
                                        </Flex>
                                    </div>

                                    <div style="width: 220px;">
                                        <Flex vertical=true gap=FlexGap::Small>
                                            <Label>"Счет Кт"</Label>
                                            <Select value=credit_account_input>
                                                {ACCOUNT_OPTIONS.iter().map(|(val, label)| {
                                                    let v = val.to_string();
                                                    let l = label.to_string();
                                                    view! { <option value=v>{l}</option> }
                                                }).collect::<Vec<_>>()}
                                            </Select>
                                        </Flex>
                                    </div>

                                    <div style="width: 240px;">
                                        <Flex vertical=true gap=FlexGap::Small>
                                            <Label>"Cabinet MP"</Label>
                                            <Select value=connection_mp_ref_input>
                                                <option value="">"Все"</option>
                                                <For
                                                    each=move || cabinet_options.get()
                                                    key=|opt| opt.id.clone()
                                                    children=move |opt: CabinetOption| {
                                                        view! {
                                                            <option value={opt.id.clone()}>{opt.label.clone()}</option>
                                                        }
                                                    }
                                                />
                                            </Select>
                                        </Flex>
                                    </div>

                                    <div style="display: flex; gap: var(--spacing-sm);">
                                        <Button
                                            appearance=ButtonAppearance::Primary
                                            on_click=move |_| apply_filters()
                                            disabled=Signal::derive(move || loading.get())
                                        >
                                            "Найти"
                                        </Button>
                                        <Button
                                            appearance=ButtonAppearance::Secondary
                                            on_click=move |_| reset_filters()
                                            disabled=Signal::derive(move || loading.get())
                                        >
                                            "Сбросить"
                                        </Button>
                                    </div>
                                </Flex>
                            </Flex>
                        </div>
                    </Show>
                </div>

                {move || {
                    if loading.get() && state.with(|s| s.entries.is_empty()) {
                        return view! {
                            <Flex gap=FlexGap::Small style="align-items: center; justify-content: center; padding: var(--spacing-4xl);">
                                <Spinner />
                                <span>"Загрузка журнала..."</span>
                            </Flex>
                        }.into_any();
                    }

                    if state.with(|s| s.entries.is_empty()) {
                        return view! {
                            <div class="alert">"Записи журнала не найдены."</div>
                        }.into_any();
                    }

                    view! {
                        <div class="table-wrapper">
                            <TableCrosshairHighlight table_id=TABLE_ID.to_string() />

                            <Table attr:id=TABLE_ID attr:style="width: 100%; min-width: 2000px;">
                                <TableHeader>
                                    <TableRow>
                                        <TableHeaderCell resizable=false class="resizable" min_width=110.0>
                                            "ID"
                                        </TableHeaderCell>

                                        <TableHeaderCell resizable=false class="resizable" min_width=170.0>
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("entry_date")>
                                                "Дата"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "entry_date"))>
                                                    {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "entry_date", state.with(|s| s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>

                                        <TableHeaderCell resizable=false class="resizable" min_width=90.0>
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("layer")>
                                                "Layer"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "layer"))>
                                                    {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "layer", state.with(|s| s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>

                                        <TableHeaderCell resizable=false class="resizable" min_width=140.0>
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("registrator_type")>
                                                "Тип регистратора"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "registrator_type"))>
                                                    {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "registrator_type", state.with(|s| s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>

                                        <TableHeaderCell resizable=false class="resizable" min_width=180.0>
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("registrator_ref")>
                                                "Регистратор"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "registrator_ref"))>
                                                    {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "registrator_ref", state.with(|s| s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>

                                        <TableHeaderCell resizable=false class="resizable" min_width=90.0>
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("debit_account")>
                                                "Дт"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "debit_account"))>
                                                    {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "debit_account", state.with(|s| s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>

                                        <TableHeaderCell resizable=false class="resizable" min_width=90.0>
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("credit_account")>
                                                "Кт"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "credit_account"))>
                                                    {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "credit_account", state.with(|s| s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>

                                        <TableHeaderCell resizable=false class="resizable" min_width=120.0>
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("amount")>
                                                "Сумма"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "amount"))>
                                                    {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "amount", state.with(|s| s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>

                                        <TableHeaderCell resizable=false class="resizable" min_width=90.0>
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("qty")>
                                                "Кол-во"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "qty"))>
                                                    {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "qty", state.with(|s| s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>

                                        <TableHeaderCell resizable=false class="resizable" min_width=150.0>
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("turnover_code")>
                                                "Вид оборота"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "turnover_code"))>
                                                    {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "turnover_code", state.with(|s| s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>

                                        <TableHeaderCell resizable=false class="resizable" min_width=220.0>
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("connection_mp_ref")>
                                                "Connection MP"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "connection_mp_ref"))>
                                                    {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "connection_mp_ref", state.with(|s| s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>

                                        <TableHeaderCell resizable=false class="resizable" min_width=180.0>
                                            "Order ID"
                                        </TableHeaderCell>

                                        <TableHeaderCell resizable=false class="resizable" min_width=150.0>
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("resource_table")>
                                                "Resource Table"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "resource_table"))>
                                                    {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "resource_table", state.with(|s| s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>

                                        <TableHeaderCell resizable=false class="resizable" min_width=90.0>
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("resource_field")>
                                                "Resource Field"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "resource_field"))>
                                                    {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "resource_field", state.with(|s| s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>

                                        <TableHeaderCell resizable=false class="resizable" min_width=140.0>
                                            "Sign"
                                        </TableHeaderCell>

                                        <TableHeaderCell resizable=false class="resizable" min_width=170.0>
                                            <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("created_at")>
                                                "Создано"
                                                <span class=move || state.with(|s| get_sort_class(&s.sort_field, "created_at"))>
                                                    {move || get_sort_indicator(&state.with(|s| s.sort_field.clone()), "created_at", state.with(|s| s.sort_ascending))}
                                                </span>
                                            </div>
                                        </TableHeaderCell>
                                    </TableRow>
                                </TableHeader>

                                <TableBody>
                                    <For
                                        each=move || state.get().entries
                                        key=|entry| entry.id.clone()
                                        children=move |entry| {
                                            let row_id = entry.id.clone();
                                            let reg_type = entry.registrator_type.clone();
                                            let reg_ref = entry.registrator_ref.clone();
                                            let reg_label = registrator_display(&reg_ref, &reg_type);
                                            let has_link = registrator_tab_key(&reg_type, &reg_ref).is_some();
                                            let reg_type_for_click = reg_type.clone();
                                            let reg_ref_for_click = reg_ref.clone();

                                            view! {
                                                <TableRow>
                                                    <TableCell>
                                                        <TableCellLayout truncate=true>
                                                            <span
                                                                class="table__link"
                                                                on:click=move |_| open_detail(row_id.clone())
                                                            >
                                                                {short_id(&entry.id).to_string()}
                                                            </span>
                                                        </TableCellLayout>
                                                    </TableCell>

                                                    <TableCell>
                                                        <TableCellLayout>
                                                            {format_journal_datetime(&entry.entry_date)}
                                                        </TableCellLayout>
                                                    </TableCell>

                                                    <TableCell>
                                                        <TableCellLayout>
                                                            {entry.layer.as_str().to_string()}
                                                        </TableCellLayout>
                                                    </TableCell>

                                                    <TableCell>
                                                        <TableCellLayout truncate=true>
                                                            {entry.registrator_type.clone()}
                                                        </TableCellLayout>
                                                    </TableCell>

                                                    <TableCell>
                                                        <TableCellLayout truncate=true>
                                                            {if has_link {
                                                                view! {
                                                                    <span
                                                                        class="table__link"
                                                                        on:click=move |_| open_registrator(reg_type_for_click.clone(), reg_ref_for_click.clone())
                                                                    >
                                                                        {reg_label.clone()}
                                                                    </span>
                                                                }.into_any()
                                                            } else {
                                                                view! { <span>{reg_label.clone()}</span> }.into_any()
                                                            }}
                                                        </TableCellLayout>
                                                    </TableCell>

                                                    <TableCell>
                                                        <TableCellLayout>
                                                            <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                                                                {entry.debit_account.clone()}
                                                            </Badge>
                                                        </TableCellLayout>
                                                    </TableCell>

                                                    <TableCell>
                                                        <TableCellLayout>
                                                            <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Informative>
                                                                {entry.credit_account.clone()}
                                                            </Badge>
                                                        </TableCellLayout>
                                                    </TableCell>

                                                    <TableCellMoney
                                                        value=Some(entry.amount)
                                                        show_currency=false
                                                        color_by_sign=false
                                                    />

                                                    <TableCell>
                                                        <TableCellLayout attr:style="text-align: right;">
                                                            {format_optional_number(entry.qty)}
                                                        </TableCellLayout>
                                                    </TableCell>

                                                    <TableCell>
                                                        <TableCellLayout truncate=true>
                                                            {entry.turnover_code.clone()}
                                                        </TableCellLayout>
                                                    </TableCell>

                                                    <TableCell>
                                                        <TableCellLayout truncate=true>
                                                            {entry.connection_mp_ref.clone().unwrap_or_else(|| "—".to_string())}
                                                        </TableCellLayout>
                                                    </TableCell>

                                                    <TableCell>
                                                        <TableCellLayout truncate=true>
                                                            {entry.order_id.clone().unwrap_or_else(|| "—".to_string())}
                                                        </TableCellLayout>
                                                    </TableCell>

                                                    <TableCell>
                                                        <TableCellLayout truncate=true>
                                                            {entry.resource_table.clone()}
                                                        </TableCellLayout>
                                                    </TableCell>

                                                    <TableCell>
                                                        <TableCellLayout truncate=true>
                                                            {entry.resource_field.clone()}
                                                        </TableCellLayout>
                                                    </TableCell>

                                                    <TableCell>
                                                        <TableCellLayout attr:style="text-align: right;">
                                                            {entry.resource_sign.to_string()}
                                                        </TableCellLayout>
                                                    </TableCell>

                                                    <TableCell>
                                                        <TableCellLayout>
                                                            {format_journal_datetime(&entry.created_at)}
                                                        </TableCellLayout>
                                                    </TableCell>
                                                </TableRow>
                                            }
                                        }
                                    />
                                </TableBody>
                            </Table>
                        </div>
                    }.into_any()
                }}
            </div>
        </PageFrame>
    }
}
