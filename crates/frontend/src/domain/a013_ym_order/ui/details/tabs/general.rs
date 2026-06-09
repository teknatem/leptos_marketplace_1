//! General tab for YM Order

use super::super::view_model::YmOrderDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::components::table::format_money;
use crate::shared::date_utils::format_datetime;
use contracts::projections::p915_mp_order_events::event::OrderEventType;
use leptos::prelude::*;
use thaw::*;

/// Человекочитаемое имя документа-регистратора события (для гиперссылки).
fn registrator_label(registrator_type: &str) -> &'static str {
    match registrator_type {
        "a013_ym_order" => "Заказ YM",
        "a034_ym_realization" => "Реализация YM",
        "p907_ym_payment_report" => "Платёж YM (p907)",
        "a035_ym_settlement_recon" => "Сверка перечислений YM",
        _ => "Регистратор",
    }
}

/// Одной строкой — что именно происходит в этот момент жизненного цикла заказа.
fn event_description(event_type: &str) -> &'static str {
    match event_type {
        "order_placed" => "Покупатель оформил заказ",
        "shipment" => "Заказ отгружен со склада",
        "delivery" => "Заказ доставлен покупателю",
        "realization" => "Признание выручки по отчёту о реализации",
        "goods_return" => "Покупатель вернул товар",
        "payment" => "Покупатель оплатил заказ маркетплейсу",
        "payment_return" => "Маркетплейс вернул оплату покупателю",
        "supplier_payment" => "Маркетплейс перечислил оплату поставщику",
        "supplier_payment_return" => "Маркетплейс удержал оплату при возврате товара",
        _ => "",
    }
}

#[component]
pub fn GeneralTab(vm: YmOrderDetailsVm) -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    // Copy-сигналы (вытаскиваем до view!, чтобы не двигать vm в несколько замыканий).
    let order_sig = vm.order;
    let connection_name = vm.connection_name;
    let organization_name = vm.organization_name;
    let marketplace_name = vm.marketplace_name;
    let order_events = vm.order_events;

    view! {
        {move || {
            let Some(order_data) = order_sig.get() else {
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
                        <CardAnimated delay_ms=0 nav_id="a013_ym_order_details_general_document">
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

                        // ── Таймлайн событий заказа (p915) ───────────────────
                        {move || {
                            let events = order_events.get();
                            if events.is_empty() {
                                return ().into_any();
                            }
                            let tabs_store = tabs_store;
                            view! {
                                <CardAnimated delay_ms=160 nav_id="a013_ym_order_details_general_events">
                                    <h4 class="details-section__title">"События заказа"</h4>
                                    <table style="width: 100%; border-collapse: collapse; font-size: 0.88em;">
                                        <thead>
                                            <tr style="background: var(--color-bg-elevated); border-bottom: 1px solid var(--color-border-subtle, var(--color-border));">
                                                <th style="color: var(--form-label-text); padding-bottom: 4px;">"Дата"</th>
                                                <th style="color: var(--form-label-text); padding-bottom: 4px;">"Событие"</th>
                                                <th style="color: var(--form-label-text); padding-bottom: 4px;">"Сумма"</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {events.into_iter().map(|e| {
                                                let tabs_store = tabs_store;
                                                let event_label = OrderEventType::from_str(&e.event_type)
                                                    .map(|t| t.label_ru().to_string())
                                                    .unwrap_or_else(|| e.event_type.clone());
                                                let description = event_description(&e.event_type);
                                                let reg_label = registrator_label(&e.registrator_type);
                                                let amount = e.amount.map(format_money).unwrap_or_default();
                                                let tab_key = format!("{}_details_{}", e.registrator_type, e.registrator_ref);
                                                let tab_title = reg_label.to_string();
                                                let has_ref = !e.registrator_ref.trim().is_empty();
                                                // Дата доставки и оплаты поставщику — зелёным,
                                                // возврат оплаты поставщику — красным.
                                                let td_date = match e.event_type.as_str() {
                                                    "delivery" | "supplier_payment" => "padding: 5px 8px; border-bottom: 1px solid var(--color-border-subtle, var(--color-border)); vertical-align: center; color: var(--color-success); font-weight: 600;",
                                                    "supplier_payment_return" => "padding: 5px 8px; border-bottom: 1px solid var(--color-border-subtle, var(--color-border)); vertical-align: center; color: var(--color-danger); font-weight: 600;",
                                                    _ => "padding: 5px 8px; border-bottom: 1px solid var(--color-border-subtle, var(--color-border)); vertical-align: center;",
                                                };
                                                let td = "padding: 5px 8px; border-bottom: 1px solid var(--color-border-subtle, var(--color-border)); vertical-align: top;";
                                                let td_r = "padding: 5px 8px; border-bottom: 1px solid var(--color-border-subtle, var(--color-border)); text-align: right; font-variant-numeric: tabular-nums; vertical-align: top;";
                                                view! {
                                                    <tr>
                                                        <td style=td_date>{e.event_date.clone()}</td>
                                                        <td style=td>
                                                            <div style="font-size: 1em; color: var(--color-text-primary);">{event_label}</div>
                                                            <div style="font-size: 0.9em; color: var(--color-text-secondary); margin-top: 1px;">
                                                                {description}
                                                                {if has_ref {
                                                                    view! {
                                                                        " — "
                                                                        <a href="#" class="table__link"
                                                                            on:click=move |ev| { ev.prevent_default(); tabs_store.open_tab(&tab_key, &tab_title); }
                                                                        >{reg_label}</a>
                                                                    }.into_any()
                                                                } else {
                                                                    view! { <span /> }.into_any()
                                                                }}
                                                            </div>
                                                        </td>
                                                        <td style=td_r>{amount}</td>
                                                    </tr>
                                                }
                                            }).collect_view()}
                                        </tbody>
                                    </table>
                                </CardAnimated>
                            }
                            .into_any()
                        }}
                    </div>

                    // ── Правая колонка ───────────────────────────────────────
                    <div class="detail-grid__col">
                        <CardAnimated delay_ms=40 nav_id="a013_ym_order_details_general_dates">
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

                        <CardAnimated delay_ms=120 nav_id="a013_ym_order_details_general_links">
                            <h4 class="details-section__title">"Связи"</h4>
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
                                    {let conn_id = conn_id.clone(); move || connection_name.get().unwrap_or_else(|| conn_id.clone())}
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
                                    {let org_id = org_id.clone(); move || organization_name.get().unwrap_or_else(|| org_id.clone())}
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
                                    {let mp_id = mp_id.clone(); move || marketplace_name.get().unwrap_or_else(|| mp_id.clone())}
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
