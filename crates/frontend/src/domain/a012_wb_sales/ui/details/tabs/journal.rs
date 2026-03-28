//! Journal tab — записи журнала операций из sys_general_ledger

use super::super::view_model::WbSalesDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
use leptos::prelude::*;
use thaw::*;

fn fmt_amount(amount: f64) -> String {
    format!("{:.2}", amount)
}

fn short_id(id: &str) -> &str {
    if id.len() >= 8 {
        &id[..8]
    } else {
        id
    }
}

/// Journal tab — бухгалтерские проводки по документу
#[component]
pub fn JournalTab(vm: WbSalesDetailsVm) -> impl IntoView {
    view! {
        {move || {
            if vm.general_ledger_entries_loading.get() {
                return view! {
                    <CardAnimated delay_ms=0 nav_id="a012_wb_sales_details_journal_loading">
                        <Flex gap=FlexGap::Small style="align-items: center; justify-content: center; padding: var(--spacing-xl);">
                            <Spinner />
                            <span>"Загрузка журнала операций..."</span>
                        </Flex>
                    </CardAnimated>
                }.into_any();
            }

            if let Some(err) = vm.general_ledger_entries_error.get() {
                return view! {
                    <CardAnimated delay_ms=0 nav_id="a012_wb_sales_details_journal_error">
                        <h4 class="details-section__title">"Журнал операций"</h4>
                        <div style="color: var(--color-error);">
                            "Ошибка загрузки: " {err}
                        </div>
                    </CardAnimated>
                }.into_any();
            }

            let entries = vm.general_ledger_entries.get();

            if entries.is_empty() {
                return view! {
                    <CardAnimated delay_ms=0 nav_id="a012_wb_sales_details_journal_empty">
                        <h4 class="details-section__title">"Журнал операций"</h4>
                        <div style="color: var(--color-text-secondary);">
                            "Записи журнала не найдены. Проведите документ для формирования проводок."
                        </div>
                    </CardAnimated>
                }.into_any();
            }

            let total_amount: f64 = entries.iter().map(|e| e.amount).sum();
            let entries_count = entries.len();

            let posting_id = entries.first().map(|e| e.id.clone()).unwrap_or_default();

            view! {
                <CardAnimated delay_ms=0 nav_id="a012_wb_sales_details_journal_table">
                    <h4 class="details-section__title">"Журнал операций"</h4>

                    // Meta row
                    <Flex gap=FlexGap::Medium style="margin-bottom: var(--spacing-md); flex-wrap: wrap;">
                        <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                            {format!("Проводок: {}", entries_count)}
                        </Badge>
                        <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Informative>
                            {format!("Проведение: {}", short_id(&posting_id))}
                        </Badge>
                        <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Success>
                            {format!("Итого: {} ₽", fmt_amount(total_amount))}
                        </Badge>
                    </Flex>

                    // Table
                    <div style="overflow-x: auto;">
                        <table class="data-table" style="width: 100%; border-collapse: collapse; font-size: var(--font-size-sm);">
                            <thead>
                                <tr class="data-table__header-row">
                                    <th class="data-table__header-cell" style="text-align: left; padding: var(--spacing-sm) var(--spacing-md);">"Дата"</th>
                                    <th class="data-table__header-cell" style="text-align: center; padding: var(--spacing-sm) var(--spacing-md);">"Дт"</th>
                                    <th class="data-table__header-cell" style="text-align: center; padding: var(--spacing-sm) var(--spacing-md);">"Кт"</th>
                                    <th class="data-table__header-cell" style="text-align: right; padding: var(--spacing-sm) var(--spacing-md);">"Сумма, ₽"</th>
                                    <th class="data-table__header-cell" style="text-align: left; padding: var(--spacing-sm) var(--spacing-md);">"Вид оборота"</th>
                                    <th class="data-table__header-cell" style="text-align: left; padding: var(--spacing-sm) var(--spacing-md);">"ID"</th>
                                </tr>
                            </thead>
                            <tbody>
                                <For
                                    each=move || entries.clone()
                                    key=|entry| entry.id.clone()
                                    children=|entry| {
                                        view! {
                                            <tr class="data-table__row">
                                                <td class="data-table__cell" style="padding: var(--spacing-sm) var(--spacing-md); white-space: nowrap;">
                                                    {entry.entry_date.clone()}
                                                </td>
                                                <td class="data-table__cell" style="padding: var(--spacing-sm) var(--spacing-md); text-align: center;">
                                                    <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                                                        {entry.debit_account.clone()}
                                                    </Badge>
                                                </td>
                                                <td class="data-table__cell" style="padding: var(--spacing-sm) var(--spacing-md); text-align: center;">
                                                    <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Informative>
                                                        {entry.credit_account.clone()}
                                                    </Badge>
                                                </td>
                                                <td class="data-table__cell" style="padding: var(--spacing-sm) var(--spacing-md); text-align: right; font-variant-numeric: tabular-nums;">
                                                    {fmt_amount(entry.amount)}
                                                </td>
                                                <td class="data-table__cell" style="padding: var(--spacing-sm) var(--spacing-md); color: var(--color-text-secondary); white-space: nowrap;">
                                                    {entry.turnover_code.clone()}
                                                </td>
                                                <td class="data-table__cell" style="padding: var(--spacing-sm) var(--spacing-md); color: var(--color-text-secondary); font-size: var(--font-size-xs); font-family: monospace;">
                                                    {short_id(&entry.id).to_string()}
                                                </td>
                                            </tr>
                                        }
                                    }
                                />
                            </tbody>
                        </table>
                    </div>
                </CardAnimated>
            }.into_any()
        }}
    }
}
