//! General tab - document info and linked entities

use super::super::view_model::WbOrdersDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::date_utils::format_datetime;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn GeneralTab(vm: WbOrdersDetailsVm) -> impl IntoView {
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
            let order_dt = format_datetime(&order_data.state.order_dt);
            let last_change_dt = order_data
                .state
                .last_change_dt
                .as_ref()
                .map(|d| format_datetime(d))
                .unwrap_or_else(|| "—".to_string());
            let cancel_dt = order_data
                .state
                .cancel_dt
                .as_ref()
                .map(|d| format_datetime(d))
                .unwrap_or_else(|| "—".to_string());
            let is_cancel = order_data.state.is_cancel;
            let is_supply = order_data.state.is_supply.unwrap_or(false);
            let is_realization = order_data.state.is_realization.unwrap_or(false);
            let wh_name = order_data
                .warehouse
                .warehouse_name
                .clone()
                .unwrap_or_else(|| "—".to_string());
            let wh_type = order_data
                .warehouse
                .warehouse_type
                .clone()
                .unwrap_or_else(|| "—".to_string());
            let country = order_data
                .geography
                .country_name
                .clone()
                .unwrap_or_else(|| "—".to_string());
            let region = order_data
                .geography
                .region_name
                .clone()
                .unwrap_or_else(|| "—".to_string());
            let district = order_data
                .geography
                .oblast_okrug_name
                .clone()
                .unwrap_or_else(|| "—".to_string());
            let created_at = format_datetime(&order_data.metadata.created_at);
            let updated_at = format_datetime(&order_data.metadata.updated_at);
            let version = order_data.metadata.version.to_string();
            let mp_ref = order_data.marketplace_product_ref.clone();
            let nom_ref = order_data.nomenclature_ref.clone();
            let base_nom_ref = order_data.base_nomenclature_ref.clone();
            let mp_ref_click = mp_ref.clone();
            let mp_ref_text = mp_ref.clone();
            let nom_ref_click = nom_ref.clone();
            let nom_ref_text = nom_ref.clone();
            let base_nom_ref_click = base_nom_ref.clone();
            let base_nom_ref_text = base_nom_ref.clone();
            let line = order_data.line;

            view! {
                <div class="detail-grid">
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
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"Дата заказа"</label>
                                    <Input value=RwSignal::new(order_dt) attr:readonly=true />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Последнее изменение"</label>
                                    <Input value=RwSignal::new(last_change_dt) attr:readonly=true />
                                </div>
                            </div>
                        </CardAnimated>

                        <CardAnimated delay_ms=80>
                            <h4 class="details-section__title">"Связанные объекты"</h4>
                            <div class="form__group">
                                <label class="form__label">"Товар маркетплейса"</label>
                                <a
                                    href="#"
                                    on:click={
                                        let mp_ref = mp_ref_click.clone();
                                        move |ev: web_sys::MouseEvent| {
                                            ev.prevent_default();
                                            if let Some(ref id) = mp_ref {
                                                tabs_store.open_tab(
                                                    &format!("a007_marketplace_product_detail_{}", id),
                                                    "Товар МП",
                                                );
                                            }
                                        }
                                    }
                                    style="color: #0078d4; text-decoration: underline; cursor: pointer;"
                                >
                                    {move || {
                                        if mp_ref_text.is_none() {
                                            return "—".to_string();
                                        }
                                        vm.marketplace_product_info
                                            .get()
                                            .map(|i| {
                                                if i.article.trim().is_empty() {
                                                    i.description
                                                } else {
                                                    format!("{} (арт. {})", i.description, i.article)
                                                }
                                            })
                                            .unwrap_or_else(|| "Открыть".to_string())
                                    }}
                                </a>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Номенклатура 1С"</label>
                                <a
                                    href="#"
                                    on:click={
                                        let nom_ref = nom_ref_click.clone();
                                        move |ev: web_sys::MouseEvent| {
                                            ev.prevent_default();
                                            if let Some(ref id) = nom_ref {
                                                tabs_store.open_tab(
                                                    &format!("a004_nomenclature_detail_{}", id),
                                                    "Номенклатура",
                                                );
                                            }
                                        }
                                    }
                                    style="color: #0078d4; text-decoration: underline; cursor: pointer;"
                                >
                                    {move || {
                                        if nom_ref_text.is_none() {
                                            return "—".to_string();
                                        }
                                        vm.nomenclature_info
                                            .get()
                                            .map(|i| {
                                                if i.article.trim().is_empty() {
                                                    i.description
                                                } else {
                                                    format!("{} (арт. {})", i.description, i.article)
                                                }
                                            })
                                            .unwrap_or_else(|| "Открыть".to_string())
                                    }}
                                </a>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Базовая номенклатура 1С"</label>
                                <a
                                    href="#"
                                    on:click={
                                        let base_nom_ref = base_nom_ref_click.clone();
                                        move |ev: web_sys::MouseEvent| {
                                            ev.prevent_default();
                                            if let Some(ref id) = base_nom_ref {
                                                tabs_store.open_tab(
                                                    &format!("a004_nomenclature_detail_{}", id),
                                                    "Базовая номенклатура",
                                                );
                                            }
                                        }
                                    }
                                    style="color: #0078d4; text-decoration: underline; cursor: pointer;"
                                >
                                    {move || {
                                        if base_nom_ref_text.is_none() {
                                            return "—".to_string();
                                        }
                                        vm.base_nomenclature_info
                                            .get()
                                            .map(|i| {
                                                if i.article.trim().is_empty() {
                                                    i.description
                                                } else {
                                                    format!("{} (арт. {})", i.description, i.article)
                                                }
                                            })
                                            .unwrap_or_else(|| "Открыть".to_string())
                                    }}
                                </a>
                            </div>
                        </CardAnimated>

                        <CardAnimated delay_ms=160>
                            <h4 class="details-section__title">"Позиция заказа"</h4>
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"Line ID"</label>
                                    <Input value=RwSignal::new(line.line_id) attr:readonly=true />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Артикул продавца"</label>
                                    <Input value=RwSignal::new(line.supplier_article) attr:readonly=true />
                                </div>
                            </div>
                            <div style="display: grid; grid-template-columns: 1fr 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"NM ID"</label>
                                    <Input value=RwSignal::new(line.nm_id.to_string()) attr:readonly=true />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Бренд"</label>
                                    <Input value=RwSignal::new(line.brand.unwrap_or_else(|| "—".to_string())) attr:readonly=true />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Размер"</label>
                                    <Input value=RwSignal::new(line.tech_size.unwrap_or_else(|| "—".to_string())) attr:readonly=true />
                                </div>
                            </div>
                            <div style="display: grid; grid-template-columns: 1fr 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"Штрихкод"</label>
                                    <Input value=RwSignal::new(line.barcode) attr:readonly=true />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Категория"</label>
                                    <Input value=RwSignal::new(line.category.unwrap_or_else(|| "—".to_string())) attr:readonly=true />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Предмет"</label>
                                    <Input value=RwSignal::new(line.subject.unwrap_or_else(|| "—".to_string())) attr:readonly=true />
                                </div>
                            </div>
                        </CardAnimated>
                    </div>

                    <div class="detail-grid__col">
                        <CardAnimated delay_ms=40>
                            <h4 class="details-section__title">"Статус и склад"</h4>
                            <Flex gap=FlexGap::Small style="margin-bottom: var(--spacing-md); flex-wrap: wrap;">
                                <Badge appearance=BadgeAppearance::Tint color=if is_cancel { BadgeColor::Danger } else { BadgeColor::Success }>
                                    {if is_cancel { "Отменен" } else { "Активен" }}
                                </Badge>
                                <Badge appearance=BadgeAppearance::Outline color=if is_supply { BadgeColor::Success } else { BadgeColor::Danger }>
                                    {if is_supply { "Supply: Yes" } else { "Supply: No" }}
                                </Badge>
                                <Badge appearance=BadgeAppearance::Outline color=if is_realization { BadgeColor::Success } else { BadgeColor::Danger }>
                                    {if is_realization { "Realization: Yes" } else { "Realization: No" }}
                                </Badge>
                            </Flex>
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"Warehouse"</label>
                                    <Input value=RwSignal::new(wh_name) attr:readonly=true />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Warehouse type"</label>
                                    <Input value=RwSignal::new(wh_type) attr:readonly=true />
                                </div>
                            </div>
                            <div style="display: grid; grid-template-columns: 1fr 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"Страна"</label>
                                    <Input value=RwSignal::new(country) attr:readonly=true />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Округ"</label>
                                    <Input value=RwSignal::new(district) attr:readonly=true />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Регион"</label>
                                    <Input value=RwSignal::new(region) attr:readonly=true />
                                </div>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Дата отмены"</label>
                                <Input value=RwSignal::new(cancel_dt) attr:readonly=true />
                            </div>
                        </CardAnimated>

                        <CardAnimated delay_ms=120>
                            <h4 class="details-section__title">"Технические связи"</h4>
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
                                        move |_| tabs_store.open_tab(&format!("a002_organization_detail_{}", org_id), "Организация")
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
                                        move |_| tabs_store.open_tab(&format!("a005_marketplace_detail_{}", mp_id), "Маркетплейс")
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
