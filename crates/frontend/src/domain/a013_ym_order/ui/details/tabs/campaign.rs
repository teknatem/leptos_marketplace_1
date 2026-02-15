//! Campaign tab for YM Order

use super::super::view_model::YmOrderDetailsVm;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn CampaignTab(vm: YmOrderDetailsVm) -> impl IntoView {
    view! {
        {move || {
            let Some(order_data) = vm.order.get() else {
                return view! { <div>"Нет данных"</div> }.into_any();
            };

            let campaign_id = order_data.header.campaign_id.clone();
            let subsidies_display = order_data
                .header
                .subsidies_json
                .clone()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .map(|v| serde_json::to_string_pretty(&v).unwrap_or_else(|_| "—".to_string()))
                .unwrap_or_else(|| "—".to_string());
            let currency = order_data.header.currency.clone().unwrap_or_default();

            view! {
                <Card>
                    <h4 class="details-section__title">"Кампания и суммы"</h4>
                    <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                        <div class="form__group">
                            <label class="form__label">"Campaign ID"</label>
                            <Input value=RwSignal::new(campaign_id) attr:readonly=true />
                        </div>
                        <div class="form__group">
                            <label class="form__label">"Источник"</label>
                            <Input value=RwSignal::new("Yandex Market".to_string()) attr:readonly=true />
                        </div>
                    </div>
                    <div style="display: grid; grid-template-columns: 1fr 1fr 1fr; gap: var(--spacing-sm);">
                        <div class="form__group">
                            <label class="form__label">"Итог (API)"</label>
                            <Input
                                value=RwSignal::new(
                                    order_data
                                        .header
                                        .total_amount
                                        .map(|v| format!("{:.2} {}", v, currency))
                                        .unwrap_or_else(|| "—".to_string()),
                                )
                                attr:readonly=true
                            />
                        </div>
                        <div class="form__group">
                            <label class="form__label">"Сумма товаров"</label>
                            <Input
                                value=RwSignal::new(
                                    order_data
                                        .header
                                        .items_total
                                        .map(|v| format!("{:.2} {}", v, currency))
                                        .unwrap_or_else(|| "—".to_string()),
                                )
                                attr:readonly=true
                            />
                        </div>
                        <div class="form__group">
                            <label class="form__label">"Доставка"</label>
                            <Input
                                value=RwSignal::new(
                                    order_data
                                        .header
                                        .delivery_total
                                        .map(|v| format!("{:.2} {}", v, currency))
                                        .unwrap_or_else(|| "—".to_string()),
                                )
                                attr:readonly=true
                            />
                        </div>
                    </div>
                    <div class="form__group">
                        <label class="form__label">"Субсидии (JSON)"</label>
                        <Textarea value=RwSignal::new(subsidies_display) resize=TextareaResize::Vertical attr:readonly=true />
                    </div>
                    <div class="form__group">
                        <label class="form__label">"Fetched At"</label>
                        <Input value=RwSignal::new(order_data.source_meta.fetched_at) attr:readonly=true />
                    </div>
                </Card>
            }
            .into_any()
        }}
    }
}
