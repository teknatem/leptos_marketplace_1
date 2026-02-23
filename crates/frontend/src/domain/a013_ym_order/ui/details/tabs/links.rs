//! Links tab - YM Payment Report records linked by order_id

use super::super::view_model::YmOrderDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::table::TableCellMoney;
use leptos::prelude::*;
use thaw::*;

fn fmt_date(s: &str) -> String {
    let bytes = s.as_bytes();
    if bytes.len() >= 10 && bytes[4] == b'-' && bytes[7] == b'-' {
        let year = &s[0..4];
        let month = &s[5..7];
        let day = &s[8..10];
        let rest = &s[10..];
        return format!("{}.{}.{}{}", day, month, year, rest);
    }
    s.to_string()
}

/// Links tab: displays p907_ym_payment_report rows for this order
#[component]
pub fn LinksTab(vm: YmOrderDetailsVm) -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    view! {
        {move || {
            if vm.payment_reports_loading.get() {
                return view! {
                    <Card>
                        <Flex gap=FlexGap::Small style="align-items: center; justify-content: center; padding: var(--spacing-xl);">
                            <Spinner />
                            <span>"Загрузка платёжных отчётов..."</span>
                        </Flex>
                    </Card>
                }.into_any();
            }

            if let Some(err) = vm.payment_reports_error.get() {
                return view! {
                    <Card>
                        <div style="color: var(--color-error);">
                            "Ошибка загрузки: " {err}
                        </div>
                    </Card>
                }.into_any();
            }

            let reports = vm.payment_reports.get();
            if reports.is_empty() {
                return view! {
                    <Card>
                        <h4 class="details-section__title">"Платёжные отчёты (p907)"</h4>
                        <div style="color: var(--color-text-secondary);">
                            "Связанные платёжные транзакции для данного заказа не найдены."
                        </div>
                    </Card>
                }.into_any();
            }

            let reports_count = reports.len();
            let total_transaction_sum: f64 = reports.iter().filter_map(|r| r.transaction_sum).sum();
            let total_bank_sum: f64 = reports.iter().filter_map(|r| r.bank_sum).sum();
            let reports_for_table = reports;

            view! {
                <Card>
                    <h4 class="details-section__title">"Платёжные отчёты (p907)"</h4>

                    <Flex gap=FlexGap::Medium style="flex-wrap: wrap; margin-bottom: var(--spacing-md);">
                        <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                            {format!("Найдено: {}", reports_count)}
                        </Badge>
                        <span>
                            "Сумма транзакций: "
                            <strong>{format!("{:.2}", total_transaction_sum)}</strong>
                        </span>
                        <span>
                            "Сумма ПП: "
                            <strong>{format!("{:.2}", total_bank_sum)}</strong>
                        </span>
                    </Flex>

                    <div style="max-height: calc(100vh - 400px); overflow: auto;">
                        <Table>
                            <TableHeader>
                                <TableRow>
                                    <TableHeaderCell resizable=true min_width=120.0>"Дата"</TableHeaderCell>
                                    <TableHeaderCell resizable=true min_width=140.0>"Тип транзакции"</TableHeaderCell>
                                    <TableHeaderCell resizable=true min_width=120.0>"ID транзакции"</TableHeaderCell>
                                    <TableHeaderCell resizable=true min_width=120.0>"SKU"</TableHeaderCell>
                                    <TableHeaderCell resizable=true min_width=200.0>"Товар / Услуга"</TableHeaderCell>
                                    <TableHeaderCell resizable=true min_width=60.0>"Кол-во"</TableHeaderCell>
                                    <TableHeaderCell resizable=true min_width=100.0>"Сумма"</TableHeaderCell>
                                    <TableHeaderCell resizable=true min_width=100.0>"Сумма ПП"</TableHeaderCell>
                                    <TableHeaderCell resizable=true min_width=100.0>"Статус"</TableHeaderCell>
                                    <TableHeaderCell resizable=true min_width=100.0>"Источник"</TableHeaderCell>
                                    <TableHeaderCell resizable=true min_width=160.0>"Комментарий"</TableHeaderCell>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                <For
                                    each=move || reports_for_table.clone()
                                    key=|r| r.record_key.clone()
                                    children={
                                        let tabs_store = tabs_store;
                                        move |report| {
                                            let record_key = report.record_key.clone();
                                            let date_str = report.transaction_date.as_deref().map(fmt_date).unwrap_or_default();
                                            let date_for_title = date_str.clone();

                                            view! {
                                                <TableRow
                                                    on:click={
                                                        let tabs_store = tabs_store;
                                                        move |_| {
                                                            let tab_key = format!(
                                                                "p907_ym_payment_report_detail_{}",
                                                                js_sys::encode_uri_component(&record_key)
                                                                    .as_string()
                                                                    .unwrap_or_else(|| record_key.clone())
                                                            );
                                                            let tab_title = format!("ЯМ Платёж {}", date_for_title);
                                                            tabs_store.open_tab(&tab_key, &tab_title);
                                                        }
                                                    }
                                                    attr:style="cursor: pointer;"
                                                >
                                                    <TableCell>
                                                        <TableCellLayout>{date_str}</TableCellLayout>
                                                    </TableCell>
                                                    <TableCell>
                                                        <TableCellLayout truncate=true>
                                                            {report.transaction_type.unwrap_or_else(|| "—".to_string())}
                                                        </TableCellLayout>
                                                    </TableCell>
                                                    <TableCell>
                                                        <TableCellLayout truncate=true>
                                                            <span style="font-family: monospace; font-size: 0.85em;">
                                                                {report.transaction_id.unwrap_or_else(|| "—".to_string())}
                                                            </span>
                                                        </TableCellLayout>
                                                    </TableCell>
                                                    <TableCell>
                                                        <TableCellLayout truncate=true>
                                                            {report.shop_sku.unwrap_or_else(|| "—".to_string())}
                                                        </TableCellLayout>
                                                    </TableCell>
                                                    <TableCell>
                                                        <TableCellLayout truncate=true>
                                                            {report.offer_or_service_name.unwrap_or_else(|| "—".to_string())}
                                                        </TableCellLayout>
                                                    </TableCell>
                                                    <TableCell>
                                                        <TableCellLayout>
                                                            {report.count.map(|v| v.to_string()).unwrap_or_else(|| "—".to_string())}
                                                        </TableCellLayout>
                                                    </TableCell>
                                                    <TableCellMoney value=report.transaction_sum />
                                                    <TableCellMoney value=report.bank_sum />
                                                    <TableCell>
                                                        <TableCellLayout truncate=true>
                                                            {report.payment_status.unwrap_or_else(|| "—".to_string())}
                                                        </TableCellLayout>
                                                    </TableCell>
                                                    <TableCell>
                                                        <TableCellLayout truncate=true>
                                                            {report.transaction_source.unwrap_or_else(|| "—".to_string())}
                                                        </TableCellLayout>
                                                    </TableCell>
                                                    <TableCell>
                                                        <TableCellLayout truncate=true>
                                                            {report.comments.unwrap_or_else(|| "—".to_string())}
                                                        </TableCellLayout>
                                                    </TableCell>
                                                </TableRow>
                                            }
                                        }
                                    }
                                />
                            </TableBody>
                        </Table>
                    </div>
                </Card>
            }.into_any()
        }}
    }
}
