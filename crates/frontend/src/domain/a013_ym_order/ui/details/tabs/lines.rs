//! Lines tab for YM Order

use super::super::view_model::YmOrderDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn LinesTab(vm: YmOrderDetailsVm) -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    view! {
        {move || {
            let Some(order_data) = vm.order.get() else {
                return view! { <div>"Нет данных"</div> }.into_any();
            };

            let lines = order_data.lines.clone();
            let lines_count = lines.len();
            let lines_for_table = lines.clone();
            let total_qty: f64 = lines.iter().map(|l| l.qty).sum();
            let total_amount: f64 = lines.iter().filter_map(|l| l.amount_line).sum();
            let lines_without_nomenclature = lines.iter().filter(|l| l.nomenclature_ref.is_none()).count();

            view! {
                <Card>
                    <h4 class="details-section__title">"Строки заказа"</h4>
                    <Flex gap=FlexGap::Medium style="margin-bottom: var(--spacing-md); flex-wrap: wrap;">
                        <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                            {format!("Строк: {}", lines_count)}
                        </Badge>
                        <span>"Qty: " <strong>{format!("{:.0}", total_qty)}</strong></span>
                        <span>"Amount: " <strong>{format!("{:.2}", total_amount)}</strong></span>
                        <Badge
                            appearance=BadgeAppearance::Filled
                            color={if lines_without_nomenclature > 0 { BadgeColor::Danger } else { BadgeColor::Success }}
                        >
                            {if lines_without_nomenclature > 0 {
                                format!("Без номенклатуры: {}", lines_without_nomenclature)
                            } else {
                                "Все строки сопоставлены".to_string()
                            }}
                        </Badge>
                    </Flex>

                    <div style="overflow-x: auto;">
                        <Table>
                            <TableHeader>
                                <TableRow>
                                    <TableHeaderCell>"Shop SKU"</TableHeaderCell>
                                    <TableHeaderCell>"Offer ID"</TableHeaderCell>
                                    <TableHeaderCell>"Название"</TableHeaderCell>
                                    <TableHeaderCell>"Qty"</TableHeaderCell>
                                    <TableHeaderCell>"Цена"</TableHeaderCell>
                                    <TableHeaderCell>"Сумма"</TableHeaderCell>
                                    <TableHeaderCell>"Товар МП"</TableHeaderCell>
                                    <TableHeaderCell>"Номенклатура"</TableHeaderCell>
                                    <TableHeaderCell>"Статус"</TableHeaderCell>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                <For
                                    each=move || lines_for_table.clone()
                                    key=|line| line.line_id.clone()
                                    children=move |line| {
                                        let line_id = line.line_id.clone();
                                        let line_id_for_mp = line_id.clone();
                                        let line_id_for_nom = line_id.clone();
                                        let nom_ref = line.nomenclature_ref.clone();
                                        let mp_ref = line.marketplace_product_ref.clone();
                                        let nom_map = vm.nomenclatures_info;
                                        let mp_map = vm.marketplace_products_info;

                                        view! {
                                            <TableRow>
                                                <TableCell><TableCellLayout><code>{line.shop_sku.clone()}</code></TableCellLayout></TableCell>
                                                <TableCell><TableCellLayout><code>{line.offer_id.clone()}</code></TableCellLayout></TableCell>
                                                <TableCell><TableCellLayout truncate=true>{line.name.clone()}</TableCellLayout></TableCell>
                                                <TableCell><TableCellLayout>{format!("{:.0}", line.qty)}</TableCellLayout></TableCell>
                                                <TableCell><TableCellLayout>{line.price_effective.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "—".to_string())}</TableCellLayout></TableCell>
                                                <TableCell><TableCellLayout>{line.amount_line.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "—".to_string())}</TableCellLayout></TableCell>
                                                <TableCell>
                                                    <TableCellLayout truncate=true>
                                                        {move || {
                                                            if let Some(ref id) = mp_ref {
                                                                let text = mp_map
                                                                    .get()
                                                                    .get(&line_id_for_mp)
                                                                    .map(|i| if i.article.is_empty() {
                                                                        i.description.clone()
                                                                    } else {
                                                                        format!("{} (арт. {})", i.description, i.article)
                                                                    })
                                                                    .unwrap_or_else(|| "Открыть".to_string());
                                                                view! {
                                                                    <a
                                                                        href="#"
                                                                        on:click={
                                                                            let id = id.clone();
                                                                            move |e: web_sys::MouseEvent| {
                                                                                e.prevent_default();
                                                                                tabs_store.open_tab(
                                                                                    &format!("a007_marketplace_product_detail_{}", id),
                                                                                    "Товар МП",
                                                                                );
                                                                            }
                                                                        }
                                                                    >
                                                                        {text}
                                                                    </a>
                                                                }
                                                                .into_any()
                                                            } else {
                                                                view! { <span>"—"</span> }.into_any()
                                                            }
                                                        }}
                                                    </TableCellLayout>
                                                </TableCell>
                                                <TableCell>
                                                    <TableCellLayout truncate=true>
                                                        {move || {
                                                            if let Some(ref id) = nom_ref {
                                                                let text = nom_map
                                                                    .get()
                                                                    .get(&line_id_for_nom)
                                                                    .map(|i| if i.article.is_empty() {
                                                                        i.description.clone()
                                                                    } else {
                                                                        format!("{} (арт. {})", i.description, i.article)
                                                                    })
                                                                    .unwrap_or_else(|| "Открыть".to_string());
                                                                view! {
                                                                    <a
                                                                        href="#"
                                                                        on:click={
                                                                            let id = id.clone();
                                                                            move |e: web_sys::MouseEvent| {
                                                                                e.prevent_default();
                                                                                tabs_store.open_tab(
                                                                                    &format!("a004_nomenclature_detail_{}", id),
                                                                                    "Номенклатура",
                                                                                );
                                                                            }
                                                                        }
                                                                    >
                                                                        {text}
                                                                    </a>
                                                                }
                                                                .into_any()
                                                            } else {
                                                                view! { <span>"—"</span> }.into_any()
                                                            }
                                                        }}
                                                    </TableCellLayout>
                                                </TableCell>
                                                <TableCell><TableCellLayout>{line.status.unwrap_or_else(|| "—".to_string())}</TableCellLayout></TableCell>
                                            </TableRow>
                                        }
                                    }
                                />
                            </TableBody>
                        </Table>
                    </div>
                </Card>
            }
            .into_any()
        }}
    }
}
