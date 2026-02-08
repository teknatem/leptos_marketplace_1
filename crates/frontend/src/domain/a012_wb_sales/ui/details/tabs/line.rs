//! Line tab - line details, amounts table, and finance details

use super::super::view_model::WbSalesDetailsVm;
use leptos::prelude::*;
use thaw::*;

/// Line tab component - displays line info and amounts/finance tables
#[component]
pub fn LineTab(vm: WbSalesDetailsVm) -> impl IntoView {
    view! {
        {move || {
            let vm = vm.clone();
            let Some(sale_data) = vm.sale.get() else {
                return view! { <div>"Нет данных"</div> }.into_any();
            };

            let line = sale_data.line.clone();

            // Line field values
            let line_id = line.line_id.clone();
            let supplier_article = line.supplier_article.clone();
            let nm_id = line.nm_id.to_string();
            let barcode = line.barcode.clone();
            let name = line.name.clone();
            let qty = format!("{:.0}", line.qty);

            // Amounts rows data
            let amounts_rows: Vec<(String, String, String, String)> = vec![
                ("Полная цена".to_string(), "total_price".to_string(), line.total_price.map(|p| format!("{:.2}", p)).unwrap_or_else(|| "—".to_string()), "rub".to_string()),
                ("Процент скидки".to_string(), "discount_percent".to_string(), line.discount_percent.map(|d| format!("{:.1}", d)).unwrap_or_else(|| "—".to_string()), "%".to_string()),
                ("Цена без скидок".to_string(), "price_list".to_string(), line.price_list.map(|p| format!("{:.2}", p)).unwrap_or_else(|| "—".to_string()), "rub".to_string()),
                ("Сумма скидок".to_string(), "discount_total".to_string(), line.discount_total.map(|d| format!("{:.2}", d)).unwrap_or_else(|| "—".to_string()), "rub".to_string()),
                ("Цена после скидок".to_string(), "price_effective".to_string(), line.price_effective.map(|p| format!("{:.2}", p)).unwrap_or_else(|| "—".to_string()), "rub".to_string()),
                ("СПП".to_string(), "spp".to_string(), line.spp.map(|s| format!("{:.1}", s)).unwrap_or_else(|| "—".to_string()), "%".to_string()),
                ("Итоговая цена".to_string(), "finished_price".to_string(), line.finished_price.map(|p| format!("{:.2}", p)).unwrap_or_else(|| "—".to_string()), "rub".to_string()),
                ("Сумма платежа".to_string(), "payment_sale_amount".to_string(), line.payment_sale_amount.map(|p| format!("{:.2}", p)).unwrap_or_else(|| "—".to_string()), "rub".to_string()),
                ("К выплате".to_string(), "amount_line".to_string(), line.amount_line.map(|a| format!("{:.2}", a)).unwrap_or_else(|| "—".to_string()), "rub".to_string()),
            ];

            // Finance rows (if reports are loaded)
            let finance_reports = vm.finance_reports.get();
            let mut finance_rows: Vec<(usize, String, String, String)> = Vec::new();
            for (idx, report) in finance_reports.iter().enumerate() {
                let row_num = idx + 1;
                finance_rows.push((row_num, "Дата операции".to_string(), "rr_dt".to_string(), report.rr_dt.clone()));
                if let Some(v) = report.ppvz_vw { finance_rows.push((row_num, "Вознаграждение ВВ, без НДС".to_string(), "ppvz_vw".to_string(), format!("{:.2}", v))); }
                if let Some(v) = report.ppvz_vw_nds { finance_rows.push((row_num, "НДС с вознаграждения ВВ".to_string(), "ppvz_vw_nds".to_string(), format!("{:.2}", v))); }
                if let Some(v) = report.retail_amount { finance_rows.push((row_num, "WB реализовал товар".to_string(), "retail_amount".to_string(), format!("{:.2}", v))); }
                if let Some(v) = report.ppvz_for_pay { finance_rows.push((row_num, "К перечислению продавцу".to_string(), "ppvz_for_pay".to_string(), format!("{:.2}", v))); }
                if let Some(v) = report.commission_percent { finance_rows.push((row_num, "Размер кВВ, %".to_string(), "commission_percent".to_string(), format!("{:.2}", v))); }
                if let Some(v) = report.retail_price { finance_rows.push((row_num, "Цена розничная".to_string(), "retail_price".to_string(), format!("{:.2}", v))); }
                if let Some(v) = report.retail_price_withdisc_rub { finance_rows.push((row_num, "Цена с учетом скидки".to_string(), "retail_price_withdisc_rub".to_string(), format!("{:.2}", v))); }
                if let Some(v) = report.acquiring_fee { finance_rows.push((row_num, "Эквайринг".to_string(), "acquiring_fee".to_string(), format!("{:.2}", v))); }
            }
            let has_finance_data = !finance_rows.is_empty();

            view! {
                <div style="display: grid; grid-template-columns: 1200px; gap: var(--spacing-md); align-items: start; justify-items: start;">
                    // Line info card
                    <Card>
                        <h4 class="details-section__title">"Строка"</h4>
                        <div style="display: grid; grid-template-columns: 1fr 1fr 1fr; gap: var(--spacing-md);">
                            <div class="form__group">
                                <label class="form__label">"Line ID"</label>
                                <Input value=RwSignal::new(line_id) attr:readonly=true />
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Артикул продавца"</label>
                                <Input value=RwSignal::new(supplier_article) attr:readonly=true />
                            </div>
                            <div class="form__group">
                                <label class="form__label">"NM ID"</label>
                                <Input value=RwSignal::new(nm_id) attr:readonly=true />
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Штрихкод"</label>
                                <Input value=RwSignal::new(barcode) attr:readonly=true />
                            </div>
                            <div class="form__group" style="grid-column: span 2;">
                                <label class="form__label">"Название"</label>
                                <Input value=RwSignal::new(name) attr:readonly=true />
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Кол-во"</label>
                                <Input value=RwSignal::new(qty) attr:readonly=true />
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Дилерская цена УТ"</label>
                                <Flex gap=FlexGap::Small style="align-items: center;">
                                    <Input
                                        value=RwSignal::new(
                                            line.dealer_price_ut
                                                .map(|p| format!("{:.2}", p))
                                                .unwrap_or_else(|| "—".to_string())
                                        )
                                        attr:readonly=true
                                        attr:style="flex: 1;"
                                    />
                                    <Button
                                        appearance=ButtonAppearance::Secondary
                                        size=ButtonSize::Small
                                        on_click={
                                            let vm = vm.clone();
                                            move |_| vm.refresh_dealer_price()
                                        }
                                        disabled=Signal::derive({
                                            let vm = vm.clone();
                                            move || vm.refreshing_price.get()
                                        })
                                        attr:title="Обновить дилерскую цену из p906_nomenclature_prices"
                                    >
                                        {
                                            let vm = vm.clone();
                                            move || if vm.refreshing_price.get() { "..." } else { "↻" }
                                        }
                                    </Button>
                                </Flex>
                            </div>
                        </div>
                    </Card>

                    // Amounts table
                    <Card>
                        <h4 class="details-section__title">"Суммы и проценты"</h4>
                        <Table>
                            <TableHeader>
                                <TableRow>
                                    <TableHeaderCell>"Наименование"</TableHeaderCell>
                                    <TableHeaderCell>"Поле"</TableHeaderCell>
                                    <TableHeaderCell>"Значение"</TableHeaderCell>
                                    <TableHeaderCell>"Ед."</TableHeaderCell>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                <For
                                    each=move || amounts_rows.clone()
                                    key=|r| r.1.clone()
                                    children=move |(name, field, value, unit)| {
                                        view! {
                                            <TableRow>
                                                <TableCell><TableCellLayout>{name}</TableCellLayout></TableCell>
                                                <TableCell><TableCellLayout><code>{field}</code></TableCellLayout></TableCell>
                                                <TableCell><TableCellLayout>{value}</TableCellLayout></TableCell>
                                                <TableCell><TableCellLayout>{unit}</TableCellLayout></TableCell>
                                            </TableRow>
                                        }
                                    }
                                />
                            </TableBody>
                        </Table>
                    </Card>

                    // Finance details table (if data exists)
                    {if has_finance_data {
                        view! {
                            <Card>
                                <h4 class="details-section__title">"Финансовые детали"</h4>
                                <div style="max-height: 60vh; overflow: auto;">
                                    <Table>
                                        <TableHeader>
                                            <TableRow>
                                                <TableHeaderCell>"#"</TableHeaderCell>
                                                <TableHeaderCell>"Наименование"</TableHeaderCell>
                                                <TableHeaderCell>"Поле"</TableHeaderCell>
                                                <TableHeaderCell>"Значение"</TableHeaderCell>
                                            </TableRow>
                                        </TableHeader>
                                        <TableBody>
                                            <For
                                                each=move || finance_rows.clone()
                                                key=|r| format!("{}-{}-{}", r.0, r.2, r.1)
                                                children=move |(num, name, field, value)| {
                                                    view! {
                                                        <TableRow>
                                                            <TableCell><TableCellLayout>{num}</TableCellLayout></TableCell>
                                                            <TableCell><TableCellLayout>{name}</TableCellLayout></TableCell>
                                                            <TableCell><TableCellLayout><code>{field}</code></TableCellLayout></TableCell>
                                                            <TableCell><TableCellLayout>{value}</TableCellLayout></TableCell>
                                                        </TableRow>
                                                    }
                                                }
                                            />
                                        </TableBody>
                                    </Table>
                                </div>
                            </Card>
                        }.into_any()
                    } else {
                        view! { <></> }.into_any()
                    }}
                </div>
            }.into_any()
        }}
    }
}
