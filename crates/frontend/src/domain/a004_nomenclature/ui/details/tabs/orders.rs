use super::super::view_model::NomenclatureDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::card_animated::CardAnimated;
use contracts::domain::a004_nomenclature::orders_dto::NomenclatureOrderRowDto;
use leptos::prelude::*;
use thaw::*;

fn format_date(value: &Option<String>) -> String {
    let Some(value) = value else {
        return "—".to_string();
    };

    let date_part = value.split_once('T').map(|(d, _)| d).unwrap_or(value);
    let parts: Vec<&str> = date_part.split('-').collect();
    if parts.len() == 3 {
        format!("{}.{}.{}", parts[2], parts[1], parts[0])
    } else {
        date_part.to_string()
    }
}

fn format_price(price: Option<f64>) -> String {
    let Some(price) = price else {
        return "—".to_string();
    };

    let formatted = format!("{price:.2}");
    let parts: Vec<&str> = formatted.split('.').collect();
    if parts.len() != 2 {
        return formatted;
    }

    let integer_part = parts[0];
    let decimal_part = parts[1];
    let chars: Vec<char> = integer_part.chars().collect();
    let mut result = String::new();

    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(' ');
        }
        result.push(*ch);
    }

    format!("{}.{}", result, decimal_part)
}

fn order_tab_key(item: &NomenclatureOrderRowDto) -> Option<String> {
    match item.marketplace.as_str() {
        "WB" => Some(format!("a015_wb_orders_details_{}", item.id)),
        "YM" => Some(format!("a013_ym_order_details_{}", item.id)),
        _ => None,
    }
}

fn order_tab_title(item: &NomenclatureOrderRowDto) -> String {
    match item.marketplace.as_str() {
        "WB" => format!("Заказ WB {}", item.document_no),
        "YM" => format!("Заказ YM {}", item.document_no),
        _ => item.document_no.clone(),
    }
}

fn status_label(item: &NomenclatureOrderRowDto) -> String {
    match item.marketplace.as_str() {
        "WB" => {
            if item.is_cancel == Some(true) {
                "Отменён".to_string()
            } else if item.is_realization == Some(true) {
                "Реализован".to_string()
            } else if item.is_supply == Some(true) {
                "В поставке".to_string()
            } else {
                "Новый".to_string()
            }
        }
        "YM" => item
            .line_status
            .clone()
            .or_else(|| item.status_norm.clone())
            .unwrap_or_else(|| "—".to_string()),
        _ => "—".to_string(),
    }
}

fn status_badge_color(label: &str) -> BadgeColor {
    match label {
        "Отменён" | "RETURNED" | "REJECTED" => BadgeColor::Danger,
        "Реализован" | "DELIVERED" => BadgeColor::Success,
        _ => BadgeColor::Informative,
    }
}

fn discount(item: &NomenclatureOrderRowDto) -> Option<f64> {
    match (item.price_before_discount, item.price_after_discount) {
        (Some(a), Some(b)) => Some(a - b),
        _ => None,
    }
}

#[component]
pub fn OrdersTab(vm: NomenclatureDetailsVm) -> impl IntoView {
    let orders = vm.orders;
    let orders_loading = vm.orders_loading;
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    view! {
        <CardAnimated delay_ms=0 nav_id="a004_nomenclature_details_orders_main">
            <h4 class="details-section__title">
                {move || format!(
                    "Заказы маркетплейсов ({}) — за последние 180 дней",
                    orders.get().len(),
                )}
            </h4>

            <Show when=move || orders_loading.get()>
                <div style="padding: var(--spacing-md); display: flex; align-items: center; gap: var(--spacing-sm);">
                    <Spinner size=SpinnerSize::Small />
                    <span style="color: var(--color-text-tertiary);">"Загрузка заказов..."</span>
                </div>
            </Show>

            <Show when=move || !orders_loading.get()>
                <Show
                    when=move || !orders.get().is_empty()
                    fallback=|| view! {
                        <div style="padding: var(--spacing-md); color: var(--color-text-tertiary);">
                            "Нет заказов за выбранный период"
                        </div>
                    }
                >
                    <Table>
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCell resizable=true min_width=60.0>"МП"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=100.0>"Дата"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=160.0>"Документ"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=110.0>"Статус"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=70.0>"Кол-во"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=120.0>"Цена без скидки"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=100.0>"Скидка"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=120.0>"Цена после скидки"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=140.0>"Итоговая цена покупателю"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=110.0>"Дилерская цена"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=90.0>"Маржа, %"</TableHeaderCell>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            <For
                                each=move || orders.get()
                                key=|item| format!("{}-{}", item.marketplace, item.id)
                                children={
                                    let tabs_store = tabs_store.clone();
                                    move |item| {
                                        let tab_key = order_tab_key(&item);
                                        let title = order_tab_title(&item);
                                        let label = item.document_no.clone();
                                        let status = status_label(&item);
                                        let status_color = status_badge_color(&status);
                                        let marketplace = item.marketplace.clone();
                                        let date_str = format_date(&item.order_date);
                                        let qty_str = format!("{:.0}", item.qty);
                                        let price_before_str = format_price(item.price_before_discount);
                                        let discount_str = format_price(discount(&item));
                                        let price_after_str = format_price(item.price_after_discount);
                                        let final_price_str = format_price(item.final_buyer_price);
                                        let dealer_price_str = format_price(item.dealer_price_ut);
                                        let margin_str = format_price(item.margin_pro);

                                        view! {
                                            <TableRow>
                                                <TableCell>
                                                    <TableCellLayout truncate=true>
                                                        {marketplace}
                                                    </TableCellLayout>
                                                </TableCell>
                                                <TableCell>
                                                    <TableCellLayout truncate=true>
                                                        {date_str}
                                                    </TableCellLayout>
                                                </TableCell>
                                                <TableCell>
                                                    <TableCellLayout truncate=true>
                                                        {if let Some(tab_key) = tab_key {
                                                            view! {
                                                                <a
                                                                    href="#"
                                                                    style="color: var(--color-primary); text-decoration: underline; cursor: pointer;"
                                                                    on:click={
                                                                        let title = title.clone();
                                                                        let tabs_store = tabs_store.clone();
                                                                        move |ev: web_sys::MouseEvent| {
                                                                            ev.prevent_default();
                                                                            tabs_store.open_tab(&tab_key, &title);
                                                                        }
                                                                    }
                                                                >
                                                                    {label.clone()}
                                                                </a>
                                                            }.into_any()
                                                        } else {
                                                            view! { <span>{label.clone()}</span> }.into_any()
                                                        }}
                                                    </TableCellLayout>
                                                </TableCell>
                                                <TableCell>
                                                    <TableCellLayout truncate=true>
                                                        <Badge appearance=BadgeAppearance::Tint color=status_color>
                                                            {status.clone()}
                                                        </Badge>
                                                    </TableCellLayout>
                                                </TableCell>
                                                <TableCell>
                                                    <TableCellLayout truncate=true>
                                                        <div style="text-align: right; width: 100%; font-variant-numeric: tabular-nums;">
                                                            {qty_str}
                                                        </div>
                                                    </TableCellLayout>
                                                </TableCell>
                                                <TableCell>
                                                    <TableCellLayout truncate=true>
                                                        <div style="text-align: right; width: 100%; font-variant-numeric: tabular-nums;">
                                                            {price_before_str}
                                                        </div>
                                                    </TableCellLayout>
                                                </TableCell>
                                                <TableCell>
                                                    <TableCellLayout truncate=true>
                                                        <div style="text-align: right; width: 100%; font-variant-numeric: tabular-nums;">
                                                            {discount_str}
                                                        </div>
                                                    </TableCellLayout>
                                                </TableCell>
                                                <TableCell>
                                                    <TableCellLayout truncate=true>
                                                        <div style="text-align: right; width: 100%; font-variant-numeric: tabular-nums;">
                                                            {price_after_str}
                                                        </div>
                                                    </TableCellLayout>
                                                </TableCell>
                                                <TableCell>
                                                    <TableCellLayout truncate=true>
                                                        <div style="text-align: right; width: 100%; font-variant-numeric: tabular-nums;">
                                                            {final_price_str}
                                                        </div>
                                                    </TableCellLayout>
                                                </TableCell>
                                                <TableCell>
                                                    <TableCellLayout truncate=true>
                                                        <div style="text-align: right; width: 100%; font-variant-numeric: tabular-nums;">
                                                            {dealer_price_str}
                                                        </div>
                                                    </TableCellLayout>
                                                </TableCell>
                                                <TableCell>
                                                    <TableCellLayout truncate=true>
                                                        <div style="text-align: right; width: 100%; font-variant-numeric: tabular-nums;">
                                                            {margin_str}
                                                        </div>
                                                    </TableCellLayout>
                                                </TableCell>
                                            </TableRow>
                                        }
                                    }
                                }
                            />
                        </TableBody>
                    </Table>
                </Show>
            </Show>
        </CardAnimated>
    }
}
