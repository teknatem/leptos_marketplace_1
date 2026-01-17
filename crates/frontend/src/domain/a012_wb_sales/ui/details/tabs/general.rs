//! General tab - document info, status, warehouse, and links

use super::super::view_model::WbSalesDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::clipboard::copy_to_clipboard;
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
            let wh_name = sale_data.warehouse.warehouse_name.clone().unwrap_or_else(|| "—".to_string());
            let wh_type = sale_data.warehouse.warehouse_type.clone().unwrap_or_else(|| "—".to_string());
            let created_at = format_datetime(&sale_data.metadata.created_at);
            let updated_at = format_datetime(&sale_data.metadata.updated_at);
            let version = sale_data.metadata.version.to_string();
            let mp_ref = sale_data.marketplace_product_ref.clone();
            let nom_ref = sale_data.nomenclature_ref.clone();

            // Open marketplace product handler
            let open_mp = {
                let tabs_store = tabs_store;
                let mp_ref = mp_ref.clone();
                let mp_info = vm.marketplace_product_info;
                move |_| {
                    let Some(mp_id) = mp_ref.clone() else { return; };
                    let title = mp_info
                        .get()
                        .map(|info| {
                            if info.article.trim().is_empty() {
                                format!("Товар МП {}", info.description)
                            } else {
                                format!("Товар МП {}", info.article)
                            }
                        })
                        .unwrap_or_else(|| "Товар МП".to_string());
                    tabs_store.open_tab(&format!("a007_marketplace_product_detail_{}", mp_id), &title);
                }
            };

            // Open nomenclature handler
            let open_nom = {
                let tabs_store = tabs_store;
                let nom_ref = nom_ref.clone();
                let nom_info = vm.nomenclature_info;
                move |_| {
                    let Some(nom_id) = nom_ref.clone() else { return; };
                    let title = nom_info
                        .get()
                        .map(|info| {
                            if info.article.trim().is_empty() {
                                format!("Номенклатура {}", info.description)
                            } else {
                                format!("Номенклатура {}", info.article)
                            }
                        })
                        .unwrap_or_else(|| "Номенклатура".to_string());
                    tabs_store.open_tab(&format!("a004_nomenclature_detail_{}", nom_id), &title);
                }
            };

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
                        <div class="form__group">
                            <label class="form__label">"Дата (sale dt)"</label>
                            <Input value=RwSignal::new(sale_dt) attr:readonly=true />
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
                            <Button
                                appearance=ButtonAppearance::Subtle
                                size=ButtonSize::Small
                                on_click=open_mp
                                disabled=Signal::derive(move || !has_mp_ref)
                            >
                                {move || {
                                    vm.marketplace_product_info
                                        .get()
                                        .map(|i| if i.article.trim().is_empty() { i.description } else { i.article })
                                        .unwrap_or_else(|| "Открыть".to_string())
                                }}
                            </Button>
                        </div>
                        <div class="form__group">
                            <label class="form__label">"Номенклатура 1С"</label>
                            <Button
                                appearance=ButtonAppearance::Subtle
                                size=ButtonSize::Small
                                on_click=open_nom
                                disabled=Signal::derive(move || !has_nom_ref)
                            >
                                {move || {
                                    vm.nomenclature_info
                                        .get()
                                        .map(|i| if i.article.trim().is_empty() { i.description } else { i.article })
                                        .unwrap_or_else(|| "Открыть".to_string())
                                }}
                            </Button>
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
                            <label class="form__label">"Connection ID"</label>
                            <IdWithCopy value=conn_id.clone() />
                        </div>
                        <div class="form__group">
                            <label class="form__label">"Organization ID"</label>
                            <IdWithCopy value=org_id.clone() />
                        </div>
                        <div class="form__group">
                            <label class="form__label">"Marketplace ID"</label>
                            <IdWithCopy value=mp_id.clone() />
                        </div>
                    </Card>

                </Flex>

                </div>
            }.into_any()
        }}
    }
}

/// ID field with copy button (disabled input + copy)
#[component]
fn IdWithCopy(value: String) -> impl IntoView {
    let value_for_copy = value.clone();

    view! {
        <Flex gap=FlexGap::Small style="align-items: center;">
            <Input value=RwSignal::new(value) attr:readonly=true attr:style="flex: 1;" />
            <Button
                appearance=ButtonAppearance::Subtle
                shape=ButtonShape::Square
                size=ButtonSize::Small
                on_click=move |_| copy_to_clipboard(&value_for_copy)
            >
                "⧉"
            </Button>
        </Flex>
    }
}
