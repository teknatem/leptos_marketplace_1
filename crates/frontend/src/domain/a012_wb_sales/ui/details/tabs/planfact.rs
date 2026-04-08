use super::super::view_model::WbSalesDetailsVm;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn PlanFactTab(vm: WbSalesDetailsVm) -> impl IntoView {
    view! {
        {move || {
            let Some(sale_data) = vm.sale.get() else {
                return view! { <div>"Нет данных"</div> }.into_any();
            };

            let line = sale_data.line.clone();
            let reports = vm.finance_reports.get();
            let target_oper_name =
                if sale_data.is_customer_return || line.finished_price.unwrap_or(0.0) < 0.0 {
                    "Возврат"
                } else {
                    "Продажа"
                };
            let sign = if target_oper_name == "Возврат" {
                -1.0
            } else {
                1.0
            };

            let relevant_reports: Vec<_> = reports
                .into_iter()
                .filter(|item| item.supplier_oper_name.as_deref() == Some(target_oper_name))
                .collect();
            let has_fact_data = relevant_reports
                .iter()
                .any(|item| item.retail_amount.unwrap_or(0.0) != 0.0);

            let sell_out_fact = has_fact_data.then(|| {
                relevant_reports
                    .iter()
                    .filter_map(|item| item.retail_amount)
                    .sum::<f64>()
                    * sign
            });
            let acquiring_fee_fact = has_fact_data.then(|| {
                relevant_reports
                    .iter()
                    .filter_map(|item| item.acquiring_fee)
                    .sum::<f64>()
                    * sign
            });
            let other_fee_fact = has_fact_data.then(|| {
                relevant_reports
                    .iter()
                    .filter_map(|item| item.rebill_logistic_cost)
                    .sum::<f64>()
                    * sign
            });
            let commission_fact = has_fact_data.then(|| {
                relevant_reports
                    .iter()
                    .map(|item| item.ppvz_vw.unwrap_or(0.0) + item.ppvz_vw_nds.unwrap_or(0.0))
                    .sum::<f64>()
                    * sign
            });
            let supplier_payout_fact = has_fact_data.then(|| {
                relevant_reports
                    .iter()
                    .filter_map(|item| item.ppvz_for_pay)
                    .sum::<f64>()
                    * sign
            });
            let profit_fact = match (
                sell_out_fact,
                acquiring_fee_fact,
                other_fee_fact,
                commission_fact,
                line.cost_of_production,
            ) {
                (Some(sell_out), Some(acquiring), Some(other), Some(commission), Some(cost)) => {
                    Some(sell_out - acquiring - other - commission - cost)
                }
                _ => None,
            };

            let fmt = |val: Option<f64>| {
                val.map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "—".to_string())
            };

            let diff_sell_out = match (sell_out_fact, line.sell_out_plan) {
                (Some(f), Some(p)) => Some(f - p),
                _ => None,
            };
            let diff_acquiring = match (acquiring_fee_fact, line.acquiring_fee_plan) {
                (Some(f), Some(p)) => Some(f - p),
                _ => None,
            };
            let diff_other = match (other_fee_fact, line.other_fee_plan) {
                (Some(f), Some(p)) => Some(f - p),
                _ => None,
            };
            let diff_commission = match (commission_fact, line.commission_plan) {
                (Some(f), Some(p)) => Some(f - p),
                _ => None,
            };
            let diff_payout = match (supplier_payout_fact, line.supplier_payout_plan) {
                (Some(f), Some(p)) => Some(f - p),
                _ => None,
            };
            let diff_profit = match (profit_fact, line.profit_plan) {
                (Some(f), Some(p)) => Some(f - p),
                _ => None,
            };

            let rows: Vec<(&str, &str, &str, String, String, String)> = vec![
                (
                    "Выручка",
                    "finished_price",
                    "retail_amount (P903)",
                    fmt(line.sell_out_plan),
                    fmt(sell_out_fact),
                    fmt(diff_sell_out),
                ),
                (
                    "Эквайринг",
                    "acquiring_fee_pro * finished_price",
                    "acquiring_fee (P903)",
                    fmt(line.acquiring_fee_plan),
                    fmt(acquiring_fee_fact),
                    fmt(diff_acquiring),
                ),
                (
                    "Прочие комиссии",
                    "0",
                    "rebill_logistic_cost (P903)",
                    fmt(line.other_fee_plan),
                    fmt(other_fee_fact),
                    fmt(diff_other),
                ),
                (
                    "Комиссия",
                    "finished_price - amount_line",
                    "ppvz_vw + ppvz_vw_nds (P903)",
                    fmt(line.commission_plan),
                    fmt(commission_fact),
                    fmt(diff_commission),
                ),
                (
                    "Выплата поставщику",
                    "amount_line - acquiring_fee_plan",
                    "ppvz_for_pay (P903)",
                    fmt(line.supplier_payout_plan),
                    fmt(supplier_payout_fact),
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
                    "sell_out - acquiring - commission - other - cost",
                    "retail - acquiring - commission - other - cost",
                    fmt(line.profit_plan),
                    fmt(profit_fact),
                    fmt(diff_profit),
                ),
            ];

            view! {
                <div style="max-width: 1000px;">
                    <Card>
                        <h4 class="details-section__title">"План/Факт сравнение"</h4>

                        <div style="margin-bottom: var(--spacing-md);">
                            <Badge
                                appearance=BadgeAppearance::Filled
                                color=if has_fact_data {
                                    BadgeColor::Success
                                } else {
                                    BadgeColor::Informative
                                }
                            >
                                {if has_fact_data {
                                    "Факт подтянут lazy из P903"
                                } else {
                                    "Доступен только план"
                                }}
                            </Badge>
                        </div>

                        <div style="overflow-x: auto;">
                            <Table>
                                <TableHeader>
                                    <TableRow>
                                        <TableHeaderCell>"Наименование"</TableHeaderCell>
                                        <TableHeaderCell>"Формула (План)"</TableHeaderCell>
                                        <TableHeaderCell>"Формула (Факт)"</TableHeaderCell>
                                        <TableHeaderCell>"План"</TableHeaderCell>
                                        <TableHeaderCell>"Факт"</TableHeaderCell>
                                        <TableHeaderCell>"Разница"</TableHeaderCell>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    <For
                                        each=move || rows.clone()
                                        key=|row| row.0.to_string()
                                        children=move |(name, formula_plan, formula_fact, plan, fact, diff)| {
                                            view! {
                                                <TableRow>
                                                    <TableCell><TableCellLayout><strong>{name}</strong></TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout><code>{formula_plan}</code></TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout><code>{formula_fact}</code></TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{plan}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{fact}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{diff}</TableCellLayout></TableCell>
                                                </TableRow>
                                            }
                                        }
                                    />
                                </TableBody>
                            </Table>
                        </div>
                    </Card>
                </div>
            }
            .into_any()
        }}
    }
}
