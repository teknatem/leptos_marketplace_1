//! General tab for YM Order

use super::super::view_model::YmOrderDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::date_utils::format_datetime;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn GeneralTab(vm: YmOrderDetailsVm) -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    view! {
        {move || {
            let Some(order_data) = vm.order.get() else {
                return view! { <div>"Нет данных"</div> }.into_any();
            };

            let conn_id = order_data.header.connection_id.clone();
            let org_id = order_data.header.organization_id.clone();
            let mp_id = order_data.header.marketplace_id.clone();
            let document_no = order_data.header.document_no.clone();
            let code = order_data.code.clone();
            let description = order_data.description.clone();

            let status_changed_at = order_data
                .state
                .status_changed_at
                .as_ref()
                .map(|d| format_datetime(d))
                .unwrap_or_else(|| "—".to_string());
            let updated_at_source = order_data
                .state
                .updated_at_source
                .as_ref()
                .map(|d| format_datetime(d))
                .unwrap_or_else(|| "—".to_string());
            let creation_date = order_data
                .state
                .creation_date
                .as_ref()
                .map(|d| format_datetime(d))
                .unwrap_or_else(|| "—".to_string());
            let delivery_date = order_data
                .state
                .delivery_date
                .as_ref()
                .map(|d| format_datetime(d))
                .unwrap_or_else(|| "—".to_string());

            let created_at = format_datetime(&order_data.metadata.created_at);
            let updated_at = format_datetime(&order_data.metadata.updated_at);
            let version = order_data.metadata.version.to_string();
            let is_error = order_data.is_error;

            view! {
                <div class="detail-grid">
                    // ── Левая колонка ────────────────────────────────────────
                    <div class="detail-grid__col">
                        <CardAnimated delay_ms=0>
                            <h4 class="details-section__title">"Документ"</h4>
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"№ документа"</label>
                                    <Input value=RwSignal::new(document_no) attr:readonly=true />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Code"</label>
                                    <Input value=RwSignal::new(code) attr:readonly=true />
                                </div>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Описание"</label>
                                <Input value=RwSignal::new(description) attr:readonly=true />
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Статус (норм.)"</label>
                                <Input value=RwSignal::new(order_data.state.status_norm.clone()) attr:readonly=true />
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Статус (сырой)"</label>
                                <Input value=RwSignal::new(order_data.state.status_raw.clone()) attr:readonly=true />
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Substatus"</label>
                                <Input value=RwSignal::new(order_data.state.substatus_raw.clone().unwrap_or_else(|| "—".to_string())) attr:readonly=true />
                            </div>
                            <Show when=move || is_error>
                                <Badge appearance=BadgeAppearance::Filled color=BadgeColor::Danger>
                                    "Есть строки без сопоставления номенклатуры"
                                </Badge>
                            </Show>
                        </CardAnimated>
                    </div>

                    // ── Правая колонка ───────────────────────────────────────
                    <div class="detail-grid__col">
                        <CardAnimated delay_ms=40>
                            <h4 class="details-section__title">"Даты"</h4>
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"Создан в источнике"</label>
                                    <Input value=RwSignal::new(creation_date) attr:readonly=true />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Изменение статуса"</label>
                                    <Input value=RwSignal::new(status_changed_at) attr:readonly=true />
                                </div>
                            </div>
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"Обновлено в источнике"</label>
                                    <Input value=RwSignal::new(updated_at_source) attr:readonly=true />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Доставка"</label>
                                    <Input value=RwSignal::new(delivery_date) attr:readonly=true />
                                </div>
                            </div>
                        </CardAnimated>

                        <CardAnimated delay_ms=120>
                            <h4 class="details-section__title">"Связи"</h4>
                            <div class="form__group">
                                <label class="form__label">"Подключение"</label>
                                <Button
                                    appearance=ButtonAppearance::Subtle
                                    size=ButtonSize::Small
                                    on_click={
                                        let conn_id = conn_id.clone();
                                        move |_| tabs_store.open_tab(&format!("a006_connection_mp_detail_{}", conn_id), "Подключение МП")
                                    }
                                    attr:style="width: 100%; justify-content: flex-start;"
                                >
                                    {conn_id}
                                </Button>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Организация"</label>
                                <Button
                                    appearance=ButtonAppearance::Subtle
                                    size=ButtonSize::Small
                                    on_click={
                                        let org_id = org_id.clone();
                                        move |_| tabs_store.open_tab(&format!("a002_organization_detail_{}", org_id), "Организация")
                                    }
                                    attr:style="width: 100%; justify-content: flex-start;"
                                >
                                    {org_id}
                                </Button>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Маркетплейс"</label>
                                <Button
                                    appearance=ButtonAppearance::Subtle
                                    size=ButtonSize::Small
                                    on_click={
                                        let mp_id = mp_id.clone();
                                        move |_| tabs_store.open_tab(&format!("a005_marketplace_detail_{}", mp_id), "Маркетплейс")
                                    }
                                    attr:style="width: 100%; justify-content: flex-start;"
                                >
                                    {mp_id}
                                </Button>
                            </div>
                            <div style="display: grid; grid-template-columns: 1fr 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"Created"</label>
                                    <Input value=RwSignal::new(created_at) attr:readonly=true />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Updated"</label>
                                    <Input value=RwSignal::new(updated_at) attr:readonly=true />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Version"</label>
                                    <Input value=RwSignal::new(version) attr:readonly=true />
                                </div>
                            </div>
                        </CardAnimated>
                    </div>
                </div>
            }
            .into_any()
        }}
    }
}
