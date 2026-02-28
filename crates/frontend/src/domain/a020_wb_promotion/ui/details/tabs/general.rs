use crate::domain::a020_wb_promotion::ui::details::view_model::WbPromotionDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
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
        {move || {
            let Some(promo) = vm.promotion.get() else {
                return view! { <div>"Нет данных"</div> }.into_any();
            };

            let advantages = promo.data.advantages.clone();
            let ranging = promo.data.ranging.clone();
            let desc = promo.data.description.clone().unwrap_or_default();

            let promo_id = promo.data.promotion_id.to_string();
            let name = promo.data.name.clone();
            let promo_type = promo.data.promotion_type.clone().unwrap_or_else(|| "—".to_string());
            let start_dt = format_dt(&promo.data.start_date_time);
            let end_dt = format_dt(&promo.data.end_date_time);
            let nom_count = promo.nomenclatures.len().to_string();
            let exceptions = opt_i32(promo.data.exception_products_count);

            let in_total = opt_i32(promo.data.in_promo_action_total);
            let in_leftovers = opt_i32(promo.data.in_promo_action_leftovers);
            let not_in_total = opt_i32(promo.data.not_in_promo_action_total);
            let not_in_leftovers = opt_i32(promo.data.not_in_promo_action_leftovers);
            let participation = promo
                .data
                .participation_percentage
                .map(|v| format!("{:.1}%", v))
                .unwrap_or_else(|| "—".to_string());

            let document_no = promo.header.document_no.clone();
            let connection_id = promo.header.connection_id.clone();
            let organization_id = promo.header.organization_id.clone();
            let fetched_at = promo.source_meta.fetched_at.clone();
            let version = promo.metadata.version.to_string();

            view! {
                <div class="detail-grid">

                    // Левая колонка
                    <div class="detail-grid__col">

                        // Данные акции
                        <CardAnimated delay_ms=0>
                            <h4 class="details-section__title">"Данные акции"</h4>
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"ID акции WB"</label>
                                    <Input value=RwSignal::new(promo_id) attr:readonly=true />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Тип акции"</label>
                                    <Input value=RwSignal::new(promo_type) attr:readonly=true />
                                </div>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Название"</label>
                                <Input value=RwSignal::new(name) attr:readonly=true />
                            </div>
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"Начало акции"</label>
                                    <Input value=RwSignal::new(start_dt) attr:readonly=true />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Окончание акции"</label>
                                    <Input value=RwSignal::new(end_dt) attr:readonly=true />
                                </div>
                            </div>
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"Номенклатур загружено"</label>
                                    <Input value=RwSignal::new(nom_count) attr:readonly=true />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Исключений"</label>
                                    <Input value=RwSignal::new(exceptions) attr:readonly=true />
                                </div>
                            </div>
                        </CardAnimated>

                        // Описание (если есть)
                        {
                            if !desc.is_empty() {
                                view! {
                                    <CardAnimated delay_ms=80>
                                        <h4 class="details-section__title">"Описание"</h4>
                                        <p style="font-size: 13px; color: var(--colorNeutralForeground1); line-height: 1.5; margin: 0;">
                                            {desc}
                                        </p>
                                    </CardAnimated>
                                }.into_any()
                            } else {
                                view! { <div></div> }.into_any()
                            }
                        }

                        // Преимущества участия (если есть)
                        {
                            if !advantages.is_empty() {
                                view! {
                                    <CardAnimated delay_ms=160>
                                        <h4 class="details-section__title">"Преимущества участия"</h4>
                                        <div style="display: flex; flex-wrap: wrap; gap: 8px;">
                                            {advantages.into_iter().map(|adv| view! {
                                                <span style="background: var(--colorBrandBackground2); color: var(--colorBrandForeground1); padding: 4px 10px; border-radius: 12px; font-size: 12px; font-weight: 500;">
                                                    {adv}
                                                </span>
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    </CardAnimated>
                                }.into_any()
                            } else {
                                view! { <div></div> }.into_any()
                            }
                        }

                    </div>

                    // Правая колонка
                    <div class="detail-grid__col">

                        // Статистика участия
                        <CardAnimated delay_ms=40>
                            <h4 class="details-section__title">"Статистика участия"</h4>
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"В акции (всего)"</label>
                                    <Input value=RwSignal::new(in_total) attr:readonly=true />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"В акции (остатки)"</label>
                                    <Input value=RwSignal::new(in_leftovers) attr:readonly=true />
                                </div>
                            </div>
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"Не в акции (всего)"</label>
                                    <Input value=RwSignal::new(not_in_total) attr:readonly=true />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Не в акции (остатки)"</label>
                                    <Input value=RwSignal::new(not_in_leftovers) attr:readonly=true />
                                </div>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"% участия"</label>
                                <Input value=RwSignal::new(participation) attr:readonly=true />
                            </div>
                        </CardAnimated>

                        // Условия рейтингового буста (если есть)
                        {
                            if !ranging.is_empty() {
                                view! {
                                    <CardAnimated delay_ms=120>
                                        <h4 class="details-section__title">"Условия рейтингового буста"</h4>
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
                                    </CardAnimated>
                                }.into_any()
                            } else {
                                view! { <div></div> }.into_any()
                            }
                        }

                        // Подключение
                        <CardAnimated delay_ms=200>
                            <h4 class="details-section__title">"Подключение"</h4>
                            <div class="form__group">
                                <label class="form__label">"Номер документа"</label>
                                <Input value=RwSignal::new(document_no) attr:readonly=true />
                            </div>
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"ID подключения"</label>
                                    <Input value=RwSignal::new(connection_id) attr:readonly=true />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"ID организации"</label>
                                    <Input value=RwSignal::new(organization_id) attr:readonly=true />
                                </div>
                            </div>
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"Загружено"</label>
                                    <Input value=RwSignal::new(fetched_at) attr:readonly=true />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Версия"</label>
                                    <Input value=RwSignal::new(version) attr:readonly=true />
                                </div>
                            </div>
                        </CardAnimated>

                    </div>

                </div>
            }.into_any()
        }}
    }
}
