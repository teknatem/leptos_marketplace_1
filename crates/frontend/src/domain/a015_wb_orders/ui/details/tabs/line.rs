//! Line tab - line-level fields and finance summary

use super::super::view_model::WbOrdersDetailsVm;
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

fn has_marketplace_order_payload(value: &Value) -> bool {
    value.get("price").is_some()
        || value.get("convertedPrice").is_some()
        || (value.get("id").is_some() && value.get("createdAt").is_some())
}

fn json_kopecks_to_rubles(value: &Value, key: &str) -> Option<f64> {
    json_number(value, key).map(|v| v / 100.0)
}

fn format_money(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{:.2}", v),
        None => "-".to_string(),
    }
}

#[component]
fn PriceMetricLabel(
    label: &'static str,
    endpoint: &'static str,
    description: &'static str,
) -> impl IntoView {
    view! {
        <HelpPopoverLabel label=label endpoint=endpoint description=description />
    }
}

#[component]
pub fn LineTab(vm: WbOrdersDetailsVm) -> impl IntoView {
    view! {
        {move || {
            let Some(order_data) = vm.order.get() else {
                return view! { <div>"Нет данных"</div> }.into_any();
            };

            let line = order_data.line;
            let reports = vm.finance_reports.get();
            let total_ppvz_vw: f64 = reports.iter().filter_map(|r| r.ppvz_vw).sum();
            let total_ppvz_vw_nds: f64 = reports.iter().filter_map(|r| r.ppvz_vw_nds).sum();
            let total_retail: f64 = reports.iter().filter_map(|r| r.retail_amount).sum();
            let total_ppvz_for_pay: f64 = reports.iter().filter_map(|r| r.ppvz_for_pay).sum();

            view! {
                <div class="detail-grid">
                    // Left column
                    <div class="detail-grid__col">
                        <MarketplaceApiAmountsCard vm=vm.clone() />

                        <CardAnimated delay_ms=40 nav_id="a015_wb_orders_details_line_amounts">
                            <h4 class="details-section__title">"Statistics API: /api/v1/supplier/orders"</h4>
                            <div class="form__hint" style="margin-bottom: var(--spacing-sm);">
                                "Поля и значения соответствуют ответу Statistics API. Для записей, созданных только из /api/v3/orders, эти суммы не дозаполняются."
                            </div>
                            <Table>
                                <TableHeader>
                                    <TableRow>
                                        <TableHeaderCell attr:style="width: auto;">"Показатель"</TableHeaderCell>
                                        <TableHeaderCell attr:style="width: auto;">"Поле"</TableHeaderCell>
                                        <TableHeaderCell attr:style="width: 100px; text-align: right;">"Значение"</TableHeaderCell>
                                        <TableHeaderCell attr:style="width: 50px;">"Ед."</TableHeaderCell>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    <TableRow>
                                        <TableCell><TableCellLayout>"Количество"</TableCellLayout></TableCell>
                                        <TableCell><TableCellLayout><code>"qty"</code></TableCellLayout></TableCell>
                                        <TableCellMoney value=Signal::derive(move || Some(line.qty)) show_currency=false color_by_sign=false />
                                        <TableCell><TableCellLayout>"шт."</TableCellLayout></TableCell>
                                    </TableRow>
                                    <TableRow>
                                        <TableCell><TableCellLayout>
                                            <PriceMetricLabel
                                                label="Полная цена"
                                                endpoint="/api/v1/supplier/orders"
                                                description="totalPrice из Statistics API: полная цена заказа до скидок WB в валюте продавца."
                                            />
                                        </TableCellLayout></TableCell>
                                        <TableCell><TableCellLayout><code>"total_price"</code></TableCellLayout></TableCell>
                                        <TableCellMoney value=Signal::derive(move || line.total_price) show_currency=false color_by_sign=false />
                                        <TableCell><TableCellLayout>"rub"</TableCellLayout></TableCell>
                                    </TableRow>
                                    <TableRow>
                                        <TableCell><TableCellLayout>"Скидка"</TableCellLayout></TableCell>
                                        <TableCell><TableCellLayout><code>"discount_percent"</code></TableCellLayout></TableCell>
                                        <TableCellMoney value=Signal::derive(move || line.discount_percent) show_currency=false color_by_sign=false />
                                        <TableCell><TableCellLayout>"%"</TableCellLayout></TableCell>
                                    </TableRow>
                                    <TableRow>
                                        <TableCell><TableCellLayout>
                                            <PriceMetricLabel
                                                label="Цена c учетом скидки"
                                                endpoint="/api/v1/supplier/orders"
                                                description="priceWithDisc из Statistics API: Фактически цена по прайс листу. Используется отдельно от Marketplace API price."
                                            />
                                        </TableCellLayout></TableCell>
                                        <TableCell><TableCellLayout><code>"price_with_disc"</code></TableCellLayout></TableCell>
                                        <TableCellMoney value=Signal::derive(move || line.price_with_disc) show_currency=false color_by_sign=false />
                                        <TableCell><TableCellLayout>"rub"</TableCellLayout></TableCell>
                                    </TableRow>
                                    <TableRow>
                                        <TableCell><TableCellLayout>"СПП"</TableCellLayout></TableCell>
                                        <TableCell><TableCellLayout><code>"spp"</code></TableCellLayout></TableCell>
                                        <TableCellMoney value=Signal::derive(move || line.spp) show_currency=false color_by_sign=false />
                                        <TableCell><TableCellLayout>"%"</TableCellLayout></TableCell>
                                    </TableRow>
                                    <TableRow>
                                        <TableCell><TableCellLayout>
                                            <PriceMetricLabel
                                                label="Итоговая цена"
                                                endpoint="/api/v1/supplier/orders"
                                                description="finishedPrice из Statistics API: цена с учетом всех скидок, кроме оплаты WB Кошельком. Предварительный операционный показатель."
                                            />
                                        </TableCellLayout></TableCell>
                                        <TableCell><TableCellLayout><code>"finished_price"</code></TableCellLayout></TableCell>
                                        <TableCellMoney value=Signal::derive(move || line.finished_price) show_currency=false color_by_sign=false />
                                        <TableCell><TableCellLayout>"rub"</TableCellLayout></TableCell>
                                    </TableRow>
                                </TableBody>
                            </Table>
                        </CardAnimated>
                    </div>

                    // Right column
                    <div class="detail-grid__col">
                        <CardAnimated delay_ms=0 nav_id="a015_wb_orders_details_line_margin">
                            <h4 class="details-section__title">"Расчет маржи"</h4>
                            <Table>
                                <TableHeader>
                                    <TableRow>
                                        <TableHeaderCell attr:style="width: auto;">"Показатель"</TableHeaderCell>
                                        <TableHeaderCell attr:style="width: auto;">"Поле"</TableHeaderCell>
                                        <TableHeaderCell attr:style="width: 100px; text-align: right;">"Значение"</TableHeaderCell>
                                        <TableHeaderCell attr:style="width: 50px;">"Ед."</TableHeaderCell>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    <TableRow>
                                        <TableCell><TableCellLayout>"Дилерская цена УТ"</TableCellLayout></TableCell>
                                        <TableCell><TableCellLayout><code>"dealer_price_ut"</code></TableCellLayout></TableCell>
                                        <TableCellMoney value=Signal::derive(move || line.dealer_price_ut) show_currency=false color_by_sign=false />
                                        <TableCell><TableCellLayout>"rub"</TableCellLayout></TableCell>
                                    </TableRow>
                                    <TableRow>
                                        <TableCell><TableCellLayout>"Маржа"</TableCellLayout></TableCell>
                                        <TableCell><TableCellLayout><code>"margin_pro"</code></TableCellLayout></TableCell>
                                        <TableCellMoney value=Signal::derive(move || line.margin_pro) show_currency=false color_by_sign=false />
                                        <TableCell><TableCellLayout>"%"</TableCellLayout></TableCell>
                                    </TableRow>
                                </TableBody>
                            </Table>
                        </CardAnimated>

                        <CardAnimated delay_ms=40 nav_id="a015_wb_orders_details_line_finance_summary">
                            <h4 class="details-section__title">"Сводка по финансовым отчетам (p903)"</h4>
                            <Flex gap=FlexGap::Medium style="flex-wrap: wrap;">
                                <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                                    {format!("Записей: {}", reports.len())}
                                </Badge>
                                <span>"PPVZ VW: " <strong>{format!("{:.2}", total_ppvz_vw)}</strong></span>
                                <span>"PPVZ VW NDS: " <strong>{format!("{:.2}", total_ppvz_vw_nds)}</strong></span>
                                <span>"Retail: " <strong>{format!("{:.2}", total_retail)}</strong></span>
                                <span>"For Pay: " <strong>{format!("{:.2}", total_ppvz_for_pay)}</strong></span>
                            </Flex>
                        </CardAnimated>
                        <SalesDetailsCard vm=vm.clone() />
                    </div>
                </div>
            }
            .into_any()
        }}
    }
}

#[component]
fn MarketplaceApiAmountsCard(vm: WbOrdersDetailsVm) -> impl IntoView {
    view! {
        {move || {
            if vm.marketplace_raw_json_loading.get() {
                return view! {
                    <CardAnimated delay_ms=40 nav_id="a015_wb_orders_details_line_marketplace_loading">
                        <h4 class="details-section__title">"Marketplace API: /api/v3/orders"</h4>
                        <Flex gap=FlexGap::Small style="align-items: center;">
                            <Spinner />
                            <span>"Загрузка исходных данных..."</span>
                        </Flex>
                    </CardAnimated>
                }
                .into_any();
            }

            // FBW (продажа со склада WB) не обслуживается Marketplace API /api/v3/orders,
            // поэтому marketplace_raw_payload_ref у таких заказов отсутствует по дизайну.
            let is_fbw = vm
                .order
                .get()
                .and_then(|o| o.warehouse.warehouse_type)
                .as_deref()
                == Some("Склад WB");
            let fbw_notice = || {
                view! {
                    <CardAnimated delay_ms=40 nav_id="a015_wb_orders_details_line_marketplace_fbw">
                        <h4 class="details-section__title">"Marketplace API: /api/v3/orders"</h4>
                        <div class="form__hint">
                            "FBW — данные Marketplace API не предусмотрены"
                        </div>
                    </CardAnimated>
                }
                .into_any()
            };

            let raw_value = vm
                .marketplace_raw_json
                .get()
                .and_then(|json| serde_json::from_str::<Value>(&json).ok());

            let Some(raw_value) = raw_value else {
                return if is_fbw {
                    fbw_notice()
                } else {
                    view! { <></> }.into_any()
                };
            };

            if !has_marketplace_order_payload(&raw_value) {
                return if is_fbw {
                    fbw_notice()
                } else {
                    view! { <></> }.into_any()
                };
            }

            let price = json_kopecks_to_rubles(&raw_value, "price");
            let final_price = json_kopecks_to_rubles(&raw_value, "finalPrice");
            let sale_price = json_kopecks_to_rubles(&raw_value, "salePrice");
            let scan_price = json_kopecks_to_rubles(&raw_value, "scanPrice");

            view! {
                <CardAnimated delay_ms=40 nav_id="a015_wb_orders_details_line_marketplace_amounts">
                    <h4 class="details-section__title">"Marketplace API: /api/v3/orders"</h4>
                    <div class="form__hint" style="margin-bottom: var(--spacing-sm);">
                        "Это исходные поля Marketplace API. Цены WB приходят в копейках, здесь показаны в рублях без переноса в поля Statistics API."
                    </div>
                    <Table>
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCell attr:style="width: auto;">"Показатель"</TableHeaderCell>
                                <TableHeaderCell attr:style="width: auto;">"Поле API"</TableHeaderCell>
                                <TableHeaderCell attr:style="width: 120px; text-align: right;">"Значение"</TableHeaderCell>
                                <TableHeaderCell attr:style="width: 70px;">"Ед."</TableHeaderCell>
                            </TableRow>
                        </TableHeader>

                        <TableBody>
                        <TableRow>
                        <TableCell><TableCellLayout>
                            <PriceMetricLabel
                                label="Цена прайс-листа"
                                endpoint="/api/v3/orders/new"
                                description="salePrice из Marketplace API: цена прайс-листа, без учета скидки WB. WB передает значение в копейках; поле может отсутствовать."
                            />
                        </TableCellLayout></TableCell>
                        <TableCell><TableCellLayout><code>"salePrice"</code></TableCellLayout></TableCell>
                        <TableCell attr:style="text-align: right;"><TableCellLayout>{format_money(sale_price)}</TableCellLayout></TableCell>
                        <TableCell><TableCellLayout>"rub"</TableCellLayout></TableCell>
                    </TableRow>
                    <TableRow>
                                <TableCell><TableCellLayout>
                                    <PriceMetricLabel
                                        label="Цена заказа"
                                        endpoint="/api/v3/orders, /api/v3/orders/new"
                                        description="price из Marketplace API: цена сборочного задания. WB передает значение в копейках, в карточке оно показано в рублях."
                                    />
                                </TableCellLayout></TableCell>
                                <TableCell><TableCellLayout><code>"price"</code></TableCellLayout></TableCell>
                                <TableCell attr:style="text-align: right;"><TableCellLayout>{format_money(price)}</TableCellLayout></TableCell>
                                <TableCell><TableCellLayout>"rub"</TableCellLayout></TableCell>
                            </TableRow>
                            <TableRow>
                                <TableCell><TableCellLayout>
                                    <PriceMetricLabel
                                        label="Финальная цена"
                                        endpoint="/api/v3/orders/new"
                                        description="finalPrice из Marketplace API: сумма, списанная с покупателя в валюте продажи с учетом всех скидок. WB передает значение в копейках; поле информационное и может отсутствовать."
                                    />
                                </TableCellLayout></TableCell>
                                <TableCell><TableCellLayout><code>"finalPrice"</code></TableCellLayout></TableCell>
                                <TableCell attr:style="text-align: right;"><TableCellLayout>{format_money(final_price)}</TableCellLayout></TableCell>
                                <TableCell><TableCellLayout>"rub"</TableCellLayout></TableCell>
                            </TableRow>
                            <TableRow>
                                <TableCell><TableCellLayout>
                                    <PriceMetricLabel
                                        label="Цена сканирования"
                                        endpoint="/api/v3/orders"
                                        description="scanPrice из Marketplace API: цена приемки заказа в пункт выдачи. WB передает значение в копейках; для части заказов поле null или отсутствует."
                                    />
                                </TableCellLayout></TableCell>
                                <TableCell><TableCellLayout><code>"scanPrice"</code></TableCellLayout></TableCell>
                                <TableCell attr:style="text-align: right;"><TableCellLayout>{format_money(scan_price)}</TableCellLayout></TableCell>
                                <TableCell><TableCellLayout>"rub"</TableCellLayout></TableCell>
                            </TableRow>
                        </TableBody>
                    </Table>
                </CardAnimated>
            }
            .into_any()
        }}
    }
}

#[component]
fn SalesDetailsCard(vm: WbOrdersDetailsVm) -> impl IntoView {
    view! {
        {move || {
            let sales = vm.wb_sales.get();

            if sales.is_empty() {
                return view! { <></> }.into_any();
            }

            let first_sale = sales[0].clone();
            let line = first_sale.line.clone();

            view! {
                <CardAnimated delay_ms=120 nav_id="a015_wb_orders_details_line_sales_details">
                    <h4 class="details-section__title">"Детализация Sales WB"</h4>
                    <Table>
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCell attr:style="width: auto;">"Наименование"</TableHeaderCell>
                                <TableHeaderCell attr:style="width: auto;">"Поле"</TableHeaderCell>
                                <TableHeaderCell attr:style="width: 100px; text-align: right;">"Значение"</TableHeaderCell>
                                <TableHeaderCell attr:style="width: 50px;">"Ед."</TableHeaderCell>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            <TableRow>
                                <TableCell><TableCellLayout>"Полная цена"</TableCellLayout></TableCell>
                                <TableCell><TableCellLayout><code>"total_price"</code></TableCellLayout></TableCell>
                                <TableCellMoney value=Signal::derive(move || line.total_price) show_currency=false color_by_sign=false />
                                <TableCell><TableCellLayout>"rub"</TableCellLayout></TableCell>
                            </TableRow>
                            <TableRow>
                                <TableCell><TableCellLayout>"Процент скидки"</TableCellLayout></TableCell>
                                <TableCell><TableCellLayout><code>"discount_percent"</code></TableCellLayout></TableCell>
                                <TableCellMoney value=Signal::derive(move || line.discount_percent) show_currency=false color_by_sign=false />
                                <TableCell><TableCellLayout>"%"</TableCellLayout></TableCell>
                            </TableRow>
                            <TableRow>
                                <TableCell><TableCellLayout>"Цена без скидок"</TableCellLayout></TableCell>
                                <TableCell><TableCellLayout><code>"price_list"</code></TableCellLayout></TableCell>
                                <TableCellMoney value=Signal::derive(move || line.price_list) show_currency=false color_by_sign=false />
                                <TableCell><TableCellLayout>"rub"</TableCellLayout></TableCell>
                            </TableRow>
                            <TableRow>
                                <TableCell><TableCellLayout>"Сумма скидок"</TableCellLayout></TableCell>
                                <TableCell><TableCellLayout><code>"discount_total"</code></TableCellLayout></TableCell>
                                <TableCellMoney value=Signal::derive(move || line.discount_total) show_currency=false color_by_sign=false />
                                <TableCell><TableCellLayout>"rub"</TableCellLayout></TableCell>
                            </TableRow>
                            <TableRow>
                                <TableCell><TableCellLayout>"Цена после скидок"</TableCellLayout></TableCell>
                                <TableCell><TableCellLayout><code>"price_effective"</code></TableCellLayout></TableCell>
                                <TableCellMoney value=Signal::derive(move || line.price_effective) show_currency=false color_by_sign=false />
                                <TableCell><TableCellLayout>"rub"</TableCellLayout></TableCell>
                            </TableRow>
                            <TableRow>
                                <TableCell><TableCellLayout>"СПП"</TableCellLayout></TableCell>
                                <TableCell><TableCellLayout><code>"spp"</code></TableCellLayout></TableCell>
                                <TableCellMoney value=Signal::derive(move || line.spp) show_currency=false color_by_sign=false />
                                <TableCell><TableCellLayout>"%"</TableCellLayout></TableCell>
                            </TableRow>
                            <TableRow>
                                <TableCell><TableCellLayout>"Итоговая цена"</TableCellLayout></TableCell>
                                <TableCell><TableCellLayout><code>"finished_price"</code></TableCellLayout></TableCell>
                                <TableCellMoney value=Signal::derive(move || line.finished_price) show_currency=false color_by_sign=false />
                                <TableCell><TableCellLayout>"rub"</TableCellLayout></TableCell>
                            </TableRow>
                            <TableRow>
                                <TableCell><TableCellLayout>"Сумма платежа"</TableCellLayout></TableCell>
                                <TableCell><TableCellLayout><code>"payment_sale_amount"</code></TableCellLayout></TableCell>
                                <TableCellMoney value=Signal::derive(move || line.payment_sale_amount) show_currency=false color_by_sign=false />
                                <TableCell><TableCellLayout>"rub"</TableCellLayout></TableCell>
                            </TableRow>
                            <TableRow>
                                <TableCell><TableCellLayout>"К выплате"</TableCellLayout></TableCell>
                                <TableCell><TableCellLayout><code>"amount_line"</code></TableCellLayout></TableCell>
                                <TableCellMoney value=Signal::derive(move || line.amount_line) show_currency=false color_by_sign=false />
                                <TableCell><TableCellLayout>"rub"</TableCellLayout></TableCell>
                            </TableRow>
                        </TableBody>
                    </Table>
                </CardAnimated>
            }.into_any()
        }}
    }
}
