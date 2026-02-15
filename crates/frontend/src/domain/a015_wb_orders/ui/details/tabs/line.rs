//! Line tab - line-level fields and finance summary

use super::super::view_model::WbOrdersDetailsVm;
use crate::shared::components::table::TableCellMoney;
use leptos::prelude::*;
use thaw::*;

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
                <div style="display: grid; grid-template-columns: 600px 600px; gap: var(--spacing-md); align-items: start;">
                    // Left column
                    <div style="display: flex; flex-direction: column; gap: var(--spacing-md);">
                        <Card>
                            <h4 class="details-section__title">"Суммы и проценты"</h4>
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
                                        <TableCell><TableCellLayout>"Полная цена"</TableCellLayout></TableCell>
                                        <TableCell><TableCellLayout><code>"total_price"</code></TableCellLayout></TableCell>
                                        <TableCellMoney value=Signal::derive(move || line.total_price) show_currency=false color_by_sign=false />
                                        <TableCell><TableCellLayout>"rub"</TableCellLayout></TableCell>
                                    </TableRow>
                                    <TableRow>
                                        <TableCell><TableCellLayout>"Цена c учетом скидки"</TableCellLayout></TableCell>
                                        <TableCell><TableCellLayout><code>"price_with_disc"</code></TableCellLayout></TableCell>
                                        <TableCellMoney value=Signal::derive(move || line.price_with_disc) show_currency=false color_by_sign=false />
                                        <TableCell><TableCellLayout>"rub"</TableCellLayout></TableCell>
                                    </TableRow>
                                    <TableRow>
                                        <TableCell><TableCellLayout>"Итоговая цена"</TableCellLayout></TableCell>
                                        <TableCell><TableCellLayout><code>"finished_price"</code></TableCellLayout></TableCell>
                                        <TableCellMoney value=Signal::derive(move || line.finished_price) show_currency=false color_by_sign=false />
                                        <TableCell><TableCellLayout>"rub"</TableCellLayout></TableCell>
                                    </TableRow>
                                    <TableRow>
                                        <TableCell><TableCellLayout>"Скидка"</TableCellLayout></TableCell>
                                        <TableCell><TableCellLayout><code>"discount_percent"</code></TableCellLayout></TableCell>
                                        <TableCellMoney value=Signal::derive(move || line.discount_percent) show_currency=false color_by_sign=false />
                                        <TableCell><TableCellLayout>"%"</TableCellLayout></TableCell>
                                    </TableRow>
                                    <TableRow>
                                        <TableCell><TableCellLayout>"СПП"</TableCellLayout></TableCell>
                                        <TableCell><TableCellLayout><code>"spp"</code></TableCellLayout></TableCell>
                                        <TableCellMoney value=Signal::derive(move || line.spp) show_currency=false color_by_sign=false />
                                        <TableCell><TableCellLayout>"%"</TableCellLayout></TableCell>
                                    </TableRow>
                                </TableBody>
                            </Table>
                        </Card>

                        <Card>
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
                        </Card>

                        <Card>
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
                        </Card>
                    </div>

                    // Right column
                    <div>
                        <SalesDetailsCard vm=vm.clone() />
                    </div>
                </div>
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
                <Card>
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
                </Card>
            }.into_any()
        }}
    }
}
