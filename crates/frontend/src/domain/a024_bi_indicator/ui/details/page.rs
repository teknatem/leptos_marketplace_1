//! Main page component for BiIndicator details (EditDetails MVVM Standard)

use super::tabs::{DataSpecTab, DrillSpecTab, GeneralTab, MetaTab, ParamsTab, ViewSpecTab};
use super::view_model::BiIndicatorDetailsVm;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn BiIndicatorDetails(
    id: Option<String>,
    #[prop(into)] on_saved: Callback<()>,
    #[prop(into)] on_cancel: Callback<()>,
) -> impl IntoView {
    let vm = BiIndicatorDetailsVm::new();

    if let Some(existing_id) = id {
        vm.load(existing_id);
    }

    let vm_header = vm.clone();
    let vm_tabs = vm.clone();
    let vm_content = vm.clone();

    view! {
        <PageFrame page_id="a024_bi_indicator--detail" category="detail">
            <Header vm=vm_header on_saved=on_saved on_cancel=on_cancel />
            <TabBar vm=vm_tabs />
            <div class="page__content">
                <ErrorDisplay vm=vm.clone() />
                <TabContent vm=vm_content />
            </div>
        </PageFrame>
    }
}

// ── Header ────────────────────────────────────────────────────────────────────

#[component]
fn Header(
    vm: BiIndicatorDetailsVm,
    on_saved: Callback<()>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let is_edit_mode = vm.is_edit_mode();
    let is_save_disabled = vm.is_save_disabled();

    let handle_save = {
        let vm = vm.clone();
        move |_| vm.save(on_saved)
    };

    view! {
        <div class="page__header">
            <div class="page__header-left">
                <h1 class="page__title">
                    {move || if is_edit_mode.get() {
                        "Редактирование индикатора"
                    } else {
                        "Новый BI индикатор"
                    }}
                </h1>
            </div>
            <div class="page__header-right">
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

// ── Tab bar ───────────────────────────────────────────────────────────────────

#[component]
fn TabBar(vm: BiIndicatorDetailsVm) -> impl IntoView {
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
                class:page__tab--active=move || active_tab.get() == "data_spec"
                on:click={
                    let vm = vm.clone();
                    move |_| vm.set_tab("data_spec")
                }
            >
                {icon("database")} " DataSpec"
            </button>

            <button
                class="page__tab"
                class:page__tab--active=move || active_tab.get() == "params"
                on:click={
                    let vm = vm.clone();
                    move |_| vm.set_tab("params")
                }
            >
                {icon("sliders")} " Параметры"
            </button>

            <button
                class="page__tab"
                class:page__tab--active=move || active_tab.get() == "view_spec"
                on:click={
                    let vm = vm.clone();
                    move |_| vm.set_tab("view_spec")
                }
            >
                {icon("layout")} " ViewSpec"
            </button>

            <button
                class="page__tab"
                class:page__tab--active=move || active_tab.get() == "drill_spec"
                on:click={
                    let vm = vm.clone();
                    move |_| vm.set_tab("drill_spec")
                }
            >
                {icon("chevrons-down")} " DrillSpec"
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

// ── Tab content ───────────────────────────────────────────────────────────────

#[component]
fn TabContent(vm: BiIndicatorDetailsVm) -> impl IntoView {
    let active_tab = vm.active_tab;

    view! {
        {move || match active_tab.get() {
            "data_spec" => view! {
                <DataSpecTab vm=vm.clone() />
            }.into_any(),
            "params" => view! {
                <ParamsTab vm=vm.clone() />
            }.into_any(),
            "view_spec" => view! {
                <ViewSpecTab vm=vm.clone() />
            }.into_any(),
            "drill_spec" => view! {
                <DrillSpecTab vm=vm.clone() />
            }.into_any(),
            "meta" => view! {
                <MetaTab vm=vm.clone() />
            }.into_any(),
            _ => view! {
                <GeneralTab vm=vm.clone() />
            }.into_any(),
        }}
    }
}

// ── Error display ─────────────────────────────────────────────────────────────

#[component]
fn ErrorDisplay(vm: BiIndicatorDetailsVm) -> impl IntoView {
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
