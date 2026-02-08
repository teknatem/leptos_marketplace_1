//! Main page component for Nomenclature details (EditDetails MVVM Standard)
//!
//! Thin wrapper that:
//! - Creates ViewModel
//! - Renders header with action buttons
//! - Renders tab bar
//! - Routes to tab components
//! - Handles lazy loading for nested data

use super::tabs::{BarcodesTab, DealerPricesTab, DimensionsTab, GeneralTab};
use super::view_model::NomenclatureDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use leptos::prelude::*;
use thaw::*;

/// Main component for Nomenclature details form
#[component]
pub fn NomenclatureDetails(
    id: Option<String>,
    #[prop(into)] on_saved: Callback<()>,
    #[prop(into)] on_cancel: Callback<()>,
) -> impl IntoView {
    // Create ViewModel
    let vm = NomenclatureDetailsVm::new();

    // Load dimension options (always needed)
    vm.load_dimension_options();

    // Load entity data if editing
    if let Some(existing_id) = id {
        vm.load(existing_id);
    }

    // Lazy loading for barcodes tab
    Effect::new({
        let vm = vm.clone();
        move || {
            if vm.active_tab.get() == "barcodes" && !vm.barcodes_loaded.get() {
                vm.load_barcodes();
            }
        }
    });

    // Lazy loading for dealer prices tab
    Effect::new({
        let vm = vm.clone();
        move || {
            if vm.active_tab.get() == "dealer_prices" && !vm.dealer_prices_loaded.get() {
                vm.load_dealer_prices();
            }
        }
    });

    // Clone for closures
    let vm_header = vm.clone();
    let vm_tabs = vm.clone();
    let vm_content = vm.clone();

    view! {
        <div class="details-container nomenclature-details">
            // Header
            <Header vm=vm_header.clone() on_saved=on_saved on_cancel=on_cancel />

            <div class="modal-body">
                // Error display
                <ErrorDisplay vm=vm.clone() />

                // Tab bar
                <TabBar vm=vm_tabs.clone() />

                // Derivative warning (between tabs and content)
                <DerivativeWarning vm=vm.clone() />

                // Tab content
                <div style="height: 60vh; overflow-y: auto; overflow-x: hidden;">
                    <TabContent vm=vm_content.clone() />
                </div>
            </div>
        </div>
    }
}

/// Header component with title and action buttons
#[component]
fn Header(
    vm: NomenclatureDetailsVm,
    on_saved: Callback<()>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let is_edit_mode = vm.is_edit_mode();
    let is_save_disabled = vm.is_save_disabled();

    let handle_save = {
        let vm = vm.clone();
        move |_| {
            vm.save(on_saved);
        }
    };

    view! {
        <div class="modal-header">
            <h3 class="modal-title">
                {move || if is_edit_mode.get() {
                    "Редактирование номенклатуры"
                } else {
                    "Новая номенклатура"
                }}
            </h3>
            <div class="modal-header-actions">
                <Button
                    appearance=ButtonAppearance::Primary
                    on_click=handle_save
                    disabled=is_save_disabled
                >
                    {icon("save")}
                    " Сохранить"
                </Button>
                <Button
                    appearance=ButtonAppearance::Secondary
                    on_click=move |_| on_cancel.run(())
                >
                    {icon("x")}
                    " Закрыть"
                </Button>
            </div>
        </div>
    }
}

/// Error display component
#[component]
fn ErrorDisplay(vm: NomenclatureDetailsVm) -> impl IntoView {
    let error = vm.error;

    view! {
        {move || error.get().map(|e| view! {
            <div class="warning-box" style="background: var(--color-error-50); border-color: var(--color-error-100); margin-bottom: var(--spacing-md);">
                <span class="warning-box__icon" style="color: var(--color-error);">"⚠"</span>
                <span class="warning-box__text" style="color: var(--color-error);">{e}</span>
            </div>
        })}
    }
}

/// Derivative warning component - displays warning when this is a derivative nomenclature
#[component]
fn DerivativeWarning(vm: NomenclatureDetailsVm) -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    view! {
        <Show when=move || vm.is_derivative.get()>
            <div style="width: 100%; margin-bottom: var(--spacing-md);">
                <MessageBar intent=MessageBarIntent::Warning>
                    <div style="display: flex; align-items: center; gap: var(--spacing-sm); flex-wrap: wrap;">
                        <span>"Это производная позиция от:"</span>
                        {move || {
                            let article = vm.base_nomenclature_article.get();
                            let name = vm.base_nomenclature_name.get();
                            let base_id = vm.base_nomenclature_ref.get();

                            if !article.is_empty() || !name.is_empty() {
                                let display_article = if article.is_empty() { "—".to_string() } else { article.clone() };
                                let title = if article.is_empty() {
                                    format!("Номенклатура {}", name)
                                } else {
                                    format!("Номенклатура {}", article)
                                };

                                view! {
                                    <Button
                                        appearance=ButtonAppearance::Transparent
                                        size=ButtonSize::Small
                                        on_click={
                                            let tabs_store = tabs_store;
                                            let base_id = base_id.clone();
                                            let title = title.clone();
                                            move |_| {
                                                if !base_id.is_empty() {
                                                    tabs_store.open_tab(&format!("a004_nomenclature_detail_{}", base_id), &title);
                                                }
                                            }
                                        }
                                        attr:style="color: var(--color-primary); text-decoration: underline; padding: 0; min-height: auto; font-weight: 600;"
                                    >
                                        {display_article}
                                    </Button>
                                    <span>{name}</span>
                                }.into_any()
                            } else {
                                view! {
                                    <span>{base_id}</span>
                                }.into_any()
                            }
                        }}
                    </div>
                </MessageBar>
            </div>
        </Show>
    }
}

/// Tab bar component using THAW buttons for better visual clarity
#[component]
fn TabBar(vm: NomenclatureDetailsVm) -> impl IntoView {
    let active_tab = vm.active_tab;
    let is_edit_mode = vm.is_edit_mode();
    let barcodes_count = vm.barcodes_count;
    let dealer_prices_count = vm.dealer_prices_count;

    // Helper to create tab button content with proper icon spacing
    let tab_icon = |name: &str| {
        view! { <span class="tab-icon">{icon(name)}</span> }
    };

    view! {
        <Flex
            gap=FlexGap::Small
            align=FlexAlign::Center
            style="margin-bottom: var(--spacing-md); padding: var(--spacing-sm); background: var(--color-bg-secondary); border-radius: var(--radius-lg); border: 1px solid var(--color-border);"
        >
            // General tab
            <Button
                appearance=Signal::derive({
                    let active_tab = active_tab;
                    move || if active_tab.get() == "general" {
                        ButtonAppearance::Primary
                    } else {
                        ButtonAppearance::Subtle
                    }
                })
                size=ButtonSize::Small
                on_click={
                    let vm = vm.clone();
                    move |_| vm.set_tab("general")
                }
            >
                {tab_icon("file-text")}
                "Основная"
            </Button>

            // Dimensions tab
            <Button
                appearance=Signal::derive({
                    let active_tab = active_tab;
                    move || if active_tab.get() == "dimensions" {
                        ButtonAppearance::Primary
                    } else {
                        ButtonAppearance::Subtle
                    }
                })
                size=ButtonSize::Small
                on_click={
                    let vm = vm.clone();
                    move |_| vm.set_tab("dimensions")
                }
            >
                {tab_icon("sliders")}
                "Измерения"
            </Button>

            // Barcodes tab (disabled for new records)
            <Button
                appearance=Signal::derive({
                    let active_tab = active_tab;
                    move || if active_tab.get() == "barcodes" {
                        ButtonAppearance::Primary
                    } else {
                        ButtonAppearance::Subtle
                    }
                })
                size=ButtonSize::Small
                on_click={
                    let vm = vm.clone();
                    move |_| vm.set_tab("barcodes")
                }
                disabled=Signal::derive(move || !is_edit_mode.get())
            >
                {tab_icon("barcode")}
                "Штрихкоды"
                <Badge
                    appearance=BadgeAppearance::Tint
                    color=Signal::derive({
                        let active_tab = active_tab;
                        move || if active_tab.get() == "barcodes" {
                            BadgeColor::Brand
                        } else {
                            BadgeColor::Informative
                        }
                    })
                    attr:style="margin-left: 6px;"
                >
                    {move || barcodes_count.get().to_string()}
                </Badge>
            </Button>

            // Dealer prices tab (disabled for new records)
            <Button
                appearance=Signal::derive({
                    let active_tab = active_tab;
                    move || if active_tab.get() == "dealer_prices" {
                        ButtonAppearance::Primary
                    } else {
                        ButtonAppearance::Subtle
                    }
                })
                size=ButtonSize::Small
                on_click={
                    let vm = vm.clone();
                    move |_| vm.set_tab("dealer_prices")
                }
                disabled=Signal::derive(move || !is_edit_mode.get())
            >
                {tab_icon("dollar-sign")}
                "Дилерские цены"
                <Badge
                    appearance=BadgeAppearance::Tint
                    color=Signal::derive({
                        let active_tab = active_tab;
                        move || if active_tab.get() == "dealer_prices" {
                            BadgeColor::Brand
                        } else {
                            BadgeColor::Informative
                        }
                    })
                    attr:style="margin-left: 6px;"
                >
                    {move || dealer_prices_count.get().to_string()}
                </Badge>
            </Button>
        </Flex>
    }
}

/// Tab content component - routes to the appropriate tab
#[component]
fn TabContent(vm: NomenclatureDetailsVm) -> impl IntoView {
    let active_tab = vm.active_tab;
    let vm_general = vm.clone();
    let vm_dimensions = vm.clone();
    let vm_barcodes = vm.clone();
    let vm_dealer_prices = vm.clone();

    view! {
        {move || match active_tab.get() {
            "general" => view! {
                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-md); height: 100%; overflow-y: auto; align-items: start; align-content: start;">
                   <GeneralTab vm=vm_general.clone() />
                   <DimensionsTab vm=vm_dimensions.clone() />
                </div>
            }.into_any(),
            "dimensions" => view! {
                <div style="height: 100%; overflow-y: auto;">
                    <DimensionsTab vm=vm_dimensions.clone() />
                </div>
            }.into_any(),
            "barcodes" => view! {
                <div style="height: 100%; overflow-y: auto;">
                    <BarcodesTab vm=vm_barcodes.clone() />
                </div>
            }.into_any(),
            "dealer_prices" => view! {
                <div style="height: 100%; overflow-y: auto;">
                    <DealerPricesTab vm=vm_dealer_prices.clone() />
                </div>
            }.into_any(),
            _ => view! {
                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-lg); height: 100%; overflow-y: auto;">
                    <GeneralTab vm=vm_general.clone() />
                    <DimensionsTab vm=vm_dimensions.clone() />
                </div>
            }.into_any(),
        }}
    }
}
