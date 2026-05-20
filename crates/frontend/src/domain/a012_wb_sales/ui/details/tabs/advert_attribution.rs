//! Advert attribution tab — расшифровка GL-проводки `advert_clicks_order_expense`.
//!
//! Показывает все reserve-строки `p913_wb_advert_order_attr` с
//! `order_key=srid` (где `srid = document_no` этого a012). Сумма по этим
//! строкам = сумма проводки `advert_clicks_order_expense` в журнале (после проведения).
//! Каждая строка содержит ссылку на исходный документ a026, из которого
//! пришёл резерв рекламных расходов.

use super::super::view_model::WbSalesDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::card_animated::CardAnimated;
use leptos::prelude::*;
use thaw::*;

fn fmt_money(v: f64) -> String {
    format!("{:.2}", v)
}

fn fmt_ratio(v: f64) -> String {
    format!("{:.2}", v)
}

fn fmt_date(v: &str) -> String {
    if let Some((y, rest)) = v.split_once('-') {
        if let Some((m, d)) = rest.split_once('-') {
            return format!("{}.{}.{}", d, m, y);
        }
    }
    v.to_string()
}

#[component]
pub fn AdvertAttributionTab(vm: WbSalesDetailsVm) -> impl IntoView {
    let tabs = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    view! {
        {move || {
            if vm.advert_attribution_loading.get() {
                return view! {
                    <CardAnimated delay_ms=0 nav_id="a012_wb_sales_details_advert_attribution_loading">
                        <Flex gap=FlexGap::Small style="align-items: center; justify-content: center; padding: var(--spacing-xl);">
                            <Spinner />
                            <span>"Загрузка атрибуции..."</span>
                        </Flex>
                    </CardAnimated>
                }.into_any();
            }

            if let Some(err) = vm.advert_attribution_error.get() {
                return view! {
                    <CardAnimated delay_ms=0 nav_id="a012_wb_sales_details_advert_attribution_error">
                        <h4 class="details-section__title">"Атрибуция"</h4>
                        <div style="color: var(--color-error);">
                            "Ошибка загрузки: " {err}
                        </div>
                    </CardAnimated>
                }.into_any();
            }

            let Some(data) = vm.advert_attribution.get() else {
                return view! {
                    <CardAnimated delay_ms=0 nav_id="a012_wb_sales_details_advert_attribution_empty">
                        <h4 class="details-section__title">"Атрибуция"</h4>
                        <div style="color: var(--color-text-secondary);">
                            "Нет данных."
                        </div>
                    </CardAnimated>
                }.into_any();
            };

            let totals = data.totals.clone();
            let rows = data.rows.clone();
            let is_posted = data.is_posted;
            let is_customer_return = data.is_customer_return;
            let srid = data.srid.clone();
            let total_sum = totals.sum;
            let gl_amount = totals.gl_advert_expense;
            let is_match = totals.is_match;
            let rows_count = totals.rows_count;
            let campaigns_count = totals.campaigns_count;

            let tabs_table = tabs.clone();

            let summary_card = view! {
                <CardAnimated delay_ms=0 nav_id="a012_wb_sales_details_advert_attribution_summary">
                    <h4 class="details-section__title">"Сводка"</h4>

                    <Flex gap=FlexGap::Medium style="flex-wrap: wrap; align-items: center;">
                        <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Informative>
                            {format!("Записей: {}", rows_count)}
                        </Badge>
                        <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Informative>
                            {format!("Кампаний: {}", campaigns_count)}
                        </Badge>
                        <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                            {format!("Сумма атрибуции: {} ₽", fmt_money(total_sum))}
                        </Badge>
                        {match (is_posted, gl_amount) {
                            (true, Some(gl)) => view! {
                                <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Success>
                                    {format!("advert_clicks_order_expense в журнале: {} ₽", fmt_money(gl))}
                                </Badge>
                            }.into_any(),
                            (true, None) => view! {
                                <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Subtle>
                                    "advert_clicks_order_expense в журнале: —"
                                </Badge>
                            }.into_any(),
                            _ => view! { <span></span> }.into_any(),
                        }}
                        {match is_match {
                            Some(true) => view! {
                                <Badge appearance=BadgeAppearance::Filled color=BadgeColor::Success>
                                    "Совпадает"
                                </Badge>
                            }.into_any(),
                            Some(false) => {
                                let delta = total_sum - gl_amount.unwrap_or(0.0);
                                view! {
                                    <Badge appearance=BadgeAppearance::Filled color=BadgeColor::Danger>
                                        {format!("Расхождение: {} ₽", fmt_money(delta))}
                                    </Badge>
                                }.into_any()
                            }
                            None => view! { <span></span> }.into_any(),
                        }}
                    </Flex>

                    <div style="margin-top: var(--spacing-sm); color: var(--color-text-secondary); font-size: var(--font-size-sm);">
                        {format!("Связанный заказ (srid): {}", srid)}
                    </div>

                    {if is_customer_return {
                        view! {
                            <div style="margin-top: var(--spacing-md);">
                                <MessageBar intent=MessageBarIntent::Warning>
                                    <span>"Это возврат покупателя — расходы по рекламе не списываются в проводку advert_clicks_order_expense. Строки ниже показаны справочно."</span>
                                </MessageBar>
                            </div>
                        }.into_any()
                    } else if !is_posted && rows_count > 0 {
                        view! {
                            <div class="form__hint" style="margin-top: var(--spacing-sm);">
                                "Документ не проведён. При проведении эта сумма будет списана как advert_clicks_order_expense одной проводкой."
                            </div>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }}
                </CardAnimated>
            };

            let table_card = if rows.is_empty() {
                view! {
                    <CardAnimated delay_ms=80 nav_id="a012_wb_sales_details_advert_attribution_table">
                        <h4 class="details-section__title">"Строки атрибуции"</h4>
                        <div style="color: var(--color-text-secondary);">
                            "Нет записей атрибуции для srid этого документа. Возможно, по этому заказу не было рекламного резерва, либо документы a026 не проведены."
                        </div>
                    </CardAnimated>
                }.into_any()
            } else {
                view! {
                    <CardAnimated delay_ms=80 nav_id="a012_wb_sales_details_advert_attribution_table">
                        <h4 class="details-section__title">"Строки атрибуции"</h4>
                        <div style="overflow-x: auto;">
                            <table class="data-table" style="width: 100%; border-collapse: collapse; font-size: var(--font-size-sm);">
                                <thead>
                                    <tr class="data-table__header-row">
                                        <th class="data-table__header-cell" style="text-align: left; padding: var(--spacing-sm) var(--spacing-md);">"Дата"</th>
                                        <th class="data-table__header-cell" style="text-align: left; padding: var(--spacing-sm) var(--spacing-md);">"Кампания"</th>
                                        <th class="data-table__header-cell" style="text-align: left; padding: var(--spacing-sm) var(--spacing-md);">"Артикул"</th>
                                        <th class="data-table__header-cell" style="text-align: right; padding: var(--spacing-sm) var(--spacing-md);">"Сумма, ₽"</th>
                                        <th class="data-table__header-cell" style="text-align: right; padding: var(--spacing-sm) var(--spacing-md);">"Доля, %"</th>
                                        <th class="data-table__header-cell" style="text-align: left; padding: var(--spacing-sm) var(--spacing-md);">"Источник (a026)"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    <For
                                        each=move || rows.clone()
                                        key=|row| row.id.clone()
                                        children=move |row| {
                                            let entry_date = fmt_date(&row.entry_date);
                                            let advert_id = row.advert_id.clone();
                                            let amount = fmt_money(row.amount);
                                            let ratio = fmt_ratio(row.ratio_percent);

                                            let article_text = row
                                                .nomenclature_article
                                                .clone()
                                                .unwrap_or_else(|| "—".to_string());
                                            let nom_ref = row.nomenclature_ref.clone();

                                            let a026_id = row.a026_id.clone();
                                            let a026_doc_no = row.a026_document_no.clone();
                                            let a026_doc_date = row.a026_document_date.clone();
                                            let a026_label = match (&a026_doc_no, &a026_doc_date) {
                                                (Some(no), Some(date)) => format!("{} от {}", no, fmt_date(date)),
                                                (Some(no), None) => no.clone(),
                                                _ => a026_id.clone().unwrap_or_else(|| "—".to_string()),
                                            };

                                            let tabs_row_nom = tabs_table.clone();
                                            let tabs_row_a026 = tabs_table.clone();

                                            view! {
                                                <tr class="data-table__row">
                                                    <td class="data-table__cell" style="padding: var(--spacing-sm) var(--spacing-md); white-space: nowrap;">
                                                        {entry_date}
                                                    </td>
                                                    <td class="data-table__cell" style="padding: var(--spacing-sm) var(--spacing-md); white-space: nowrap;">
                                                        <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                                                            {advert_id}
                                                        </Badge>
                                                        {if row.is_problem {
                                                            view! {
                                                                <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Danger attr:style="margin-left: 6px;">
                                                                    "проблема"
                                                                </Badge>
                                                            }.into_any()
                                                        } else {
                                                            view! { <span></span> }.into_any()
                                                        }}
                                                    </td>
                                                    <td class="data-table__cell" style="padding: var(--spacing-sm) var(--spacing-md);">
                                                        {match nom_ref {
                                                            Some(nref) if !nref.is_empty() => {
                                                                let key = format!("a004_nomenclature_details_{}", nref);
                                                                let label = article_text.clone();
                                                                view! {
                                                                    <a href="#" class="table__link" on:click=move |e| {
                                                                        e.prevent_default();
                                                                        tabs_row_nom.open_tab(&key, "Номенклатура");
                                                                    }>{label}</a>
                                                                }.into_any()
                                                            }
                                                            _ => view! { <span>{article_text}</span> }.into_any(),
                                                        }}
                                                    </td>
                                                    <td class="data-table__cell" style="padding: var(--spacing-sm) var(--spacing-md); text-align: right; font-variant-numeric: tabular-nums;">
                                                        {amount}
                                                    </td>
                                                    <td class="data-table__cell" style="padding: var(--spacing-sm) var(--spacing-md); text-align: right; font-variant-numeric: tabular-nums; color: var(--color-text-secondary);">
                                                        {ratio}
                                                    </td>
                                                    <td class="data-table__cell" style="padding: var(--spacing-sm) var(--spacing-md);">
                                                        {match a026_id {
                                                            Some(aid) if !aid.is_empty() => {
                                                                let key = format!("a026_wb_advert_daily_details_{}", aid);
                                                                let title = "WB Ads".to_string();
                                                                let label = a026_label.clone();
                                                                view! {
                                                                    <a href="#" class="table__link" on:click=move |e| {
                                                                        e.prevent_default();
                                                                        tabs_row_a026.open_tab(&key, &title);
                                                                    }>{label}</a>
                                                                }.into_any()
                                                            }
                                                            _ => view! { <span class="text-muted">{a026_label}</span> }.into_any(),
                                                        }}
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
            };

            view! {
                <div style="display: flex; flex-direction: column; gap: var(--spacing-md); width: 100%;">
                    {summary_card}
                    {table_card}
                </div>
            }.into_any()
        }}
    }
}
