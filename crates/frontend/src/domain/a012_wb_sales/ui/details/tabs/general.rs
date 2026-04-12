//! General tab - document info, status, warehouse, and links

use super::super::view_model::WbSalesDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::date_utils::format_datetime;
use leptos::prelude::*;
use thaw::*;

fn format_prod_cost_status(status: &str) -> String {
    match status {
        "ok" => "ok".to_string(),
        "missing_nomenclature_ref" => "нет номенклатуры".to_string(),
        "nomenclature_not_found" => "номенклатура не найдена".to_string(),
        "missing_p912_cost" => "нет себестоимости p912".to_string(),
        "kit_variant_not_found" => "не найден состав комплекта".to_string(),
        "empty_kit" => "пустой состав комплекта".to_string(),
        "missing_component_costs" => "нет себестоимости компонентов".to_string(),
        "not_checked" => "не проверено".to_string(),
        _ => status.to_string(),
    }
}

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
            let last_change_dt = sale_data
                .state
                .last_change_dt
                .as_ref()
                .map(|d| format_datetime(d))
                .unwrap_or_else(|| "—".to_string());
            let is_supply = sale_data.state.is_supply.unwrap_or(false);
            let is_realization = sale_data.state.is_realization.unwrap_or(false);
            let is_fact = sale_data.line.is_fact.unwrap_or(false);
            let prod_cost_problem = sale_data.prod_cost_problem;
            let prod_cost_checked = sale_data.prod_cost_status.is_some();
            let prod_cost_status = sale_data
                .prod_cost_status
                .clone()
                .map(|value| format_prod_cost_status(&value))
                .unwrap_or_else(|| "не проверено".to_string());
            let prod_cost_problem_message = sale_data
                .prod_cost_problem_message
                .clone()
                .unwrap_or_else(|| {
                    if prod_cost_checked {
                        "Проблемы не выявлены".to_string()
                    } else {
                        "Проверка prod-себестоимости ещё не выполнялась".to_string()
                    }
                });
            let prod_cost_resolved_total = sale_data
                .prod_cost_resolved_total
                .map(|value| format!("{value:.2}"))
                .unwrap_or_else(|| "—".to_string());
            let wh_name = sale_data
                .warehouse
                .warehouse_name
                .clone()
                .unwrap_or_else(|| "—".to_string());
            let wh_type = sale_data
                .warehouse
                .warehouse_type
                .clone()
                .unwrap_or_else(|| "—".to_string());
            let created_at = format_datetime(&sale_data.metadata.created_at);
            let updated_at = format_datetime(&sale_data.metadata.updated_at);
            let version = sale_data.metadata.version.to_string();
            let mp_ref = sale_data.marketplace_product_ref.clone();
            let nom_ref = sale_data.nomenclature_ref.clone();
            let mp_ref_click = mp_ref.clone();
            let mp_ref_text = mp_ref.clone();
            let nom_ref_click = nom_ref.clone();
            let nom_ref_text = nom_ref.clone();

            view! {
                <div class="detail-grid">
                    // ── Левая колонка ────────────────────────────────────────
                    <div class="detail-grid__col">
                        <CardAnimated delay_ms=0 nav_id="a012_wb_sales_details_general_document">
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
                            <div class="form__group">
                                <label class="form__label">"Описание"</label>
                                <Input value=RwSignal::new(description) attr:readonly=true />
                            </div>
                        </CardAnimated>

                        <CardAnimated delay_ms=80 nav_id="a012_wb_sales_details_general_links">
                            <h4 class="details-section__title">"Номенклатура"</h4>
                            <div class="form__group">
                                <label class="form__label">"Товар маркетплейса"</label>
                                <a
                                    href="#"
                                    on:click={
                                        let mp_ref = mp_ref_click.clone();
                                        move |ev: web_sys::MouseEvent| {
                                            ev.prevent_default();
                                            if let Some(ref id) = mp_ref {
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
                                                    &format!("a007_marketplace_product_details_{}", id),
                                                    &title,
                                                );
                                            }
                                        }
                                    }
                                    style="color: var(--color-primary); text-decoration: underline; cursor: pointer; font-weight: 500;"
                                >
                                    {move || {
                                        if mp_ref_text.is_none() {
                                            return "—".to_string();
                                        }
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
                                                    &format!("a004_nomenclature_details_{}", id),
                                                    &title,
                                                );
                                            }
                                        }
                                    }
                                    style="color: var(--color-primary); text-decoration: underline; cursor: pointer; font-weight: 500;"
                                >
                                    {move || {
                                        if nom_ref_text.is_none() {
                                            return "—".to_string();
                                        }
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
                            </div>
                        </CardAnimated>

                        <CardAnimated delay_ms=160 nav_id="a012_wb_sales_details_general_warehouse">
                            <h4 class="details-section__title">"Склад"</h4>
                            <div class="form__group">
                                <label class="form__label">"Название"</label>
                                <Input value=RwSignal::new(wh_name) attr:readonly=true />
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Тип"</label>
                                <Input value=RwSignal::new(wh_type) attr:readonly=true />
                            </div>
                        </CardAnimated>
                    </div>

                    // ── Правая колонка ───────────────────────────────────────
                    <div class="detail-grid__col">
                        <CardAnimated delay_ms=40 nav_id="a012_wb_sales_details_general_status">
                            <h4 class="details-section__title">"Статус"</h4>
                            <div style="margin: var(--spacing-sm); display: flex; flex-wrap: wrap; gap: var(--spacing-sm);">
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
                                <Badge
                                    appearance=BadgeAppearance::Filled
                                    color=if prod_cost_problem {
                                        BadgeColor::Warning
                                    } else if prod_cost_checked {
                                        BadgeColor::Success
                                    } else {
                                        BadgeColor::Informative
                                    }
                                >
                                    {if prod_cost_problem {
                                        "Prod cost: problem"
                                    } else if prod_cost_checked {
                                        "Prod cost: ok"
                                    } else {
                                        "Prod cost: not checked"
                                    }}
                                </Badge>
                            </div>
                            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-sm);">
                                <div class="form__group">
                                    <label class="form__label">"Prod cost status"</label>
                                    <Input value=RwSignal::new(prod_cost_status) attr:readonly=true />
                                </div>
                                <div class="form__group">
                                    <label class="form__label">"Prod cost total"</label>
                                    <Input value=RwSignal::new(prod_cost_resolved_total) attr:readonly=true />
                                </div>
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Prod cost diagnostics"</label>
                                <Textarea value=RwSignal::new(prod_cost_problem_message) attr:readonly=true attr:rows=3 />
                            </div>
                            <div class="form__group">
                                <label class="form__label">"Last change"</label>
                                <Input value=RwSignal::new(last_change_dt) attr:readonly=true />
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

                        <CardAnimated delay_ms=120 nav_id="a012_wb_sales_details_general_tech_links">
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
                        </CardAnimated>
                    </div>
                </div>
            }
            .into_any()
        }}
    }
}
