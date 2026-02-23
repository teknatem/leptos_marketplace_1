//! Plan/Fact tab - comparison table for financial metrics

use super::super::view_model::WbSalesDetailsVm;
use leptos::prelude::*;
use thaw::*;

/// Plan/Fact tab component - displays comparison of planned vs actual financial metrics
#[component]
pub fn PlanFactTab(vm: WbSalesDetailsVm) -> impl IntoView {
    view! {
        {move || {
            let Some(sale_data) = vm.sale.get() else {
                return view! { <div>"Нет данных"</div> }.into_any();
            };

            let line = sale_data.line.clone();

            // Helper function to format optional f64 values
            let fmt = |val: Option<f64>| {
                val.map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "—".to_string())
            };

            // Calculate differences
            let diff_sell_out = match (line.sell_out_fact, line.sell_out_plan) {
                (Some(f), Some(p)) => Some(f - p),
                _ => None,
            };
            let diff_acquiring = match (line.acquiring_fee_fact, line.acquiring_fee_plan) {
                (Some(f), Some(p)) => Some(f - p),
                _ => None,
            };
            let diff_other = match (line.other_fee_fact, line.other_fee_plan) {
                (Some(f), Some(p)) => Some(f - p),
                _ => None,
            };
            let diff_commission = match (line.commission_fact, line.commission_plan) {
                (Some(f), Some(p)) => Some(f - p),
                _ => None,
            };
            let diff_payout = match (line.supplier_payout_fact, line.supplier_payout_plan) {
                (Some(f), Some(p)) => Some(f - p),
                _ => None,
            };
            let diff_profit = match (line.profit_fact, line.profit_plan) {
                (Some(f), Some(p)) => Some(f - p),
                _ => None,
            };

            // Build table rows with all 6 columns
            let rows: Vec<(&str, &str, &str, String, String, String)> = vec![
                (
                    "Выручка",
                    "finished_price",
                    "retail_amount (P903)",
                    fmt(line.sell_out_plan),
                    fmt(line.sell_out_fact),
                    fmt(diff_sell_out),
                ),
                (
                    "Эквайринг",
                    "acquiring_fee_pro × finished_price",
                    "acquiring_fee (P903)",
                    fmt(line.acquiring_fee_plan),
                    fmt(line.acquiring_fee_fact),
                    fmt(diff_acquiring),
                ),
                (
                    "Прочие комиссии",
                    "0",
                    "rebill_logistic_cost (P903)",
                    fmt(line.other_fee_plan),
                    fmt(line.other_fee_fact),
                    fmt(diff_other),
                ),
                (
                    "Комиссия",
                    "finished_price - amount_line",
                    "ppvz_vw + ppvz_vw_nds (P903)",
                    fmt(line.commission_plan),
                    fmt(line.commission_fact),
                    fmt(diff_commission),
                ),
                (
                    "Выплата поставщику",
                    "finished_price - acquiring_fee_plan",
                    "ppvz_for_pay (P903)",
                    fmt(line.supplier_payout_plan),
                    fmt(line.supplier_payout_fact),
                    fmt(diff_payout),
                ),
                (
                    "Себестоимость",
                    "из номенклатуры",
                    "из номенклатуры",
                    fmt(line.cost_of_production),
                    fmt(line.cost_of_production),
                    "—".to_string(),
                ),
                (
                    "Прибыль",
                    "выручка - эквайринг - комиссия - прочие - себестоимость",
                    "retail - acquiring - commission - other - cost",
                    fmt(line.profit_plan),
                    fmt(line.profit_fact),
                    fmt(diff_profit),
                ),
            ];

            view! {
                <div style="max-width: 1000px;">
                    <Card>
                        <h4 class="details-section__title">"План/Факт сравнение"</h4>

                        <div style="margin-bottom: var(--spacing-md);">
                            {move || {
                                let is_fact = line.is_fact.unwrap_or(false);
                                view! {
                                    <Badge
                                        appearance=BadgeAppearance::Filled
                                        color=if is_fact { BadgeColor::Success } else { BadgeColor::Informative }
                                    >
                                        {if is_fact {
                                            "Документ содержит фактические данные (есть P903)"
                                        } else {
                                            "Документ содержит плановые данные (нет P903)"
                                        }}
                                    </Badge>
                                }
                            }}
                        </div>

                        <div style="overflow-x: auto;">
                            <Table>
                                <TableHeader>
                                    <TableRow>
                                        <TableHeaderCell>"Наименование"</TableHeaderCell>
                                        <TableHeaderCell attr:style="color: var(--colorBrandForeground2);">"Формула (План)"</TableHeaderCell>
                                        <TableHeaderCell>"Формула (Факт)"</TableHeaderCell>
                                        <TableHeaderCell max_width=100 attr:style="color: var(--colorBrandForeground2);">"План"</TableHeaderCell>
                                        <TableHeaderCell max_width=100>"Факт"</TableHeaderCell>
                                        <TableHeaderCell max_width=100>"Разница"</TableHeaderCell>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    <For
                                        each=move || rows.clone()
                                        key=|r| r.0.to_string()
                                        children=move |(name, formula_plan, formula_fact, plan, fact, diff)| {
                                            view! {
                                                <TableRow>
                                                    <TableCell>
                                                        <TableCellLayout>
                                                            <strong>{name}</strong>
                                                        </TableCellLayout>
                                                    </TableCell>
                                                    <TableCell>
                                                        <TableCellLayout>
                                                            <code style="font-size: 1em; color: var(--colorBrandForeground2);">{formula_plan}</code>
                                                        </TableCellLayout>
                                                    </TableCell>
                                                    <TableCell>
                                                        <TableCellLayout>
                                                            <code style="font-size: 1em;">{formula_fact}</code>
                                                        </TableCellLayout>
                                                    </TableCell>
                                                    <TableCell>
                                                        <TableCellLayout attr:style="justify-content: flex-end;">
                                                            <span style="color: var(--colorBrandForeground2);">{plan}</span>
                                                        </TableCellLayout>
                                                    </TableCell>
                                                    <TableCell>
                                                        <TableCellLayout attr:style="justify-content: flex-end;">
                                                            {fact}
                                                        </TableCellLayout>
                                                    </TableCell>
                                                    <TableCell>
                                                        <TableCellLayout attr:style="justify-content: flex-end;">
                                                            {diff}
                                                        </TableCellLayout>
                                                    </TableCell>
                                                </TableRow>
                                            }
                                        }
                                    />
                                </TableBody>
                            </Table>
                        </div>

                        <div>
                            <h5 style="margin: var(--spacing-sm);">"Как рассчитываются показатели"</h5>
                            <Flex vertical=false gap=FlexGap::Large justify=FlexJustify::Start align=FlexAlign::FlexStart>
                            <div style = "display: block;">
                                <p><strong>"План"</strong>" (когда нет данных P903, is_fact = false):"</p>
                                <ul style="margin-left: var(--spacing-lg);">
                                    <li>"sell_out_plan = finished_price"</li>
                                    <li>"acquiring_fee_plan = acquiring_fee_pro × finished_price"</li>
                                    <li>"other_fee_plan = 0"</li>
                                    <li>"commission_plan = finished_price - amount_line"</li>
                                    <li>"supplier_payout_plan = finished_price - acquiring_fee_plan"</li>
                                    <li>"profit_plan = finished_price - acquiring_fee_plan - commission_plan - other_fee_plan - cost_of_production"</li>
                                </ul>
                            </div>
                            <div style = "display: block;">
                                <p><strong>"Факт"</strong>" (когда есть данные P903, is_fact = true):"</p>
                                <ul style="margin-left: var(--spacing-lg);">
                                    <li>"sell_out_fact = retail_amount (из P903)"</li>
                                    <li>"acquiring_fee_fact = acquiring_fee (из P903)"</li>
                                    <li>"other_fee_fact = rebill_logistic_cost (из P903)"</li>
                                    <li>"commission_fact = ppvz_vw + ppvz_vw_nds (из P903)"</li>
                                    <li>"supplier_payout_fact = ppvz_for_pay (из P903)"</li>
                                    <li>"profit_fact = retail_amount - acquiring_fee - commission - other_fee - cost_of_production"</li>
                                </ul>
                            </div>
                            </Flex>
                        </div>
                    </Card>
                </div>
            }.into_any()
        }}
    }
}
