use crate::general_ledger::api::fetch_wb_weekly_reconciliation;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::api_utils::api_base;
use crate::shared::components::date_range_picker::DateRangePicker;
use crate::shared::components::table::{format_money, TableCrosshairHighlight};
use crate::shared::list_utils::{get_sort_class, get_sort_indicator, sort_list, Sortable};
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_LIST;
use chrono::{Datelike, Utc};
use contracts::domain::a006_connection_mp::aggregate::ConnectionMP;
use contracts::domain::common::AggregateId;
use contracts::general_ledger::{WbWeeklyReconciliationQuery, WbWeeklyReconciliationRow};
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cmp::Ordering;
use thaw::*;

const TABLE_ID: &str = "wb-weekly-reconciliation-table";

#[derive(Clone, Debug)]
struct CabinetOption {
    id: String,
    label: String,
}

fn default_month_range() -> (String, String) {
    let now = Utc::now().date_naive();
    let year = now.year();
    let month = now.month();
    let start = chrono::NaiveDate::from_ymd_opt(year, month, 1).expect("start");
    let end = if month == 12 {
        chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1).map(|d| d - chrono::Duration::days(1))
    } else {
        chrono::NaiveDate::from_ymd_opt(year, month + 1, 1).map(|d| d - chrono::Duration::days(1))
    }
    .expect("end");

    (
        start.format("%Y-%m-%d").to_string(),
        end.format("%Y-%m-%d").to_string(),
    )
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

    let mut options: Vec<CabinetOption> = data
        .into_iter()
        .map(|connection| {
            let label = if connection.base.description.trim().is_empty() {
                connection.base.code.clone()
            } else {
                connection.base.description.clone()
            };
            CabinetOption {
                id: connection.base.id.as_string(),
                label,
            }
        })
        .collect();
    options.sort_by(|left, right| left.label.cmp(&right.label));
    options
}

fn display_optional_money(value: Option<f64>) -> String {
    value.map(format_money).unwrap_or_default()
}

fn compare_optional_text(left: &Option<String>, right: &Option<String>) -> Ordering {
    match (left, right) {
        (Some(a), Some(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn compare_optional_number(left: Option<f64>, right: Option<f64>) -> Ordering {
    match (left, right) {
        (Some(a), Some(b)) => a.partial_cmp(&b).unwrap_or(Ordering::Equal),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

impl Sortable for WbWeeklyReconciliationRow {
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering {
        match field {
            "service_name" => self
                .service_name
                .to_lowercase()
                .cmp(&other.service_name.to_lowercase()),
            "connection_name" => {
                let left = self
                    .connection_name
                    .clone()
                    .or_else(|| Some(self.connection_id.clone()));
                let right = other
                    .connection_name
                    .clone()
                    .or_else(|| Some(other.connection_id.clone()));
                compare_optional_text(&left, &right)
            }
            "report_period_from" => {
                compare_optional_text(&self.report_period_from, &other.report_period_from)
            }
            "report_period_to" => {
                compare_optional_text(&self.report_period_to, &other.report_period_to)
            }
            "realized_goods_total" => {
                compare_optional_number(self.realized_goods_total, other.realized_goods_total)
            }
            "wb_reward_with_vat" => {
                compare_optional_number(self.wb_reward_with_vat, other.wb_reward_with_vat)
            }
            "seller_transfer_total" => {
                compare_optional_number(self.seller_transfer_total, other.seller_transfer_total)
            }
            "gl_total_balance" => {
                compare_optional_number(self.gl_total_balance, other.gl_total_balance)
            }
            "difference" => compare_optional_number(self.difference, other.difference),
            _ => Ordering::Equal,
        }
    }
}

#[component]
pub fn WbWeeklyReconciliationPage() -> impl IntoView {
    let tabs_store = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let (default_from, default_to) = default_month_range();

    let date_from = RwSignal::new(default_from);
    let date_to = RwSignal::new(default_to);
    let cabinet_sig = RwSignal::new(String::new());

    let rows = RwSignal::new(Vec::<WbWeeklyReconciliationRow>::new());
    let cabinets = RwSignal::new(Vec::<CabinetOption>::new());
    let loading = RwSignal::new(false);
    let error_msg = RwSignal::new(Option::<String>::None);
    let loaded = RwSignal::new(false);
    let sort_field = RwSignal::new("report_period_to".to_string());
    let sort_ascending = RwSignal::new(false);

    Effect::new(move |_| {
        spawn_local(async move {
            cabinets.set(load_cabinet_options().await);
        });
    });

    let load_report = move || {
        let date_from_value = date_from.get_untracked();
        let date_to_value = date_to.get_untracked();
        let cabinet_value = cabinet_sig.get_untracked();

        spawn_local(async move {
            loading.set(true);
            error_msg.set(None);

            let query = WbWeeklyReconciliationQuery {
                date_from: Some(date_from_value),
                date_to: Some(date_to_value),
                connection_id: if cabinet_value.trim().is_empty() {
                    None
                } else {
                    Some(cabinet_value)
                },
            };

            match fetch_wb_weekly_reconciliation(&query).await {
                Ok(response) => {
                    rows.set(response.items);
                    loaded.set(true);
                }
                Err(err) => error_msg.set(Some(err)),
            }

            loading.set(false);
        });
    };

    let open_document = move |row: &WbWeeklyReconciliationRow| {
        tabs_store.open_tab(
            &format!("a027_wb_documents_details_{}", row.document_id),
            &format!("WB Doc {}", row.service_name),
        );
    };

    let toggle_sort = move |field: &'static str| {
        if sort_field.get_untracked() == field {
            sort_ascending.update(|value| *value = !*value);
        } else {
            sort_field.set(field.to_string());
            sort_ascending.set(true);
        }
    };

    let sorted_rows = Signal::derive(move || {
        let mut items = rows.get();
        sort_list(&mut items, &sort_field.get(), sort_ascending.get());
        items
    });

    view! {
        <PageFrame page_id="wb_weekly_reconciliation--list" category=PAGE_CAT_LIST>
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Сверка weekly WB и GL 7609"</h1>
                    <Badge appearance=BadgeAppearance::Filled>
                        {move || rows.get().len().to_string()}
                    </Badge>
                </div>
            </div>

            <div class="page__content">
                <div class="filter-panel">
                    <div class="filter-panel-header">
                        <div class="filter-panel-header__left">
                            "Фильтры"
                        </div>
                        <div class="filter-panel-header__right">
                            <Button
                                appearance=ButtonAppearance::Primary
                                on_click=move |_| load_report()
                                disabled=Signal::derive(move || loading.get())
                            >
                                {move || if loading.get() { "Загрузка..." } else { "Применить" }}
                            </Button>
                        </div>
                    </div>

                    <div class="filter-panel-content">
                        <Flex gap=FlexGap::Small align=FlexAlign::End>
                            <div style="min-width: 420px;">
                                <DateRangePicker
                                    date_from=Signal::derive(move || date_from.get())
                                    date_to=Signal::derive(move || date_to.get())
                                    on_change=Callback::new(move |(from, to): (String, String)| {
                                        date_from.set(from);
                                        date_to.set(to);
                                    })
                                    label="Период".to_string()
                                />
                            </div>

                            <div style="width: 280px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Кабинет"</Label>
                                    <Select value=cabinet_sig>
                                        <option value="">"Все кабинеты"</option>
                                        {move || {
                                            cabinets.get().into_iter().map(|cabinet| {
                                                view! {
                                                    <option value=cabinet.id.clone()>{cabinet.label}</option>
                                                }
                                            }).collect_view()
                                        }}
                                    </Select>
                                </Flex>
                            </div>
                        </Flex>
                    </div>
                </div>

                {move || error_msg.get().map(|err| view! { <div class="alert alert--error">{err}</div> })}

                {move || {
                    if loading.get() {
                        view! { <div class="page__placeholder"><Spinner /> " Загрузка..."</div> }.into_any()
                    } else if !loaded.get() {
                        view! { <div class="page__placeholder">"Задайте фильтры и нажмите \"Применить\""</div> }.into_any()
                    } else if rows.get().is_empty() {
                        view! { <div class="page__placeholder">"Нет данных для выбранных фильтров"</div> }.into_any()
                    } else {
                        view! {
                            <div class="table-wrapper">
                                <TableCrosshairHighlight table_id=TABLE_ID.to_string() />
                                <Table attr:id=TABLE_ID attr:style="width: 100%; min-width: 1480px;">
                                    <TableHeader>
                                        <TableRow>
                                            <TableHeaderCell>
                                                <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("service_name")>
                                                    "Документ"
                                                    <span class=move || get_sort_class(&sort_field.get(), "service_name")>
                                                        {move || get_sort_indicator(&sort_field.get(), "service_name", sort_ascending.get())}
                                                    </span>
                                                </div>
                                            </TableHeaderCell>
                                            <TableHeaderCell>
                                                <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("connection_name")>
                                                    "Кабинет"
                                                    <span class=move || get_sort_class(&sort_field.get(), "connection_name")>
                                                        {move || get_sort_indicator(&sort_field.get(), "connection_name", sort_ascending.get())}
                                                    </span>
                                                </div>
                                            </TableHeaderCell>
                                            <TableHeaderCell>
                                                <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("report_period_from")>
                                                    "Период с"
                                                    <span class=move || get_sort_class(&sort_field.get(), "report_period_from")>
                                                        {move || get_sort_indicator(&sort_field.get(), "report_period_from", sort_ascending.get())}
                                                    </span>
                                                </div>
                                            </TableHeaderCell>
                                            <TableHeaderCell>
                                                <div class="table__sortable-header" style="cursor: pointer;" on:click=move |_| toggle_sort("report_period_to")>
                                                    "Период по"
                                                    <span class=move || get_sort_class(&sort_field.get(), "report_period_to")>
                                                        {move || get_sort_indicator(&sort_field.get(), "report_period_to", sort_ascending.get())}
                                                    </span>
                                                </div>
                                            </TableHeaderCell>
                                            <TableHeaderCell>
                                                <div class="table__sortable-header" style="cursor: pointer; text-align: right;" on:click=move |_| toggle_sort("realized_goods_total")>
                                                    "Итого стоимость реализованного товара"
                                                    <span class=move || get_sort_class(&sort_field.get(), "realized_goods_total")>
                                                        {move || get_sort_indicator(&sort_field.get(), "realized_goods_total", sort_ascending.get())}
                                                    </span>
                                                </div>
                                            </TableHeaderCell>
                                            <TableHeaderCell>
                                                <div class="table__sortable-header" style="cursor: pointer; text-align: right;" on:click=move |_| toggle_sort("wb_reward_with_vat")>
                                                    "Сумма вознаграждения Вайлдберриз"
                                                    <span class=move || get_sort_class(&sort_field.get(), "wb_reward_with_vat")>
                                                        {move || get_sort_indicator(&sort_field.get(), "wb_reward_with_vat", sort_ascending.get())}
                                                    </span>
                                                </div>
                                            </TableHeaderCell>
                                            <TableHeaderCell>
                                                <div class="table__sortable-header" style="cursor: pointer; text-align: right;" on:click=move |_| toggle_sort("seller_transfer_total")>
                                                    "Итого к перечислению Продавцу"
                                                    <span class=move || get_sort_class(&sort_field.get(), "seller_transfer_total")>
                                                        {move || get_sort_indicator(&sort_field.get(), "seller_transfer_total", sort_ascending.get())}
                                                    </span>
                                                </div>
                                            </TableHeaderCell>
                                            <TableHeaderCell>
                                                <div class="table__sortable-header" style="cursor: pointer; text-align: right;" on:click=move |_| toggle_sort("gl_total_balance")>
                                                    "Результат из gl_account_view__7609"
                                                    <span class=move || get_sort_class(&sort_field.get(), "gl_total_balance")>
                                                        {move || get_sort_indicator(&sort_field.get(), "gl_total_balance", sort_ascending.get())}
                                                    </span>
                                                </div>
                                            </TableHeaderCell>
                                            <TableHeaderCell>
                                                <div class="table__sortable-header" style="cursor: pointer; text-align: right;" on:click=move |_| toggle_sort("difference")>
                                                    "Разница"
                                                    <span class=move || get_sort_class(&sort_field.get(), "difference")>
                                                        {move || get_sort_indicator(&sort_field.get(), "difference", sort_ascending.get())}
                                                    </span>
                                                </div>
                                            </TableHeaderCell>
                                        </TableRow>
                                    </TableHeader>
                                    <TableBody>
                                        <For
                                            each=move || sorted_rows.get()
                                            key=|row| row.document_id.clone()
                                            children=move |row| {
                                                let open_row = row.clone();
                                                view! {
                                                    <TableRow>
                                                        <TableCell>
                                                            <TableCellLayout>
                                                                <a
                                                                    href="#"
                                                                    class="table__link"
                                                                    on:click=move |ev| {
                                                                        ev.prevent_default();
                                                                        open_document(&open_row);
                                                                    }
                                                                >
                                                                    {row.service_name.clone()}
                                                                </a>
                                                            </TableCellLayout>
                                                        </TableCell>
                                                        <TableCell>
                                                            <TableCellLayout truncate=true>
                                                                {row.connection_name.clone().unwrap_or(row.connection_id.clone())}
                                                            </TableCellLayout>
                                                        </TableCell>
                                                        <TableCell>
                                                            <TableCellLayout>{row.report_period_from.clone().unwrap_or_default()}</TableCellLayout>
                                                        </TableCell>
                                                        <TableCell>
                                                            <TableCellLayout>{row.report_period_to.clone().unwrap_or_default()}</TableCellLayout>
                                                        </TableCell>
                                                        <TableCell>
                                                            <TableCellLayout attr:style="display:block; width:100%; text-align:right;">
                                                                {display_optional_money(row.realized_goods_total)}
                                                            </TableCellLayout>
                                                        </TableCell>
                                                        <TableCell>
                                                            <TableCellLayout attr:style="display:block; width:100%; text-align:right;">
                                                                {display_optional_money(row.wb_reward_with_vat)}
                                                            </TableCellLayout>
                                                        </TableCell>
                                                        <TableCell>
                                                            <TableCellLayout attr:style="display:block; width:100%; text-align:right;">
                                                                {display_optional_money(row.seller_transfer_total)}
                                                            </TableCellLayout>
                                                        </TableCell>
                                                        <TableCell>
                                                            <TableCellLayout attr:style="display:block; width:100%; text-align:right;">
                                                                {display_optional_money(row.gl_total_balance)}
                                                            </TableCellLayout>
                                                        </TableCell>
                                                        <TableCell>
                                                            <TableCellLayout attr:style="display:block; width:100%; text-align:right;">
                                                                {display_optional_money(row.difference)}
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
                    }
                }}
            </div>
        </PageFrame>
    }
}
