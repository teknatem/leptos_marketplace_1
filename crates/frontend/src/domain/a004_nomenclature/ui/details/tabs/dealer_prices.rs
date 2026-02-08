//! Dealer prices tab - nested table with dealer prices from UT
//!
//! Shows prices for current nomenclature and base nomenclature (if derivative)

use super::super::view_model::NomenclatureDetailsVm;
use leptos::prelude::*;
use thaw::*;

/// Helper function to format price with 2 decimal places and thousand separators
fn format_price(price: f64) -> String {
    let formatted = format!("{:.2}", price);

    // Split into integer and decimal parts
    let parts: Vec<&str> = formatted.split('.').collect();
    if parts.len() != 2 {
        return formatted;
    }

    let integer_part = parts[0];
    let decimal_part = parts[1];

    // Add thousand separators to integer part
    let chars: Vec<char> = integer_part.chars().collect();
    let mut result = String::new();

    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(' ');
        }
        result.push(*ch);
    }

    // Combine with decimal part
    format!("{}.{}", result, decimal_part)
}

/// Dealer prices tab component with THAW Table
#[component]
pub fn DealerPricesTab(vm: NomenclatureDetailsVm) -> impl IntoView {
    let dealer_prices = vm.dealer_prices;
    let dealer_prices_loading = vm.dealer_prices_loading;

    view! {
        <div class="details-section">
            <h4 class="details-section__title">
                {move || format!("Дилерские цены ({})", dealer_prices.get().len())}
            </h4>

            // Loading state
            <Show when=move || dealer_prices_loading.get()>
                <div style="padding: var(--spacing-md); display: flex; align-items: center; gap: var(--spacing-sm);">
                    <Spinner size=SpinnerSize::Small />
                    <span style="color: var(--color-text-tertiary);">"Загрузка цен..."</span>
                </div>
            </Show>

            // Data table
            <Show when=move || !dealer_prices_loading.get()>
                <Show
                    when=move || !dealer_prices.get().is_empty()
                    fallback=|| view! {
                        <div style="padding: var(--spacing-md); color: var(--color-text-tertiary);">
                            "Нет данных о ценах"
                        </div>
                    }
                >
                    <Table>
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCell resizable=true min_width=120.0>"Дата"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=150.0>"Источник"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=120.0>"Цена"</TableHeaderCell>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            <For
                                each=move || dealer_prices.get()
                                key=|p| format!("{}_{}", p.period, p.nomenclature_ref)
                                children=move |price| {
                                    let badge_color = match price.source.as_str() {
                                        "Текущая" => BadgeColor::Brand,
                                        "Базовая" => BadgeColor::Informative,
                                        _ => BadgeColor::Brand,
                                    };
                                    view! {
                                        <TableRow>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {price.period.clone()}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <Badge appearance=BadgeAppearance::Tint color=badge_color>
                                                    {price.source.clone()}
                                                </Badge>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    <div style="text-align: right; width: 100%;">
                                                        {format_price(price.price)}
                                                    </div>
                                                </TableCellLayout>
                                            </TableCell>
                                        </TableRow>
                                    }
                                }
                            />
                        </TableBody>
                    </Table>
                </Show>
            </Show>
        </div>
    }
}
