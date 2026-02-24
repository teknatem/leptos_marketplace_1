use crate::domain::a020_wb_promotion::ui::details::view_model::WbPromotionDetailsVm;
use leptos::prelude::*;
use thaw::*;

fn format_dt(dt: &str) -> String {
    if let Some(date_part) = dt.split('T').next() {
        if let Some((year, rest)) = date_part.split_once('-') {
            if let Some((month, day)) = rest.split_once('-') {
                let time_part = dt.split('T').nth(1).unwrap_or("");
                let time_clean = time_part.split('Z').next().unwrap_or("").split('+').next().unwrap_or("");
                if !time_clean.is_empty() {
                    return format!("{}.{}.{} {}", day, month, year, &time_clean[..time_clean.len().min(5)]);
                }
                return format!("{}.{}.{}", day, month, year);
            }
        }
    }
    dt.to_string()
}

fn opt_i32(v: Option<i32>) -> String {
    v.map(|n| n.to_string()).unwrap_or_else(|| "—".to_string())
}

fn opt_f64(v: Option<f64>) -> String {
    v.map(|n| format!("{:.1}", n)).unwrap_or_else(|| "—".to_string())
}

#[component]
pub fn GeneralTab(vm: WbPromotionDetailsVm) -> impl IntoView {
    view! {
        <div style="padding: var(--spacing-lg);">
            {move || {
                let Some(promo) = vm.promotion.get() else {
                    return view! { <div>"Нет данных"</div> }.into_any();
                };

                let advantages = promo.data.advantages.clone();
                let ranging = promo.data.ranging.clone();

                view! {
                    <div style="display: flex; flex-direction: column; gap: var(--spacing-xl);">

                        // Первая строка: Данные акции + Статистика участия
                        <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-xl);">
                            <div>
                                <h3 style="margin-bottom: var(--spacing-md); font-size: 14px; font-weight: 600; color: var(--colorNeutralForeground2);">"Данные акции"</h3>
                                <table style="width: 100%; border-collapse: collapse;">
                                    <tbody>
                                        <FieldRow label="ID акции WB" value=promo.data.promotion_id.to_string() />
                                        <FieldRow label="Название" value=promo.data.name.clone() />
                                        <FieldRow
                                            label="Тип акции"
                                            value=promo.data.promotion_type.clone().unwrap_or_else(|| "—".to_string())
                                        />
                                        <FieldRow label="Начало акции" value=format_dt(&promo.data.start_date_time) />
                                        <FieldRow label="Окончание акции" value=format_dt(&promo.data.end_date_time) />
                                        <FieldRow
                                            label="Загружено номенклатур"
                                            value=promo.nomenclatures.len().to_string()
                                        />
                                        <FieldRow label="Исключений" value=opt_i32(promo.data.exception_products_count) />
                                    </tbody>
                                </table>
                            </div>

                            <div>
                                <h3 style="margin-bottom: var(--spacing-md); font-size: 14px; font-weight: 600; color: var(--colorNeutralForeground2);">"Статистика участия"</h3>
                                <table style="width: 100%; border-collapse: collapse;">
                                    <tbody>
                                        <FieldRow
                                            label="В акции (всего)"
                                            value=opt_i32(promo.data.in_promo_action_total)
                                        />
                                        <FieldRow
                                            label="В акции (остатки)"
                                            value=opt_i32(promo.data.in_promo_action_leftovers)
                                        />
                                        <FieldRow
                                            label="Не в акции (всего)"
                                            value=opt_i32(promo.data.not_in_promo_action_total)
                                        />
                                        <FieldRow
                                            label="Не в акции (остатки)"
                                            value=opt_i32(promo.data.not_in_promo_action_leftovers)
                                        />
                                        <FieldRow
                                            label="% участия"
                                            value={
                                                promo.data.participation_percentage
                                                    .map(|v| format!("{:.1}%", v))
                                                    .unwrap_or_else(|| "—".to_string())
                                            }
                                        />
                                    </tbody>
                                </table>
                            </div>
                        </div>

                        // Описание
                        {
                            let desc = promo.data.description.clone().unwrap_or_default();
                            if !desc.is_empty() {
                                view! {
                                    <div>
                                        <h3 style="margin-bottom: var(--spacing-md); font-size: 14px; font-weight: 600; color: var(--colorNeutralForeground2);">"Описание"</h3>
                                        <p style="font-size: 13px; color: var(--colorNeutralForeground1); line-height: 1.5; background: var(--colorNeutralBackground2); padding: 10px 12px; border-radius: 4px;">
                                            {desc}
                                        </p>
                                    </div>
                                }.into_any()
                            } else {
                                view! { <div></div> }.into_any()
                            }
                        }

                        // Преимущества
                        {
                            if !advantages.is_empty() {
                                view! {
                                    <div>
                                        <h3 style="margin-bottom: var(--spacing-md); font-size: 14px; font-weight: 600; color: var(--colorNeutralForeground2);">"Преимущества участия"</h3>
                                        <div style="display: flex; flex-wrap: wrap; gap: 8px;">
                                            {advantages.into_iter().map(|adv| view! {
                                                <span style="background: var(--colorBrandBackground2); color: var(--colorBrandForeground1); padding: 4px 10px; border-radius: 12px; font-size: 12px; font-weight: 500;">
                                                    {adv}
                                                </span>
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    </div>
                                }.into_any()
                            } else {
                                view! { <div></div> }.into_any()
                            }
                        }

                        // Условия рейтингового буста
                        {
                            if !ranging.is_empty() {
                                view! {
                                    <div>
                                        <h3 style="margin-bottom: var(--spacing-md); font-size: 14px; font-weight: 600; color: var(--colorNeutralForeground2);">"Условия рейтингового буста"</h3>
                                        <table style="width: 100%; border-collapse: collapse;">
                                            <thead>
                                                <tr style="background: var(--colorNeutralBackground2);">
                                                    <th style="padding: 6px 10px; font-size: 12px; text-align: left; color: var(--colorNeutralForeground2);">"Условие"</th>
                                                    <th style="padding: 6px 10px; font-size: 12px; text-align: right; color: var(--colorNeutralForeground2);">"% участия"</th>
                                                    <th style="padding: 6px 10px; font-size: 12px; text-align: right; color: var(--colorNeutralForeground2);">"Буст"</th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                {ranging.into_iter().map(|r| {
                                                    let condition = r.condition.clone().unwrap_or_else(|| "—".to_string());
                                                    let rate = opt_f64(r.participation_rate);
                                                    let boost = opt_f64(r.boost);
                                                    view! {
                                                        <tr style="border-bottom: 1px solid var(--colorNeutralStroke2);">
                                                            <td style="padding: 6px 10px; font-size: 12px;">{condition}</td>
                                                            <td style="padding: 6px 10px; font-size: 12px; text-align: right;">{rate}</td>
                                                            <td style="padding: 6px 10px; font-size: 12px; text-align: right;">{boost}</td>
                                                        </tr>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </tbody>
                                        </table>
                                    </div>
                                }.into_any()
                            } else {
                                view! { <div></div> }.into_any()
                            }
                        }

                        // Подключение / метаданные
                        <div>
                            <h3 style="margin-bottom: var(--spacing-md); font-size: 14px; font-weight: 600; color: var(--colorNeutralForeground2);">"Подключение"</h3>
                            <table style="width: 100%; border-collapse: collapse;">
                                <tbody>
                                    <FieldRow label="Номер документа" value=promo.header.document_no.clone() />
                                    <FieldRow label="ID подключения" value=promo.header.connection_id.clone() />
                                    <FieldRow label="ID организации" value=promo.header.organization_id.clone() />
                                    <FieldRow label="Загружено" value=promo.source_meta.fetched_at.clone() />
                                    <FieldRow label="Версия" value=promo.metadata.version.to_string() />
                                </tbody>
                            </table>
                        </div>

                    </div>
                }.into_any()
            }}
        </div>
    }
}

#[component]
fn FieldRow(label: &'static str, value: String) -> impl IntoView {
    view! {
        <tr style="border-bottom: 1px solid var(--colorNeutralStroke2);">
            <td style="padding: 6px 8px; font-size: 12px; color: var(--colorNeutralForeground2); white-space: nowrap; width: 200px;">
                {label}
            </td>
            <td style="padding: 6px 8px; font-size: 12px; color: var(--colorNeutralForeground1);">
                {value}
            </td>
        </tr>
    }
}
