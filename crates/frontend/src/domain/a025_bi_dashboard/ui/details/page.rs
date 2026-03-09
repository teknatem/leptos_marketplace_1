//! BiDashboard details page (EditDetails MVVM Standard)

use super::tabs::{FiltersTab, GeneralTab, LayoutTab, MetaTab};
use super::view_model::BiDashboardDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn BiDashboardDetails(
    id: Option<String>,
    #[prop(into)] on_saved: Callback<()>,
    #[prop(into)] on_cancel: Callback<()>,
) -> impl IntoView {
    let vm = BiDashboardDetailsVm::new();

    if let Some(existing_id) = id {
        vm.load(existing_id);
    }

    let vm_header = vm.clone();
    let vm_tabs = vm.clone();
    let vm_content = vm.clone();

    view! {
        <PageFrame page_id="a025_bi_dashboard_details" category="detail">
            <Header vm=vm_header on_saved=on_saved on_cancel=on_cancel />
            <TabBar vm=vm_tabs />
            <div class="page__content">
                <ErrorDisplay vm=vm.clone() />
                <SuccessDisplay vm=vm.clone() />
                <TabContent vm=vm_content />
            </div>
        </PageFrame>
    }
}

// ── Header ─────────────────────────────────────────────────────────────────

#[component]
fn Header(
    vm: BiDashboardDetailsVm,
    on_saved: Callback<()>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let tabs_ctx = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let is_edit_mode = vm.is_edit_mode();
    let is_save_disabled = vm.is_save_disabled();
    let can_open_view = Signal::derive({
        let vm = vm.clone();
        move || vm.id.get().is_some()
    });

    let handle_save = {
        let vm = vm.clone();
        move |_| vm.save(on_saved)
    };

    let handle_open_view = {
        let vm = vm.clone();
        move |_| {
            if let Some(id) = vm.id.get_untracked() {
                let code = vm.code.get_untracked();
                let key = format!("a025_bi_dashboard_view_{}", id);
                let title = if code.trim().is_empty() {
                    "Просмотр BI Dashboard".to_string()
                } else {
                    format!("Просмотр · {}", code)
                };
                tabs_ctx.open_tab(&key, &title);
            }
        }
    };

    view! {
        <div class="page__header">
            <div class="page__header-left">
                <h1 class="page__title">
                    {move || if is_edit_mode.get() {
                        "Редактирование дашборда"
                    } else {
                        "Новый BI дашборд"
                    }}
                </h1>
            </div>
            <div class="page__header-right">
                <Button
                    appearance=ButtonAppearance::Secondary
                    on_click=handle_open_view
                    disabled=move || !can_open_view.get()
                >
                    {icon("eye")} " Просмотр"
                </Button>
                <Button
                    appearance=ButtonAppearance::Primary
                    on_click=handle_save
                    disabled=is_save_disabled
                >
                    {icon("save")} " Сохранить"
                </Button>
                <Button
                    appearance=ButtonAppearance::Secondary
                    on_click=move |_| on_cancel.run(())
                >
                    {icon("x")} " Закрыть"
                </Button>
            </div>
        </div>
    }
}

// ── Tab bar ────────────────────────────────────────────────────────────────

#[component]
fn TabBar(vm: BiDashboardDetailsVm) -> impl IntoView {
    let active_tab = vm.active_tab;

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
                {icon("file-text")} " Основная"
            </button>

            <button
                class="page__tab"
                class:page__tab--active=move || active_tab.get() == "layout"
                on:click={
                    let vm = vm.clone();
                    move |_| vm.set_tab("layout")
                }
            >
                {icon("layout")} " Структура"
            </button>

            <button
                class="page__tab"
                class:page__tab--active=move || active_tab.get() == "filters"
                on:click={
                    let vm = vm.clone();
                    move |_| vm.set_tab("filters")
                }
            >
                {icon("filter")} " Фильтры"
            </button>

            <button
                class="page__tab"
                class:page__tab--active=move || active_tab.get() == "meta"
                on:click=move |_| vm.set_tab("meta")
            >
                {icon("info")} " Мета"
            </button>
        </div>
    }
}

// ── Tab content ────────────────────────────────────────────────────────────

#[component]
fn TabContent(vm: BiDashboardDetailsVm) -> impl IntoView {
    let active_tab = vm.active_tab;

    view! {
        {move || match active_tab.get() {
            "layout" => view! { <LayoutTab vm=vm.clone() /> }.into_any(),
            "filters" => view! { <FiltersTab vm=vm.clone() /> }.into_any(),
            "meta" => view! { <MetaTab vm=vm.clone() /> }.into_any(),
            _ => view! { <GeneralTab vm=vm.clone() /> }.into_any(),
        }}
    }
}

// ── Error/Success display ──────────────────────────────────────────────────

#[component]
fn ErrorDisplay(vm: BiDashboardDetailsVm) -> impl IntoView {
    let error = vm.error;
    view! {
        {move || error.get().map(|e| view! {
            <div
                class="warning-box"
                style="background: var(--color-error-50); border-color: var(--color-error-100); margin-bottom: var(--spacing-md);"
            >
                <span class="warning-box__icon" style="color: var(--color-error);">"⚠"</span>
                <span class="warning-box__text" style="color: var(--color-error);">{e}</span>
            </div>
        })}
    }
}

#[component]
fn SuccessDisplay(vm: BiDashboardDetailsVm) -> impl IntoView {
    let success = vm.success;
    view! {
        {move || success.get().map(|s| view! {
            <div
                class="warning-box"
                style="background: var(--color-success-50); border-color: var(--color-success-100); margin-bottom: var(--spacing-md);"
            >
                <span class="warning-box__icon" style="color: var(--color-success);">"✓"</span>
                <span class="warning-box__text" style="color: var(--color-success);">{s}</span>
            </div>
        })}
    }
}
