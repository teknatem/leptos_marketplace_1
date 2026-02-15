//! Projections tab for YM Order

use super::super::view_model::YmOrderDetailsVm;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn ProjectionsTab(vm: YmOrderDetailsVm) -> impl IntoView {
    view! {
        {move || {
            if vm.projections_loading.get() {
                return view! {
                    <Card>
                        <Flex gap=FlexGap::Small style="align-items: center; justify-content: center; padding: var(--spacing-xl);">
                            <Spinner />
                            <span>"Загрузка проекций..."</span>
                        </Flex>
                    </Card>
                }
                .into_any();
            }

            let Some(proj_data) = vm.projections.get() else {
                return view! {
                    <Card>
                        <div style="color: var(--color-text-secondary);">
                            "Проекции не загружены"
                        </div>
                    </Card>
                }
                .into_any();
            };

            let p900_items = proj_data["p900_sales_register"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            let p904_items = proj_data["p904_sales_data"].as_array().cloned().unwrap_or_default();

            view! {
                <div style="display: grid; grid-template-columns: 1fr; gap: var(--spacing-md);">
                    <Card>
                        <h4 class="details-section__title">
                            {format!("Sales Register (p900) - {} записей", p900_items.len())}
                        </h4>
                        <div style="overflow-x: auto;">
                            <Table>
                                <TableHeader>
                                    <TableRow>
                                        <TableHeaderCell>"MP"</TableHeaderCell>
                                        <TableHeaderCell>"SKU"</TableHeaderCell>
                                        <TableHeaderCell>"Title"</TableHeaderCell>
                                        <TableHeaderCell>"Qty"</TableHeaderCell>
                                        <TableHeaderCell>"Amount"</TableHeaderCell>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    <For
                                        each=move || p900_items.clone()
                                        key=|item| {
                                            format!(
                                                "{}-{}",
                                                item["seller_sku"].as_str().unwrap_or(""),
                                                item["title"].as_str().unwrap_or("")
                                            )
                                        }
                                        children=move |item| {
                                            let mp = item["marketplace"].as_str().unwrap_or("—").to_string();
                                            let sku = item["seller_sku"].as_str().unwrap_or("—").to_string();
                                            let title = item["title"].as_str().unwrap_or("—").to_string();
                                            let qty = item["qty"].as_f64().unwrap_or(0.0);
                                            let amount = item["amount_line"].as_f64().unwrap_or(0.0);
                                            view! {
                                                <TableRow>
                                                    <TableCell><TableCellLayout>{mp}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout><code>{sku}</code></TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout truncate=true>{title}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{format!("{:.0}", qty)}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{format!("{:.2}", amount)}</TableCellLayout></TableCell>
                                                </TableRow>
                                            }
                                        }
                                    />
                                </TableBody>
                            </Table>
                        </div>
                    </Card>

                    <Card>
                        <h4 class="details-section__title">
                            {format!("Sales Data (p904) - {} записей", p904_items.len())}
                        </h4>
                        <div style="overflow-x: auto;">
                            <Table>
                                <TableHeader>
                                    <TableRow>
                                        <TableHeaderCell>"Article"</TableHeaderCell>
                                        <TableHeaderCell>"Price List"</TableHeaderCell>
                                        <TableHeaderCell>"Customer In"</TableHeaderCell>
                                        <TableHeaderCell>"Customer Out"</TableHeaderCell>
                                        <TableHeaderCell>"Commission Out"</TableHeaderCell>
                                        <TableHeaderCell>"Acquiring Out"</TableHeaderCell>
                                        <TableHeaderCell>"Total"</TableHeaderCell>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    <For
                                        each=move || p904_items.clone()
                                        key=|item| item["article"].as_str().unwrap_or("").to_string()
                                        children=move |item| {
                                            let article = item["article"].as_str().unwrap_or("—").to_string();
                                            let price_list = item["price_list"].as_f64().unwrap_or(0.0);
                                            let customer_in = item["customer_in"].as_f64().unwrap_or(0.0);
                                            let customer_out = item["customer_out"].as_f64().unwrap_or(0.0);
                                            let commission_out = item["commission_out"].as_f64().unwrap_or(0.0);
                                            let acquiring_out = item["acquiring_out"].as_f64().unwrap_or(0.0);
                                            let total = item["total"].as_f64().unwrap_or(0.0);
                                            view! {
                                                <TableRow>
                                                    <TableCell><TableCellLayout><code>{article}</code></TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{format!("{:.2}", price_list)}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{format!("{:.2}", customer_in)}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{format!("{:.2}", customer_out)}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{format!("{:.2}", commission_out)}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{format!("{:.2}", acquiring_out)}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{format!("{:.2}", total)}</TableCellLayout></TableCell>
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
