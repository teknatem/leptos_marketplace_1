//! Main page component for YM Order details (MVVM Standard)

use super::tabs::{CampaignTab, GeneralTab, JsonTab, LinesTab, LinksTab, ProjectionsTab};
use super::view_model::YmOrderDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn YmOrderDetail(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let vm = YmOrderDetailsVm::new();
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let stored_id = StoredValue::new(id.clone());

    vm.load(id);

    Effect::new({
        let vm = vm.clone();
        move || {
            if let Some(order_data) = vm.order.get() {
                let tab_key = format!("a013_ym_order_detail_{}", stored_id.get_value());
                let tab_title = format!("YM Order {}", order_data.header.document_no);
                tabs_store.update_tab_title(&tab_key, &tab_title);
            }
        }
    });

    Effect::new({
        let vm = vm.clone();
        move || match vm.active_tab.get() {
            "json" if !vm.raw_json_loaded.get() => vm.load_raw_json(),
            "projections" if !vm.projections_loaded.get() => vm.load_projections(),
            "links" if !vm.payment_reports_loaded.get() => vm.load_payment_reports(),
            _ => {}
        }
    });

    let vm_header = vm.clone();
    let vm_tabs = vm.clone();
    let vm_content = vm.clone();

    view! {
        <PageFrame page_id="a013_ym_order--detail" category="detail">
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
                        }
                        .into_any()
                    } else if let Some(err) = vm.error.get() {
                        view! {
                            <div style="padding: var(--spacing-lg); background: var(--color-error-50); border: 1px solid var(--color-error-100); border-radius: var(--radius-sm); color: var(--color-error); margin: var(--spacing-lg);">
                                <strong>"Ошибка: "</strong>{err}
                            </div>
                        }
                        .into_any()
                    } else if vm.order.get().is_some() {
                        view! {
                            <TabContent vm=vm_content.clone() />
                        }
                        .into_any()
                    } else {
                        view! { <div>"Нет данных"</div> }.into_any()
                    }
                }}
            </div>
        </PageFrame>
    }
}

// ── Header ────────────────────────────────────────────────────────────────────

#[component]
fn Header(vm: YmOrderDetailsVm, on_close: Callback<()>) -> impl IntoView {
    let is_posted = vm.is_posted();
    let document_no = vm.document_no();
    let order = vm.order;

    view! {
        <div class="page__header">
            <div class="page__header-left">
                <h1 class="page__title">{move || format!("YM Order {}", document_no.get())}</h1>
                <Show when=move || order.get().is_some()>
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
fn PostButtons(vm: YmOrderDetailsVm) -> impl IntoView {
    let is_posted = vm.is_posted();
    let posting = vm.posting;
    let order = vm.order;

    let on_post = {
        let vm = vm.clone();
        Callback::new(move |_: ()| vm.post())
    };
    let on_unpost = {
        let vm = vm;
        Callback::new(move |_: ()| vm.unpost())
    };

    view! {
        <Show when=move || order.get().is_some()>
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
fn TabBar(vm: YmOrderDetailsVm) -> impl IntoView {
    let active_tab = vm.active_tab;
    let projections_count = vm.projections_count();
    let payment_reports_count = vm.payment_reports_count();

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
                class:page__tab--active=move || active_tab.get() == "lines"
                on:click={
                    let vm = vm.clone();
                    move |_| vm.set_tab("lines")
                }
            >
                {icon("list")} "Строки"
            </button>

            <button
                class="page__tab"
                class:page__tab--active=move || active_tab.get() == "campaign"
                on:click={
                    let vm = vm.clone();
                    move |_| vm.set_tab("campaign")
                }
            >
                {icon("building")} "Кампания"
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
                    {move || payment_reports_count.get().to_string()}
                </Badge>
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
fn TabContent(vm: YmOrderDetailsVm) -> impl IntoView {
    let active_tab = vm.active_tab;
    let vm_general = vm.clone();
    let vm_lines = vm.clone();
    let vm_campaign = vm.clone();
    let vm_links = vm.clone();
    let vm_json = vm.clone();
    let vm_projections = vm.clone();

    view! {
        {move || match active_tab.get() {
            "general"     => view! { <GeneralTab     vm=vm_general.clone()     /> }.into_any(),
            "lines"       => view! { <LinesTab       vm=vm_lines.clone()       /> }.into_any(),
            "campaign"    => view! { <CampaignTab    vm=vm_campaign.clone()    /> }.into_any(),
            "links"       => view! { <LinksTab       vm=vm_links.clone()       /> }.into_any(),
            "json"        => view! { <JsonTab        vm=vm_json.clone()        /> }.into_any(),
            "projections" => view! { <ProjectionsTab vm=vm_projections.clone() /> }.into_any(),
            _             => view! { <GeneralTab     vm=vm_general.clone()     /> }.into_any(),
        }}
    }
}
