//! Links tab - linked finance reports

use super::super::view_model::WbSalesDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use contracts::projections::p903_wb_finance_report::dto::WbFinanceReportDto;
use leptos::prelude::*;
use thaw::*;

/// Links tab component - displays linked finance reports
#[component]
pub fn LinksTab(vm: WbSalesDetailsVm) -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    view! {
        {move || {
            if vm.finance_reports_loading.get() {
                return view! {
                    <Card>
                        <Flex gap=FlexGap::Small style="align-items: center; justify-content: center; padding: var(--spacing-xl);">
                            <Spinner />
                            <span>"Загрузка финансовых отчетов..."</span>
                        </Flex>
                    </Card>
                }.into_any();
            }

            if let Some(err) = vm.finance_reports_error.get() {
                return view! {
                    <Card>
                        <div style="color: var(--color-error);">
                            "Ошибка загрузки: " {err}
                        </div>
                    </Card>
                }.into_any();
            }

            let reports = vm.finance_reports.get();
            if reports.is_empty() {
                return view! {
                    <Card>
                        <h4 class="details-section__title">"Финансовые отчеты"</h4>
                        <div style="color: var(--color-text-secondary);">
                            "Связанные финансовые отчеты не найдены для данного SRID."
                        </div>
                    </Card>
                }.into_any();
            }

            // Calculate totals
            let reports_count = reports.len();
            let total_ppvz_vw: f64 = reports.iter().filter_map(|r| r.ppvz_vw).sum();
            let total_ppvz_vw_nds: f64 = reports.iter().filter_map(|r| r.ppvz_vw_nds).sum();
            let total_retail: f64 = reports.iter().filter_map(|r| r.retail_amount).sum();
            let total_ppvz_for_pay: f64 = reports.iter().filter_map(|r| r.ppvz_for_pay).sum();
            let total_acquiring: f64 = reports.iter().filter_map(|r| r.acquiring_fee).sum();

            // Clone for use in For loop
            let reports_for_table = reports;

            view! {
                <Card>
                    <h4 class="details-section__title">"Финансовые отчеты"</h4>

                    // Summary badges
                    <Flex gap=FlexGap::Medium style="flex-wrap: wrap; margin-bottom: var(--spacing-md);">
                        <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                            {format!("Найдено: {}", reports_count)}
                        </Badge>
                        <span>"PPVZ VW: " <strong>{format!("{:.2}", total_ppvz_vw)}</strong></span>
                        <span>"PPVZ VW NDS: " <strong>{format!("{:.2}", total_ppvz_vw_nds)}</strong></span>
                        <span>"Retail: " <strong>{format!("{:.2}", total_retail)}</strong></span>
                        <span>"For Pay: " <strong>{format!("{:.2}", total_ppvz_for_pay)}</strong></span>
                        <span>"Acquiring: " <strong>{format!("{:.2}", total_acquiring)}</strong></span>
                    </Flex>

                    // Table
                    <div style="max-height: calc(100vh - 400px); overflow: auto;">
                        <Table>
                            <TableHeader>
                                <TableRow>
                                    <TableHeaderCell>"Date (rr_dt)"</TableHeaderCell>
                                    <TableHeaderCell>"RRD ID"</TableHeaderCell>
                                    <TableHeaderCell>"PPVZ VW"</TableHeaderCell>
                                    <TableHeaderCell>"PPVZ VW NDS"</TableHeaderCell>
                                    <TableHeaderCell>"Retail Amount"</TableHeaderCell>
                                    <TableHeaderCell>"PPVZ For Pay"</TableHeaderCell>
                                    <TableHeaderCell>"Commission %"</TableHeaderCell>
                                    <TableHeaderCell>"Retail Price"</TableHeaderCell>
                                    <TableHeaderCell>"Retail w/Disc"</TableHeaderCell>
                                    <TableHeaderCell>"Acquiring Fee"</TableHeaderCell>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                <For
                                    each=move || reports_for_table.clone()
                                    key=|r| format!("{}_{}", r.rr_dt, r.rrd_id)
                                    children={
                                        let tabs_store = tabs_store;
                                        move |report: WbFinanceReportDto| {
                                            let rr_dt = report.rr_dt.clone();
                                            let rrd_id = report.rrd_id;
                                            let rr_dt_for_click = rr_dt.clone();

                                            view! {
                                                <TableRow
                                                    on:click={
                                                        let tabs_store = tabs_store;
                                                        move |_| {
                                                            let rr_dt_encoded = urlencoding::encode(&rr_dt_for_click).into_owned();
                                                            let tab_key = format!(
                                                                "p903_wb_finance_report_detail_{}__{}",
                                                                rr_dt_encoded, rrd_id
                                                            );
                                                            let tab_title = format!("WB FR {} #{}", rr_dt_for_click, rrd_id);
                                                            tabs_store.open_tab(&tab_key, &tab_title);
                                                        }
                                                    }
                                                    attr:style="cursor: pointer;"
                                                >
                                                    <TableCell><TableCellLayout>{rr_dt}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{rrd_id}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{report.ppvz_vw.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "—".to_string())}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{report.ppvz_vw_nds.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "—".to_string())}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{report.retail_amount.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "—".to_string())}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{report.ppvz_for_pay.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "—".to_string())}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{report.commission_percent.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "—".to_string())}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{report.retail_price.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "—".to_string())}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{report.retail_price_withdisc_rub.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "—".to_string())}</TableCellLayout></TableCell>
                                                    <TableCell><TableCellLayout>{report.acquiring_fee.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "—".to_string())}</TableCellLayout></TableCell>
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
