//! General tab - return document info and linked entities (a015 standard)

use super::super::view_model::YmReturnDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::components::ui::FieldDisplay;
use crate::shared::date_utils::format_datetime_utc_local;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn GeneralTab(vm: YmReturnDetailsVm) -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    view! {
        {move || {
            let Some(data) = vm.return_data.get() else {
                return view! { <div>"Нет данных"</div> }.into_any();
            };

            let conn_id = data.header.connection_id.clone();
            let org_id = data.header.organization_id.clone();
            let mp_id = data.header.marketplace_id.clone();

            let return_id = data.header.return_id.to_string();
            let order_id = data.header.order_id.to_string();
            let description = data.description.clone();
            let campaign_id = data.header.campaign_id.clone();

            let return_type = data.header.return_type.clone();
            let return_type_label = match return_type.as_str() {
                "UNREDEEMED" => "Невыкуп".to_string(),
                "RETURN" => "Возврат".to_string(),
                _ => return_type.clone(),
            };
            let return_type_badge = match return_type.as_str() {
                "UNREDEEMED" => "badge badge--warning",
                "RETURN" => "badge badge--info",
                _ => "badge badge--neutral",
            };

            let refund_status = data.state.refund_status.clone();
            let refund_status_badge = match refund_status.as_str() {
                "REFUNDED" => "badge badge--success",
                "NOT_REFUNDED" => "badge badge--error",
                "REFUND_IN_PROGRESS" => "badge badge--warning",
                _ => "badge badge--neutral",
            };

            let amount = data
                .header
                .amount
                .map(|a| {
                    format!(
                        "{:.2}{}",
                        a,
                        data.header
                            .currency
                            .clone()
                            .map(|c| format!(" {}", c))
                            .unwrap_or_default()
                    )
                })
                .unwrap_or_else(|| "—".to_string());

            let created_at_source = data
                .state
                .created_at_source
                .as_ref()
                .map(|d| format_datetime_utc_local(d, "%d.%m.%Y %H:%M:%S"))
                .unwrap_or_else(|| "—".to_string());
            let updated_at_source = data
                .state
                .updated_at_source
                .as_ref()
                .map(|d| format_datetime_utc_local(d, "%d.%m.%Y %H:%M:%S"))
                .unwrap_or_else(|| "—".to_string());
            let refund_date = data
                .state
                .refund_date
                .as_ref()
                .map(|d| format_datetime_utc_local(d, "%d.%m.%Y %H:%M:%S"))
                .unwrap_or_else(|| "—".to_string());
            let fetched_at =
                format_datetime_utc_local(&data.source_meta.fetched_at, "%d.%m.%Y %H:%M:%S");
            let version = data.source_meta.document_version.to_string();

            let total_items: i32 = data.lines.iter().map(|l| l.count).sum();
            let total_amount: f64 = data
                .lines
                .iter()
                .map(|l| l.price.unwrap_or(0.0) * l.count as f64)
                .sum();
            let total_items = total_items.to_string();
            let total_amount = format!("{:.2}", total_amount);

            view! {
                <div class="detail-grid">
                    <div class="detail-grid__col">
                        <CardAnimated delay_ms=0 nav_id="a016_ym_returns_details_general_document">
                            <h4 class="details-section__title">"Документ"</h4>
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"Return №"</label>
                                    <FieldDisplay value=return_id />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Order №"</label>
                                    {
                                        let order_id = order_id.clone();
                                        move || {
                                            let order_id = order_id.clone();
                                            match vm.source_order_id.get() {
                                                Some(oid) => view! {
                                                    <Button
                                                        appearance=ButtonAppearance::Subtle
                                                        size=ButtonSize::Small
                                                        on_click=move |_| tabs_store.open_tab(&format!("a013_ym_order_details_{}", oid), "YM Заказ")
                                                        attr:style="width: 100%; justify-content: flex-start;"
                                                    >
                                                        {order_id}
                                                    </Button>
                                                }.into_any(),
                                                None => view! { <FieldDisplay value=order_id /> }.into_any(),
                                            }
                                        }
                                    }
                                </div>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Тип"</label>
                                <div><span class=return_type_badge>{return_type_label}</span></div>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Описание"</label>
                                <FieldDisplay value=description />
                            </div>
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"Создан (источник)"</label>
                                    <FieldDisplay value=created_at_source />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Обновлён (источник)"</label>
                                    <FieldDisplay value=updated_at_source />
                                </div>
                            </div>
                        </CardAnimated>

                        <CardAnimated delay_ms=80 nav_id="a016_ym_returns_details_general_refund">
                            <h4 class="details-section__title">"Возврат"</h4>
                            <div class="form__group">
                                <label class="form__label">"Статус возврата денег"</label>
                                <div><span class=refund_status_badge>{refund_status}</span></div>
                            </div>
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"Сумма"</label>
                                    <FieldDisplay value=amount />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Дата возврата денег"</label>
                                    <FieldDisplay value=refund_date />
                                </div>
                            </div>
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"Товаров (шт.)"</label>
                                    <FieldDisplay value=total_items />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Сумма по строкам"</label>
                                    <FieldDisplay value=total_amount />
                                </div>
                            </div>
                        </CardAnimated>

                        <CardAnimated delay_ms=120 nav_id="a016_ym_returns_details_general_tech_links">
                            <h4 class="details-section__title">"Технические связи"</h4>
                            <div class="form__group">
                                <label class="form__label">"Подключение"</label>
                                <Button
                                    appearance=ButtonAppearance::Subtle
                                    size=ButtonSize::Small
                                    on_click={
                                        let conn_id = conn_id.clone();
                                        move |_| tabs_store.open_tab(&format!("a006_connection_mp_details_{}", conn_id), "Подключение МП")
                                    }
                                    attr:style="width: 100%; justify-content: flex-start;"
                                >
                                    {move || {
                                        vm.connection_info
                                            .get()
                                            .map(|i| i.description)
                                            .unwrap_or_else(|| "Загрузка...".to_string())
                                    }}
                                </Button>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Организация"</label>
                                <Button
                                    appearance=ButtonAppearance::Subtle
                                    size=ButtonSize::Small
                                    on_click={
                                        let org_id = org_id.clone();
                                        move |_| tabs_store.open_tab(&format!("a002_organization_details_{}", org_id), "Организация")
                                    }
                                    attr:style="width: 100%; justify-content: flex-start;"
                                >
                                    {move || {
                                        vm.organization_info
                                            .get()
                                            .map(|i| i.description)
                                            .unwrap_or_else(|| "Загрузка...".to_string())
                                    }}
                                </Button>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Маркетплейс"</label>
                                <Button
                                    appearance=ButtonAppearance::Subtle
                                    size=ButtonSize::Small
                                    on_click={
                                        let mp_id = mp_id.clone();
                                        move |_| tabs_store.open_tab(&format!("a005_marketplace_details_{}", mp_id), "Маркетплейс")
                                    }
                                    attr:style="width: 100%; justify-content: flex-start;"
                                >
                                    {move || {
                                        vm.marketplace_info
                                            .get()
                                            .map(|i| i.name)
                                            .unwrap_or_else(|| "Загрузка...".to_string())
                                    }}
                                </Button>
                            </div>
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"Campaign ID"</label>
                                    <FieldDisplay value=campaign_id />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Версия"</label>
                                    <FieldDisplay value=version />
                                </div>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Получено из API"</label>
                                <FieldDisplay value=fetched_at />
                            </div>
                        </CardAnimated>
                    </div>
                </div>
            }
            .into_any()
        }}
    }
}
