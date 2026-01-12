//! Barcodes tab - nested table with barcodes from various sources
//!
//! Shows barcodes associated with the nomenclature from WB, OZON, YM, 1C

use super::super::view_model::NomenclatureDetailsVm;
use leptos::prelude::*;
use thaw::*;

/// Barcodes tab component with THAW Table
#[component]
pub fn BarcodesTab(vm: NomenclatureDetailsVm) -> impl IntoView {
    let barcodes = vm.barcodes;
    let barcodes_count = vm.barcodes_count;
    let barcodes_loading = vm.barcodes_loading;

    view! {
        <div class="details-section">
            <h4 class="details-section__title">
                {move || format!("Штрихкоды ({})", barcodes_count.get())}
            </h4>

            // Loading state
            <Show when=move || barcodes_loading.get()>
                <div style="padding: var(--spacing-md); display: flex; align-items: center; gap: var(--spacing-sm);">
                    <Spinner size=SpinnerSize::Small />
                    <span style="color: var(--color-text-tertiary);">"Загрузка штрихкодов..."</span>
                </div>
            </Show>

            // Data table
            <Show when=move || !barcodes_loading.get()>
                <Show
                    when=move || !barcodes.get().is_empty()
                    fallback=|| view! {
                        <div style="padding: var(--spacing-md); color: var(--color-text-tertiary);">
                            "Нет штрихкодов"
                        </div>
                    }
                >
                    <Table>
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCell resizable=true min_width=180.0>"Штрихкод"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=110.0>"Источник"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=120.0>"Артикул"</TableHeaderCell>
                                <TableHeaderCell resizable=true min_width=160.0>"Обновлено"</TableHeaderCell>
                                <TableHeaderCell resizable=false>"Активен"</TableHeaderCell>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            <For
                                each=move || barcodes.get()
                                key=|b| b.barcode.clone()
                                children=move |barcode| {
                                    let badge_color = match barcode.source.as_str() {
                                        "WB" => BadgeColor::Important,
                                        "OZON" => BadgeColor::Brand,
                                        "YM" => BadgeColor::Warning,
                                        "1C" => BadgeColor::Success,
                                        _ => BadgeColor::Brand,
                                    };
                                    view! {
                                        <TableRow>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {barcode.barcode.clone()}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <Badge appearance=BadgeAppearance::Tint color=badge_color>
                                                    {barcode.source.clone()}
                                                </Badge>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {barcode.article.clone().unwrap_or_else(|| "—".to_string())}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                <TableCellLayout truncate=true>
                                                    {barcode.updated_at.clone()}
                                                </TableCellLayout>
                                            </TableCell>
                                            <TableCell>
                                                {if barcode.is_active {
                                                    view! {
                                                        <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Success>
                                                            "✓"
                                                        </Badge>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Danger>
                                                            "✗"
                                                        </Badge>
                                                    }.into_any()
                                                }}
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
