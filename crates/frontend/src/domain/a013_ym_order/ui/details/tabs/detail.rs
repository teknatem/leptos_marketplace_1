//! Detail tab - YM order totals and operational metrics

use super::super::view_model::YmOrderDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::components::popover::HelpPopoverLabel;
use crate::shared::components::table::TableCellMoney;
use leptos::prelude::*;
use serde_json::Value;
use thaw::*;

fn json_number(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(|v| {
        v.as_f64()
            .or_else(|| v.as_i64().map(|n| n as f64))
            .or_else(|| v.as_u64().map(|n| n as f64))
    })
}

fn order_payload(value: &Value) -> &Value {
    value
        .get("order")
        .or_else(|| value.get("result").and_then(|result| result.get("order")))
        .unwrap_or(value)
}

fn subsidies_total(subsidies_json: Option<&str>) -> f64 {
    subsidies_json
        .and_then(|json| serde_json::from_str::<Vec<Value>>(json).ok())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("amount").and_then(|amount| amount.as_f64()))
                .sum()
        })
        .unwrap_or(0.0)
}

#[component]
fn MetricLabel(
    label: &'static str,
    #[prop(optional, default = "")] endpoint: &'static str,
    #[prop(optional, default = "")] description: &'static str,
) -> impl IntoView {
    view! {
        {if description.is_empty() {
            view! { <span>{label}</span> }.into_any()
        } else {
            view! {
                <HelpPopoverLabel label=label endpoint=endpoint description=description />
            }
            .into_any()
        }}
    }
}

#[component]
fn MoneyMetricRow(
    label: &'static str,
    field: &'static str,
    #[prop(into)] value: Signal<Option<f64>>,
    #[prop(into)] unit: String,
    #[prop(optional, default = false)] bold: bool,
    #[prop(optional, default = false)] color_by_sign: bool,
    #[prop(optional, default = "")] endpoint: &'static str,
    #[prop(optional, default = "")] description: &'static str,
) -> impl IntoView {
    view! {
        <TableRow>
            <TableCell>
                <TableCellLayout>
                    <MetricLabel label=label endpoint=endpoint description=description />
                </TableCellLayout>
            </TableCell>
            <TableCell>
                <TableCellLayout>
                    <code>{field}</code>
                </TableCellLayout>
            </TableCell>
            <TableCellMoney value=value show_currency=false color_by_sign=color_by_sign bold=bold />
            <TableCell>
                <TableCellLayout>{unit}</TableCellLayout>
            </TableCell>
        </TableRow>
    }
}

#[component]
fn CountMetricRow(
    label: &'static str,
    field: &'static str,
    value: String,
    #[prop(into)] unit: String,
) -> impl IntoView {
    view! {
        <TableRow>
            <TableCell>
                <TableCellLayout>{label}</TableCellLayout>
            </TableCell>
            <TableCell>
                <TableCellLayout>
                    <code>{field}</code>
                </TableCellLayout>
            </TableCell>
            <TableCell class="text-right">
                <TableCellLayout>{value}</TableCellLayout>
            </TableCell>
            <TableCell>
                <TableCellLayout>{unit}</TableCellLayout>
            </TableCell>
        </TableRow>
    }
}

#[component]
fn MetricTable(children: Children) -> impl IntoView {
    view! {
        <Table>
            <TableHeader>
                <TableRow>
                    <TableHeaderCell attr:style="width: auto;">"Показатель"</TableHeaderCell>
                    <TableHeaderCell attr:style="width: auto;">"Поле"</TableHeaderCell>
                    <TableHeaderCell attr:style="width: 120px; text-align: right;">"Значение"</TableHeaderCell>
                    <TableHeaderCell attr:style="width: 60px;">"Ед."</TableHeaderCell>
                </TableRow>
            </TableHeader>
            <TableBody>{children()}</TableBody>
        </Table>
    }
}

#[component]
pub fn DetailTab(vm: YmOrderDetailsVm) -> impl IntoView {
    view! {
        {move || {
            let Some(order_data) = vm.order.get() else {
                return view! { <div>"Нет данных"</div> }.into_any();
            };

            let lines = order_data.lines.clone();
            let lines_count = lines.len();
            let total_qty: f64 = lines.iter().map(|line| line.qty).sum();
            let price_list_amount: f64 = lines
                .iter()
                .filter_map(|line| line.price_list.map(|price| price * line.qty))
                .sum();
            let discount_amount: f64 = lines
                .iter()
                .filter_map(|line| line.discount_total.map(|discount| discount * line.qty))
                .sum();
            let price_effective_amount: f64 = lines
                .iter()
                .filter_map(|line| line.price_effective.map(|price| price * line.qty))
                .sum();
            let amount_line_total: f64 = lines.iter().filter_map(|line| line.amount_line).sum();
            let buyer_amount: f64 = lines
                .iter()
                .filter_map(|line| line.buyer_price.map(|price| price * line.qty))
                .sum();
            let line_subsidies_total: f64 = lines
                .iter()
                .map(|line| subsidies_total(line.subsidies_json.as_deref()))
                .sum();
            let header_subsidies_total = subsidies_total(order_data.header.subsidies_json.as_deref());
            let subsidies_total_value = if header_subsidies_total > 0.0 {
                header_subsidies_total
            } else {
                line_subsidies_total
            };
            let effective_amount = amount_line_total + subsidies_total_value;
            let total_dealer_amount_lines: f64 = lines
                .iter()
                .filter_map(|line| line.dealer_price_ut.map(|price| price * line.qty))
                .sum();
            let total_dealer_amount = order_data
                .header
                .total_dealer_amount
                .or_else(|| (total_dealer_amount_lines > 0.0).then_some(total_dealer_amount_lines));
            let lines_without_nomenclature = lines
                .iter()
                .filter(|line| line.nomenclature_ref.is_none())
                .count();
            let currency = StoredValue::new(
                order_data
                    .header
                    .currency
                    .clone()
                    .unwrap_or_else(|| "RUR".to_string()),
            );
            let header_total_amount = order_data.header.total_amount;
            let header_items_total = order_data.header.items_total;
            let header_delivery_total = order_data.header.delivery_total;
            let header_margin_pro = order_data.header.margin_pro;

            view! {
                <div class="detail-grid">
                    <div class="detail-grid__col">
                        <CardAnimated delay_ms=0 nav_id="a013_ym_order_details_detail_totals">
                            <h4 class="details-section__title">"Итоги заказа YM"</h4>
                            <div class="form__hint" style="margin-bottom: var(--spacing-sm);">
                                "Сводные суммы из заголовка заказа и рассчитанные итоги по строкам."
                            </div>
                            <MetricTable>
                                <MoneyMetricRow
                                    label="Итог заказа API"
                                    field="header.total_amount"
                                    value=Signal::derive(move || header_total_amount)
                                    unit=currency.get_value()
                                    endpoint="/campaigns/{campaignId}/orders/{orderId}"
                                    description="total из Partner API: общая сумма заказа, как её вернул Яндекс Маркет."
                                />
                                <MoneyMetricRow
                                    label="Сумма товаров"
                                    field="header.items_total"
                                    value=Signal::derive(move || header_items_total)
                                    unit=currency.get_value()
                                    endpoint="/campaigns/{campaignId}/orders/{orderId}"
                                    description="itemsTotal из Partner API: стоимость товарных позиций без доставки."
                                />
                                <MoneyMetricRow
                                    label="Доставка"
                                    field="header.delivery_total"
                                    value=Signal::derive(move || header_delivery_total)
                                    unit=currency.get_value()
                                />
                                <MoneyMetricRow
                                    label="Субсидии"
                                    field="header.subsidies_json / lines.subsidies_json"
                                    value=Signal::derive(move || Some(subsidies_total_value))
                                    unit=currency.get_value()
                                />
                                <MoneyMetricRow
                                    label="Эффективная сумма"
                                    field="sum(amount_line) + subsidies"
                                    value=Signal::derive(move || Some(effective_amount))
                                    unit=currency.get_value()
                                    bold=true
                                />
                            </MetricTable>
                        </CardAnimated>

                        <RawApiAmountsCard vm=vm.clone() currency=currency.get_value() />
                    </div>

                    <div class="detail-grid__col">
                        <CardAnimated delay_ms=40 nav_id="a013_ym_order_details_detail_lines">
                            <h4 class="details-section__title">"Строки и цены"</h4>
                            <MetricTable>
                                <CountMetricRow
                                    label="Строк заказа"
                                    field="lines.len()"
                                    value=lines_count.to_string()
                                    unit="шт."
                                />
                                <MoneyMetricRow
                                    label="Количество"
                                    field="sum(qty)"
                                    value=Signal::derive(move || Some(total_qty))
                                    unit="шт."
                                    color_by_sign=false
                                />
                                <MoneyMetricRow
                                    label="Сумма по прайсу"
                                    field="sum(price_list * qty)"
                                    value=Signal::derive(move || Some(price_list_amount))
                                    unit=currency.get_value()
                                    color_by_sign=false
                                />
                                <MoneyMetricRow
                                    label="Скидки"
                                    field="sum(discount_total * qty)"
                                    value=Signal::derive(move || Some(discount_amount))
                                    unit=currency.get_value()
                                    color_by_sign=false
                                />
                                <MoneyMetricRow
                                    label="Цена после скидок"
                                    field="sum(price_effective * qty)"
                                    value=Signal::derive(move || Some(price_effective_amount))
                                    unit=currency.get_value()
                                    color_by_sign=false
                                />
                                <MoneyMetricRow
                                    label="Оплачивает покупатель"
                                    field="sum(buyer_price * qty)"
                                    value=Signal::derive(move || Some(buyer_amount))
                                    unit=currency.get_value()
                                    color_by_sign=false
                                />
                                <MoneyMetricRow
                                    label="Сумма строк"
                                    field="sum(amount_line)"
                                    value=Signal::derive(move || Some(amount_line_total))
                                    unit=currency.get_value()
                                    bold=true
                                    color_by_sign=false
                                />
                            </MetricTable>
                        </CardAnimated>

                        <CardAnimated delay_ms=80 nav_id="a013_ym_order_details_detail_margin">
                            <h4 class="details-section__title">"Маржа и сопоставление"</h4>
                            <MetricTable>
                                <MoneyMetricRow
                                    label="Дилерская сумма"
                                    field="header.total_dealer_amount"
                                    value=Signal::derive(move || total_dealer_amount)
                                    unit=currency.get_value()
                                    color_by_sign=false
                                />
                                <MoneyMetricRow
                                    label="Маржа"
                                    field="header.margin_pro"
                                    value=Signal::derive(move || header_margin_pro)
                                    unit="%"
                                    bold=true
                                    color_by_sign=true
                                />
                                <CountMetricRow
                                    label="Без номенклатуры"
                                    field="lines.nomenclature_ref"
                                    value=lines_without_nomenclature.to_string()
                                    unit="стр."
                                />
                            </MetricTable>
                        </CardAnimated>

                        <PaymentReportsSummaryCard vm=vm.clone() currency=currency.get_value() />
                    </div>
                </div>
            }
            .into_any()
        }}
    }
}

#[component]
fn RawApiAmountsCard(vm: YmOrderDetailsVm, currency: String) -> impl IntoView {
    let currency = StoredValue::new(currency);

    view! {
        {move || {
            if vm.raw_json_loading.get() {
                return view! {
                    <CardAnimated delay_ms=80 nav_id="a013_ym_order_details_detail_raw_loading">
                        <h4 class="details-section__title">"Raw API сверка"</h4>
                        <Flex gap=FlexGap::Small style="align-items: center;">
                            <Spinner />
                            <span>"Загрузка исходных данных..."</span>
                        </Flex>
                    </CardAnimated>
                }
                .into_any();
            }

            let raw_value = vm
                .raw_json
                .get()
                .and_then(|json| serde_json::from_str::<Value>(&json).ok());
            let Some(raw_value) = raw_value else {
                // raw-payload может отсутствовать в БД — штатная ситуация, не спиннер.
                return view! {
                    <CardAnimated delay_ms=80 nav_id="a013_ym_order_details_detail_raw_empty">
                        <h4 class="details-section__title">"Raw API сверка"</h4>
                        <div style="color: var(--color-text-secondary);">
                            "Исходный JSON Partner API для этого заказа не сохранён в БД."
                        </div>
                    </CardAnimated>
                }
                .into_any();
            };

            let payload = order_payload(&raw_value);
            let items_count = payload
                .get("items")
                .and_then(|items| items.as_array())
                .map(|items| items.len())
                .unwrap_or(0);
            let total = json_number(payload, "total");
            let items_total = json_number(payload, "itemsTotal");
            let delivery_total = json_number(payload, "deliveryTotal");

            view! {
                <CardAnimated delay_ms=80 nav_id="a013_ym_order_details_detail_raw_api">
                    <h4 class="details-section__title">"Raw API сверка"</h4>
                    <div class="form__hint" style="margin-bottom: var(--spacing-sm);">
                        "Контрольные значения напрямую из сохранённого JSON Partner API."
                    </div>
                    <MetricTable>
                        <CountMetricRow
                            label="Строк в API"
                            field="items.length"
                            value=items_count.to_string()
                            unit="шт."
                        />
                        <MoneyMetricRow
                            label="Итог API"
                            field="total"
                            value=Signal::derive(move || total)
                            unit=currency.get_value()
                            color_by_sign=false
                        />
                        <MoneyMetricRow
                            label="Товары API"
                            field="itemsTotal"
                            value=Signal::derive(move || items_total)
                            unit=currency.get_value()
                            color_by_sign=false
                        />
                        <MoneyMetricRow
                            label="Доставка API"
                            field="deliveryTotal"
                            value=Signal::derive(move || delivery_total)
                            unit=currency.get_value()
                            color_by_sign=false
                        />
                    </MetricTable>
                </CardAnimated>
            }
            .into_any()
        }}
    }
}

#[component]
fn PaymentReportsSummaryCard(vm: YmOrderDetailsVm, currency: String) -> impl IntoView {
    let currency = StoredValue::new(currency);

    view! {
        {move || {
            if vm.payment_reports_loading.get() {
                return view! {
                    <CardAnimated delay_ms=120 nav_id="a013_ym_order_details_detail_payment_loading">
                        <h4 class="details-section__title">"Платёжные отчёты (p907)"</h4>
                        <Flex gap=FlexGap::Small style="align-items: center;">
                            <Spinner />
                            <span>"Загрузка платёжных отчётов..."</span>
                        </Flex>
                    </CardAnimated>
                }
                .into_any();
            }

            if let Some(err) = vm.payment_reports_error.get() {
                return view! {
                    <CardAnimated delay_ms=120 nav_id="a013_ym_order_details_detail_payment_error">
                        <h4 class="details-section__title">"Платёжные отчёты (p907)"</h4>
                        <div style="color: var(--color-error);">
                            "Ошибка загрузки: " {err}
                        </div>
                    </CardAnimated>
                }
                .into_any();
            }

            let reports = vm.payment_reports.get();
            let reports_count = reports.len();
            // Справочные строки p907 («Справочно: …») дублируют суммы и не идут в итог.
            let total_transaction_sum: f64 = reports
                .iter()
                .filter(|report| {
                    !report
                        .payment_status
                        .as_deref()
                        .map(|s| s.trim_start().starts_with("Справочно"))
                        .unwrap_or(false)
                })
                .filter_map(|report| report.transaction_sum)
                .sum();

            view! {
                <CardAnimated delay_ms=120 nav_id="a013_ym_order_details_detail_payment_summary">
                    <h4 class="details-section__title">"Платёжные отчёты (p907)"</h4>
                    <MetricTable>
                        <CountMetricRow
                            label="Записей"
                            field="p907.rows"
                            value=reports_count.to_string()
                            unit="шт."
                        />
                        <MoneyMetricRow
                            label="Сумма (без справочных)"
                            field="sum(transaction_sum ∉ «Справочно»)"
                            value=Signal::derive(move || Some(total_transaction_sum))
                            unit=currency.get_value()
                            color_by_sign=false
                        />
                    </MetricTable>
                </CardAnimated>
            }
            .into_any()
        }}
    }
}
