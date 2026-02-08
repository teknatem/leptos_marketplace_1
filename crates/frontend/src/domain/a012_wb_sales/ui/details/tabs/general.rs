//! General tab - document info, status, warehouse, and links

use super::super::view_model::WbSalesDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::date_utils::format_datetime;
use leptos::prelude::*;
use thaw::*;

/// General tab component - displays document overview cards
#[component]
pub fn GeneralTab(vm: WbSalesDetailsVm) -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    view! {
        {move || {
            let Some(sale_data) = vm.sale.get() else {
                return view! { <div>"Нет данных"</div> }.into_any();
            };

            let conn_id = sale_data.header.connection_id.clone();
            let org_id = sale_data.header.organization_id.clone();
            let mp_id = sale_data.header.marketplace_id.clone();
            let document_no = sale_data.header.document_no.clone();
            let sale_id = sale_data.header.sale_id.clone().unwrap_or_else(|| "—".to_string());
            let code = sale_data.code.clone();
            let description = sale_data.description.clone();
            let event_type = sale_data.state.event_type.clone();
            let status_norm = sale_data.state.status_norm.clone();
            let sale_dt = format_datetime(&sale_data.state.sale_dt);
            let last_change_dt = sale_data.state.last_change_dt.as_ref()
                .map(|d| format_datetime(d))
                .unwrap_or_else(|| "—".to_string());
            let is_supply = sale_data.state.is_supply.unwrap_or(false);
            let is_realization = sale_data.state.is_realization.unwrap_or(false);
            let is_fact = sale_data.line.is_fact.unwrap_or(false);
            let wh_name = sale_data.warehouse.warehouse_name.clone().unwrap_or_else(|| "—".to_string());
            let wh_type = sale_data.warehouse.warehouse_type.clone().unwrap_or_else(|| "—".to_string());
            let created_at = format_datetime(&sale_data.metadata.created_at);
            let updated_at = format_datetime(&sale_data.metadata.updated_at);
            let version = sale_data.metadata.version.to_string();
            let mp_ref = sale_data.marketplace_product_ref.clone();
            let nom_ref = sale_data.nomenclature_ref.clone();

            // Check if refs exist for disabling buttons
            let has_mp_ref = mp_ref.is_some();
            let has_nom_ref = nom_ref.is_some();

            view! {
                <div style="display: grid; grid-template-columns: 600px 600px; gap: var(--spacing-md); max-width: 1250px; align-items: start; align-content: start;">

                //left column
                <Flex vertical=true gap=FlexGap::Medium>
                    // Document card
                    <Card attr:style="width: 600px; margin: 0px;">
                        <h4 class="details-section__title">"Документ"</h4>
                        <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                            <div class="form__group">
                                <label class="form__label">"Дата (sale dt)"</label>
                                <Input value=RwSignal::new(sale_dt) attr:readonly=true />
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Sale ID"</label>
                                <Input value=RwSignal::new(sale_id) attr:readonly=true />
                            </div>
                        </div>
                        <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                            <div class="form__group">
                                <label class="form__label">"№"</label>
                                <Input value=RwSignal::new(document_no) attr:readonly=true />
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Code"</label>
                                <Input value=RwSignal::new(code) attr:readonly=true />
                            </div>
                        </div>

                    </Card>
                    // Goods card
                    <Card attr:style="width: 600px; margin: 0px;">
                        <h4 class="details-section__title">"Номенклатура"</h4>
                        <div class="form__group">
                            <label class="form__label">"Товар маркетплейса"</label>
                            <div style="padding: 8px 0; text-align: left;">
                                {
                                    let mp_ref_click = mp_ref.clone();
                                    view! {
                                        <Show
                                            when=move || has_mp_ref
                                            fallback=|| view! { <span style="color: var(--color-text-secondary);">"—"</span> }
                                        >
                                            <a
                                                href="#"
                                                on:click={
                                                    let mp_ref_inner = mp_ref_click.clone();
                                                    move |ev: web_sys::MouseEvent| {
                                                        ev.prevent_default();
                                                        if let Some(ref mp_id) = mp_ref_inner {
                                                            let title = vm
                                                                .marketplace_product_info
                                                                .get()
                                                                .map(|info| {
                                                                    if info.article.trim().is_empty() {
                                                                        format!("Товар МП {}", info.description)
                                                                    } else {
                                                                        format!("Товар МП {}", info.article)
                                                                    }
                                                                })
                                                                .unwrap_or_else(|| "Товар МП".to_string());
                                                            tabs_store.open_tab(
                                                                &format!("a007_marketplace_product_detail_{}", mp_id),
                                                                &title,
                                                            );
                                                        }
                                                    }
                                                }
                                                style="color: #0078d4; text-decoration: underline; cursor: pointer; font-weight: 500;"
                                            >
                                                {move || {
                                                    vm.marketplace_product_info
                                                        .get()
                                                        .map(|i| {
                                                            if i.description.trim().is_empty() {
                                                                if i.article.trim().is_empty() {
                                                                    "Открыть".to_string()
                                                                } else {
                                                                    format!("арт. {}", i.article)
                                                                }
                                                            } else if i.article.trim().is_empty() {
                                                                i.description
                                                            } else {
                                                                format!("{} (арт. {})", i.description, i.article)
                                                            }
                                                        })
                                                        .unwrap_or_else(|| "Открыть".to_string())
                                                }}
                                            </a>
                                        </Show>
                                    }
                                }
                            </div>
                        </div>
                        <div class="form__group">
                            <label class="form__label">"Номенклатура 1С"</label>
                            <div style="padding: 8px 0; text-align: left;">
                                {
                                    let nom_ref_click = nom_ref.clone();
                                    view! {
                                        <Show
                                            when=move || has_nom_ref
                                            fallback=|| view! { <span style="color: var(--color-text-secondary);">"—"</span> }
                                        >
                                            <a
                                                href="#"
                                                on:click={
                                                    let nom_ref_inner = nom_ref_click.clone();
                                                    move |ev: web_sys::MouseEvent| {
                                                        ev.prevent_default();
                                                        if let Some(ref nom_id) = nom_ref_inner {
                                                            let title = vm
                                                                .nomenclature_info
                                                                .get()
                                                                .map(|info| {
                                                                    if info.article.trim().is_empty() {
                                                                        format!("Номенклатура {}", info.description)
                                                                    } else {
                                                                        format!("Номенклатура {}", info.article)
                                                                    }
                                                                })
                                                                .unwrap_or_else(|| "Номенклатура".to_string());
                                                            tabs_store.open_tab(
                                                                &format!("a004_nomenclature_detail_{}", nom_id),
                                                                &title,
                                                            );
                                                        }
                                                    }
                                                }
                                                style="color: #0078d4; text-decoration: underline; cursor: pointer; font-weight: 500;"
                                            >
                                                {move || {
                                                    vm.nomenclature_info
                                                        .get()
                                                        .map(|i| {
                                                            if i.description.trim().is_empty() {
                                                                if i.article.trim().is_empty() {
                                                                    "Открыть".to_string()
                                                                } else {
                                                                    format!("арт. {}", i.article)
                                                                }
                                                            } else if i.article.trim().is_empty() {
                                                                i.description
                                                            } else {
                                                                format!("{} (арт. {})", i.description, i.article)
                                                            }
                                                        })
                                                        .unwrap_or_else(|| "Открыть".to_string())
                                                }}
                                            </a>
                                        </Show>
                                    }
                                }
                            </div>
                        </div>

                    </Card>
                    // Warehouse card
                    <Card attr:style="width: 600px; margin: 0px;">
                        <h4 class="details-section__title">"Склад"</h4>
                        <div class="form__group">
                            <label class="form__label">"Название"</label>
                            <Input value=RwSignal::new(wh_name) attr:readonly=true />
                        </div>
                        <div class="form__group">
                            <label class="form__label">"Тип"</label>
                            <Input value=RwSignal::new(wh_type) attr:readonly=true />
                        </div>
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
                    </Card>

                </Flex>

                //right column
                <Flex vertical=true gap=FlexGap::Medium>
                // Status card
                <Card attr:style="width: 600px; margin: 0px;">
                <h4 class="details-section__title">"Статус"</h4>
                <Flex gap=FlexGap::Small style="margin-bottom: var(--spacing-md);">
                    <Badge
                        appearance=BadgeAppearance::Filled
                        color=if is_fact { BadgeColor::Success } else { BadgeColor::Informative }
                    >
                        {if is_fact { "Факт" } else { "План" }}
                    </Badge>
                    <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                        {event_type}
                    </Badge>
                    <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Success>
                        {status_norm}
                    </Badge>
                    <Badge
                        appearance=BadgeAppearance::Outline
                        color=if is_supply { BadgeColor::Success } else { BadgeColor::Danger }
                    >
                        {if is_supply { "Supply: Yes" } else { "Supply: No" }}
                    </Badge>
                    <Badge
                        appearance=BadgeAppearance::Outline
                        color=if is_realization { BadgeColor::Success } else { BadgeColor::Danger }
                    >
                        {if is_realization { "Realization: Yes" } else { "Realization: No" }}
                    </Badge>
                </Flex>
                <div class="form__group">
                    <label class="form__label">"Last change"</label>
                    <Input value=RwSignal::new(last_change_dt) attr:readonly=true />
                </div>
                <div class="form__group">
                    <label class="form__label">"Описание"</label>
                    <Input value=RwSignal::new(description) attr:readonly=true />
                </div>
            </Card>
                    // Links card
                    <Card attr:style="width: 600px; margin: 0px;">
                        <h4 class="details-section__title">"Связи"</h4>
                        <div class="form__group">
                            <label class="form__label">"Подключение"</label>
                            <Flex gap=FlexGap::Small style="align-items: center;">
                                <Button
                                    appearance=ButtonAppearance::Subtle
                                    size=ButtonSize::Small
                                    on_click={
                                        let tabs_store = tabs_store;
                                        let conn_id = conn_id.clone();
                                        move |_| {
                                            tabs_store.open_tab(&format!("a006_connection_mp_detail_{}", conn_id), "Подключение МП");
                                        }
                                    }
                                    attr:style="flex: 1; justify-content: flex-start;"
                                >
                                    {move || {
                                        vm.connection_info
                                            .get()
                                            .map(|i| i.description)
                                            .unwrap_or_else(|| "Загрузка...".to_string())
                                    }}
                                </Button>
                            </Flex>
                        </div>
                        <div class="form__group">
                            <label class="form__label">"Организация"</label>
                            <Flex gap=FlexGap::Small style="align-items: center;">
                                <Button
                                    appearance=ButtonAppearance::Subtle
                                    size=ButtonSize::Small
                                    on_click={
                                        let tabs_store = tabs_store;
                                        let org_id = org_id.clone();
                                        move |_| {
                                            tabs_store.open_tab(&format!("a002_organization_detail_{}", org_id), "Организация");
                                        }
                                    }
                                    attr:style="flex: 1; justify-content: flex-start;"
                                >
                                    {move || {
                                        vm.organization_info
                                            .get()
                                            .map(|i| i.description)
                                            .unwrap_or_else(|| "Загрузка...".to_string())
                                    }}
                                </Button>
                            </Flex>
                        </div>
                        <div class="form__group">
                            <label class="form__label">"Маркетплейс"</label>
                            <Flex gap=FlexGap::Small style="align-items: center;">
                                <Button
                                    appearance=ButtonAppearance::Subtle
                                    size=ButtonSize::Small
                                    on_click={
                                        let tabs_store = tabs_store;
                                        let mp_id = mp_id.clone();
                                        move |_| {
                                            tabs_store.open_tab(&format!("a005_marketplace_detail_{}", mp_id), "Маркетплейс");
                                        }
                                    }
                                    attr:style="flex: 1; justify-content: flex-start;"
                                >
                                    {move || {
                                        vm.marketplace_info
                                            .get()
                                            .map(|i| i.name)
                                            .unwrap_or_else(|| "Загрузка...".to_string())
                                    }}
                                </Button>
                            </Flex>
                        </div>
                    </Card>

                </Flex>

                </div>
            }.into_any()
        }}
    }
}
