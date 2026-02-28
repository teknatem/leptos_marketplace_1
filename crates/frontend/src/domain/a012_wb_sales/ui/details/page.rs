//! Main page component for WB Sales details (MVVM Standard)
//!
//! Thin wrapper that:
//! - Creates ViewModel
//! - Renders header with action buttons
//! - Renders tab bar
//! - Routes to tab components
//! - Handles lazy loading for nested data

use super::tabs::{GeneralTab, JsonTab, LineTab, LinksTab, PlanFactTab, ProjectionsTab};
use super::view_model::WbSalesDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use leptos::prelude::*;
use thaw::*;

/// Main component for WB Sales details
#[component]
pub fn WbSalesDetail(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let vm = WbSalesDetailsVm::new();
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let stored_id = StoredValue::new(id.clone());

    vm.load(id);

    Effect::new({
        let vm = vm.clone();
        move || {
            if let Some(sale_data) = vm.sale.get() {
                let tab_key = format!("a012_wb_sales_detail_{}", stored_id.get_value());
                let sale_id = sale_data
                    .header
                    .sale_id
                    .clone()
                    .unwrap_or_else(|| "—".to_string());
                let tab_title = format!("WB Sales {}", sale_id);
                tabs_store.update_tab_title(&tab_key, &tab_title);
            }
        }
    });

    Effect::new({
        let vm = vm.clone();
        move || match vm.active_tab.get() {
            "json" if !vm.raw_json_loaded.get() => vm.load_raw_json(),
            "projections" if !vm.projections_loaded.get() => vm.load_projections(),
            "links" | "line" if !vm.finance_reports_loaded.get() => vm.load_finance_reports(),
            _ => {}
        }
    });

    let vm_header = vm.clone();
    let vm_tabs = vm.clone();
    let vm_content = vm.clone();

    view! {
        <PageFrame page_id="a012_wb_sales--detail" category="detail">
            <Header vm=vm_header on_close=on_close />

            <TabBar vm=vm_tabs />

            <div class="page__content">
                {move || {
                    if vm.loading.get() {
                        view! {
                            <Flex gap=FlexGap::Small style="align-items: center; padding: var(--spacing-4xl); justify-content: center;">
                                <Spinner />
                                <span>"Загрузка..."</span>
                            </Flex>
                        }.into_any()
                    } else if let Some(err) = vm.error.get() {
                        view! {
                            <div style="padding: var(--spacing-lg); background: var(--color-error-50); border: 1px solid var(--color-error-100); border-radius: var(--radius-sm); color: var(--color-error); margin: var(--spacing-lg);">
                                <strong>"Ошибка: "</strong>{err}
                            </div>
                        }.into_any()
                    } else if vm.sale.get().is_some() {
                        view! {
                            <TabContent vm=vm_content.clone() />
                        }.into_any()
                    } else {
                        view! { <div>"No data"</div> }.into_any()
                    }
                }}
            </div>
        </PageFrame>
    }
}

// ── Header ────────────────────────────────────────────────────────────────────

#[component]
fn Header(vm: WbSalesDetailsVm, on_close: Callback<()>) -> impl IntoView {
    let is_posted = vm.is_posted();
    let sale_id = vm.sale_id();
    let sale = vm.sale;

    view! {
        <div class="page__header">
            <div class="page__header-left">
                <h1 class="page__title">{move || format!("WB Sales {}", sale_id.get())}</h1>
                <Show when=move || sale.get().is_some()>
                    {move || {
                        let posted = is_posted.get();
                        view! {
                            <Badge
                                appearance=BadgeAppearance::Filled
                                color=if posted { BadgeColor::Success } else { BadgeColor::Warning }
                            >
                                {if posted { "Проведен" } else { "Не проведен" }}
                            </Badge>
                        }
                    }}
                </Show>
            </div>
            <div class="page__header-right">
                <PostButtons vm=vm.clone() />
                <Button
                    appearance=ButtonAppearance::Secondary
                    size=ButtonSize::Medium
                    on_click=move |_| on_close.run(())
                >
                    {icon("x")} "Закрыть"
                </Button>
            </div>
        </div>
    }
}

// ── Post / Unpost ─────────────────────────────────────────────────────────────

#[component]
fn PostButtons(vm: WbSalesDetailsVm) -> impl IntoView {
    let is_posted = vm.is_posted();
    let posting = vm.posting;
    let sale = vm.sale;

    let on_post = {
        let vm = vm.clone();
        Callback::new(move |_: ()| vm.post())
    };
    let on_unpost = {
        let vm = vm;
        Callback::new(move |_: ()| vm.unpost())
    };

    view! {
        <Show when=move || sale.get().is_some()>
            <Show when=move || !is_posted.get()>
                <Button
                    appearance=ButtonAppearance::Primary
                    size=ButtonSize::Medium
                    on_click=move |_| on_post.run(())
                    disabled=Signal::derive(move || posting.get())
                >
                    {move || if posting.get() { "Проведение..." } else { "✓ Post" }}
                </Button>
            </Show>
            <Show when=move || is_posted.get()>
                <Button
                    appearance=ButtonAppearance::Secondary
                    size=ButtonSize::Medium
                    on_click=move |_| on_unpost.run(())
                    disabled=Signal::derive(move || posting.get())
                >
                    {move || if posting.get() { "Отмена..." } else { "✗ Unpost" }}
                </Button>
            </Show>
        </Show>
    }
}

// ── Tab bar ───────────────────────────────────────────────────────────────────

#[component]
fn TabBar(vm: WbSalesDetailsVm) -> impl IntoView {
    let active_tab = vm.active_tab;
    let projections_count = vm.projections_count();
    let finance_reports_count = vm.finance_reports_count();

    view! {
        <div class="page__tabs">
            <button
                class="page__tab"
                class:page__tab--active=move || active_tab.get() == "general"
                on:click={
                    let vm = vm.clone();
                    move |_| vm.set_tab("general")
                }
            >
                {icon("file-text")} "Общие"
            </button>

            <button
                class="page__tab"
                class:page__tab--active=move || active_tab.get() == "planfact"
                on:click={
                    let vm = vm.clone();
                    move |_| vm.set_tab("planfact")
                }
            >
                {icon("trending-up")} "План/Факт"
            </button>

            <button
                class="page__tab"
                class:page__tab--active=move || active_tab.get() == "line"
                on:click={
                    let vm = vm.clone();
                    move |_| vm.set_tab("line")
                }
            >
                {icon("list")} "Подробно"
            </button>

            <button
                class="page__tab"
                class:page__tab--active=move || active_tab.get() == "json"
                on:click={
                    let vm = vm.clone();
                    move |_| vm.set_tab("json")
                }
            >
                {icon("code")} "JSON"
            </button>

            <button
                class="page__tab"
                class:page__tab--active=move || active_tab.get() == "links"
                on:click={
                    let vm = vm.clone();
                    move |_| vm.set_tab("links")
                }
            >
                {icon("link")} "Связи"
                <Badge
                    appearance=BadgeAppearance::Tint
                    color=Signal::derive({
                        let active_tab = active_tab;
                        move || if active_tab.get() == "links" {
                            BadgeColor::Brand
                        } else {
                            BadgeColor::Informative
                        }
                    })
                    attr:style="margin-left: 6px;"
                >
                    {move || finance_reports_count.get().to_string()}
                </Badge>
            </button>

            <button
                class="page__tab"
                class:page__tab--active=move || active_tab.get() == "projections"
                on:click=move |_| vm.set_tab("projections")
            >
                {icon("layers")} "Проекции"
                <Badge
                    appearance=BadgeAppearance::Tint
                    color=Signal::derive({
                        let active_tab = active_tab;
                        move || if active_tab.get() == "projections" {
                            BadgeColor::Brand
                        } else {
                            BadgeColor::Informative
                        }
                    })
                    attr:style="margin-left: 6px;"
                >
                    {move || projections_count.get().to_string()}
                </Badge>
            </button>
        </div>
    }
}

// ── Tab content ───────────────────────────────────────────────────────────────

#[component]
fn TabContent(vm: WbSalesDetailsVm) -> impl IntoView {
    let active_tab = vm.active_tab;
    let vm_general = vm.clone();
    let vm_planfact = vm.clone();
    let vm_line = vm.clone();
    let vm_json = vm.clone();
    let vm_links = vm.clone();
    let vm_projections = vm.clone();

    view! {
        {move || match active_tab.get() {
            "general"     => view! { <GeneralTab     vm=vm_general.clone()     /> }.into_any(),
            "planfact"    => view! { <PlanFactTab    vm=vm_planfact.clone()    /> }.into_any(),
            "line"        => view! { <LineTab       vm=vm_line.clone()        /> }.into_any(),
            "json"        => view! { <JsonTab       vm=vm_json.clone()        /> }.into_any(),
            "links"       => view! { <LinksTab      vm=vm_links.clone()       /> }.into_any(),
            "projections" => view! { <ProjectionsTab vm=vm_projections.clone() /> }.into_any(),
            _             => view! { <GeneralTab     vm=vm_general.clone()     /> }.into_any(),
        }}
    }
}
