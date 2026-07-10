//! Main page component for WB Sales Funnel Daily details (MVVM Standard)
//!
//! Thin wrapper that creates the ViewModel, renders the header,
//! the tab bar, and routes to tab components.

use super::tabs::{GeneralTab, LinesTab};
use super::view_model::WbSalesFunnelDailyDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_DETAIL;
use crate::system::favorites::ui::FavoriteButton;
use leptos::prelude::*;
use thaw::*;

/// Main component for WB Sales Funnel Daily details
#[component]
pub fn WbSalesFunnelDailyDetail(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let tabs = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let vm = WbSalesFunnelDailyDetailsVm::new(tabs);
    let stored_id = StoredValue::new(id.clone());

    vm.load(id);

    let vm_header = vm.clone();
    let vm_tabs = vm.clone();
    let vm_content = vm.clone();

    view! {
        <PageFrame page_id="a036_wb_sales_funnel_daily--detail" category=PAGE_CAT_DETAIL class="page--wide">
            <Header vm=vm_header favorite_target_id=stored_id.get_value() on_close=on_close />

            <TabBar vm=vm_tabs />

            <div class="page__content">
                {move || {
                    if vm.loading.get() {
                        view! {
                            <Flex gap=FlexGap::Small style="align-items:center;justify-content:center;padding:var(--spacing-4xl);">
                                <Spinner />
                                <span>"Загрузка..."</span>
                            </Flex>
                        }.into_any()
                    } else if let Some(err) = vm.error.get() {
                        view! { <div class="alert alert--error">{err}</div> }.into_any()
                    } else if vm.doc.get().is_some() {
                        view! { <TabContent vm=vm_content.clone() /> }.into_any()
                    } else {
                        view! { <div class="alert">"Документ не найден."</div> }.into_any()
                    }
                }}
            </div>
        </PageFrame>
    }
}

// ── Header ────────────────────────────────────────────────────────────────────

#[component]
fn Header(
    vm: WbSalesFunnelDailyDetailsVm,
    favorite_target_id: String,
    on_close: Callback<()>,
) -> impl IntoView {
    let title = vm.header_title();
    let favorite_title = vm.tab_label();
    let tab_key = format!("a036_wb_sales_funnel_daily_details_{}", favorite_target_id);

    view! {
        <div class="page__header">
            <div class="page__header-left">
                <FavoriteButton
                    target_kind="a036_wb_sales_funnel_daily_details".to_string()
                    target_id=favorite_target_id
                    target_title=favorite_title
                    tab_key=tab_key
                />
                <h1 class="page__title">{move || title.get()}</h1>
            </div>
            <div class="page__header-right">
                <Button
                    appearance=ButtonAppearance::Subtle
                    size=ButtonSize::Medium
                    on_click=move |_| on_close.run(())
                >
                    <span class="page-action-button__content">
                        <span class="page-action-button__icon page-action-button__icon--close">{icon("x")}</span>
                        <span class="page-action-button__text">"Закрыть"</span>
                    </span>
                </Button>
            </div>
        </div>
    }
}

// ── Tab bar ───────────────────────────────────────────────────────────────────

#[component]
fn TabBar(vm: WbSalesFunnelDailyDetailsVm) -> impl IntoView {
    let active_tab = vm.active_tab;

    view! {
        <div class="page__tabs">
            <button
                class="page__tab"
                class:page__tab--active=move || active_tab.get() == "general"
                on:click={ let vm = vm.clone(); move |_| vm.set_tab("general") }
            >
                "Общие"
            </button>

            <button
                class="page__tab"
                class:page__tab--active=move || active_tab.get() == "lines"
                on:click={ let vm = vm.clone(); move |_| vm.set_tab("lines") }
            >
                "Позиции"
            </button>
        </div>
    }
}

// ── Tab content ───────────────────────────────────────────────────────────────

#[component]
fn TabContent(vm: WbSalesFunnelDailyDetailsVm) -> impl IntoView {
    let active_tab = vm.active_tab;
    let vm_general = vm.clone();
    let vm_lines = vm.clone();

    view! {
        {move || match active_tab.get() {
            "general" => view! { <GeneralTab vm=vm_general.clone() /> }.into_any(),
            "lines"   => view! { <LinesTab   vm=vm_lines.clone()   /> }.into_any(),
            _         => view! { <GeneralTab vm=vm_general.clone() /> }.into_any(),
        }}
    }
}
