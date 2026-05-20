use crate::domain::a032_wb_returns_claims::ui::details::view_model::WbReturnsClaimsDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::components::ui::{FieldDisplay, FieldDisplayMultiline};
use crate::shared::date_utils::format_datetime_utc_local;
use leptos::prelude::*;
use thaw::*;

fn status_label(status: Option<i32>) -> &'static str {
    match status {
        Some(1) => "Открыта",
        Some(2) => "На рассмотрении",
        Some(3) => "Одобрена",
        Some(4) => "Отклонена",
        Some(5) => "Закрыта",
        _ => "Неизвестен",
    }
}

fn fmt_dt(iso: &str) -> String {
    format_datetime_utc_local(iso, "%d.%m.%Y %H:%M:%S")
}

fn fmt_opt_dt(iso: Option<&str>) -> String {
    iso.map(fmt_dt).unwrap_or_else(|| "—".to_string())
}

fn fmt_opt_string(s: Option<&str>) -> String {
    s.map(|v| v.to_string()).unwrap_or_else(|| "—".to_string())
}

#[component]
pub fn GeneralTab(vm: WbReturnsClaimsDetailsVm) -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    let item_sig = vm.item;
    let conn_info_sig = vm.connection_info;
    let org_info_sig = vm.organization_info;
    let mp_info_sig = vm.marketplace_info;
    let vm_stored = StoredValue::new(vm);

    view! {
        {move || {
            let Some(d) = item_sig.get() else {
                return view! { <div>"Нет данных"</div> }.into_any();
            };

            let claim_id = d.claim_id.clone();
            let claim_type = d
                .claim_type
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".to_string());
            let status_text = format!(
                "{} ({})",
                status_label(d.status),
                d.status.map(|v| v.to_string()).unwrap_or_default()
            );
            let status_ex = d
                .status_ex
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".to_string());
            let is_archive_str = if d.is_archive { "Да" } else { "Нет" }.to_string();
            let dt_str = fmt_dt(&d.dt);
            let dt_update_str = fmt_opt_dt(d.dt_update.as_deref());
            let delivery_dt_str = fmt_opt_dt(d.delivery_dt.as_deref());

            let nm_id_str = d.nm_id.to_string();
            let imt_name = fmt_opt_string(d.imt_name.as_deref());
            let price_str = d
                .price
                .map(|p| {
                    format!(
                        "{:.2} {}",
                        p,
                        d.currency_code.clone().unwrap_or_default()
                    )
                })
                .unwrap_or_else(|| "—".to_string());
            let order_dt_str = fmt_opt_dt(d.order_dt.as_deref());
            let origin_id_info = fmt_opt_string(d.origin_id_info.as_deref());
            let srid = d.srid.clone();

            let user_comment = fmt_opt_string(d.user_comment.as_deref());
            let wb_comment = fmt_opt_string(d.wb_comment.as_deref());
            let actions = fmt_opt_string(d.actions.as_deref());

            let conn_id = d.connection_id.clone();
            let org_id = d.organization_id.clone();
            let mp_id = d.marketplace_id.clone();
            let created_at = fmt_dt(&d.metadata.created_at);
            let updated_at = fmt_dt(&d.metadata.updated_at);
            let version = d.metadata.version.to_string();

            view! {
                <div class="detail-grid">
                    <div class="detail-grid__col">
                        <CardAnimated delay_ms=0 nav_id="a032_wb_returns_claims_details_claim">
                            <h4 class="details-section__title">"Заявка"</h4>
                            <div class="form__group">
                                <label class="form__label">"ID заявки WB"</label>
                                <FieldDisplay value=claim_id />
                            </div>
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"Статус"</label>
                                    <FieldDisplay value=status_text />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Статус (расш.)"</label>
                                    <FieldDisplay value=status_ex />
                                </div>
                            </div>
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"Тип заявки"</label>
                                    <FieldDisplay value=claim_type />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Архив"</label>
                                    <FieldDisplay value=is_archive_str />
                                </div>
                            </div>
                            <div style="display: grid; grid-template-columns: 1fr 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"Дата создания"</label>
                                    <FieldDisplay value=dt_str />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Дата обновления"</label>
                                    <FieldDisplay value=dt_update_str />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Дата доставки"</label>
                                    <FieldDisplay value=delivery_dt_str />
                                </div>
                            </div>
                        </CardAnimated>

                        <CardAnimated delay_ms=80 nav_id="a032_wb_returns_claims_details_comments">
                            <h4 class="details-section__title">"Комментарии"</h4>
                            <div class="form__group">
                                <label class="form__label">"Комментарий покупателя"</label>
                                <FieldDisplayMultiline value=user_comment />
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Комментарий WB"</label>
                                <FieldDisplayMultiline value=wb_comment />
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Доступные действия"</label>
                                <FieldDisplay value=actions />
                            </div>
                        </CardAnimated>
                    </div>

                    <div class="detail-grid__col">
                        <CardAnimated delay_ms=40 nav_id="a032_wb_returns_claims_details_product">
                            <h4 class="details-section__title">"Товар"</h4>
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"nmId (WB)"</label>
                                    <FieldDisplay value=nm_id_str />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Цена"</label>
                                    <FieldDisplay value=price_str />
                                </div>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Наименование товара"</label>
                                <FieldDisplay value=imt_name />
                            </div>
                            <div class="form__group">
                                <label class="form__label">"srid заказа"</label>
                                {match srid {
                                    Some(s) if !s.is_empty() => {
                                        let s_for_click = s.clone();
                                        view! {
                                            <a
                                                href="#"
                                                class="table__link form__value--mono"
                                                title="Открыть заказ WB"
                                                on:click=move |e: web_sys::MouseEvent| {
                                                    e.prevent_default();
                                                    let srid = s_for_click.clone();
                                                    vm_stored.with_value(|v| {
                                                        v.open_order_by_srid(srid, tabs_store)
                                                    });
                                                }
                                                style="display: inline-block; padding: var(--spacing-xs) 0;"
                                            >
                                                {s}
                                            </a>
                                        }
                                        .into_any()
                                    }
                                    _ => view! { <FieldDisplay value="—".to_string() /> }.into_any(),
                                }}
                            </div>
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"Дата заказа"</label>
                                    <FieldDisplay value=order_dt_str />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"IMEI / ID"</label>
                                    <FieldDisplay value=origin_id_info />
                                </div>
                            </div>
                        </CardAnimated>

                        <CardAnimated delay_ms=120 nav_id="a032_wb_returns_claims_details_tech_links">
                            <h4 class="details-section__title">"Технические связи"</h4>
                            <div class="form__group">
                                <label class="form__label">"Подключение"</label>
                                <Button
                                    appearance=ButtonAppearance::Subtle
                                    size=ButtonSize::Small
                                    on_click={
                                        let conn_id = conn_id.clone();
                                        move |_| tabs_store.open_tab(
                                            &format!("a006_connection_mp_details_{}", conn_id),
                                            "Подключение МП",
                                        )
                                    }
                                    attr:style="width: 100%; justify-content: flex-start;"
                                >
                                    {move || {
                                        conn_info_sig
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
                                        move |_| tabs_store.open_tab(
                                            &format!("a002_organization_details_{}", org_id),
                                            "Организация",
                                        )
                                    }
                                    attr:style="width: 100%; justify-content: flex-start;"
                                >
                                    {move || {
                                        org_info_sig
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
                                        move |_| tabs_store.open_tab(
                                            &format!("a005_marketplace_details_{}", mp_id),
                                            "Маркетплейс",
                                        )
                                    }
                                    attr:style="width: 100%; justify-content: flex-start;"
                                >
                                    {move || {
                                        mp_info_sig
                                            .get()
                                            .map(|i| i.name)
                                            .unwrap_or_else(|| "Загрузка...".to_string())
                                    }}
                                </Button>
                            </div>
                            <div style="display: grid; grid-template-columns: 1fr 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"Created"</label>
                                    <FieldDisplay value=created_at />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Updated"</label>
                                    <FieldDisplay value=updated_at />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Version"</label>
                                    <FieldDisplay value=version />
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
