//! Main page component for WB Orders details (MVVM Standard)

use super::tabs::{GeneralTab, JsonTab, LineTab, LinksTab, SalesTab};
use super::view_model::WbOrdersDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn WbOrdersDetail(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let vm = WbOrdersDetailsVm::new();
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let stored_id = StoredValue::new(id.clone());

    vm.load(id);

    Effect::new({
        let vm = vm.clone();
        move || {
            if let Some(order_data) = vm.order.get() {
                let tab_key = format!("a015_wb_orders_detail_{}", stored_id.get_value());
                let tab_title = format!("WB Order {}", order_data.header.document_no);
                tabs_store.update_tab_title(&tab_key, &tab_title);
            }
        }
    });

    Effect::new({
        let vm = vm.clone();
        move || match vm.active_tab.get() {
            "json" if !vm.raw_json_loaded.get() => vm.load_raw_json(),
            "links" | "line" if !vm.finance_reports_loaded.get() => vm.load_finance_reports(),
            "sales" if !vm.wb_sales_loaded.get() => vm.load_wb_sales(),
            _ => {}
        }
    });

    let vm_header = vm.clone();
    let vm_tabs = vm.clone();
    let vm_content = vm.clone();

    view! {
        <PageFrame page_id="a015_wb_orders--detail" category="detail">
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

#[component]
fn Header(vm: WbOrdersDetailsVm, on_close: Callback<()>) -> impl IntoView {
    let is_posted = vm.is_posted();
    let document_no = vm.document_no();
    let order = vm.order;

    view! {
        <div class="page__header">
            <div class="page__header-left">
                <h1 class = "page__title">{move || format!("WB Order {}", document_no.get())}</h1>
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
                    "Закрыть"
                </Button>
            </div>
        </div>
    }
}

#[component]
fn PostButtons(vm: WbOrdersDetailsVm) -> impl IntoView {
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

#[component]
fn TabBar(vm: WbOrdersDetailsVm) -> impl IntoView {
    let active_tab = vm.active_tab;
    let finance_reports_count = vm.finance_reports_count();
    let wb_sales_count = vm.wb_sales_count();

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
                class:page__tab--active=move || active_tab.get() == "sales"
                on:click=move |_| vm.set_tab("sales")
            >
                {icon("shopping-cart")} "Sales"
                <Badge
                    appearance=BadgeAppearance::Tint
                    color=Signal::derive({
                        let active_tab = active_tab;
                        move || if active_tab.get() == "sales" {
                            BadgeColor::Brand
                        } else {
                            BadgeColor::Informative
                        }
                    })
                    attr:style="margin-left: 6px;"
                >
                    {move || wb_sales_count.get().to_string()}
                </Badge>
            </button>
        </div>
    }
}

#[component]
fn TabContent(vm: WbOrdersDetailsVm) -> impl IntoView {
    let active_tab = vm.active_tab;
    let vm_general = vm.clone();
    let vm_line = vm.clone();
    let vm_json = vm.clone();
    let vm_links = vm.clone();
    let vm_sales = vm.clone();

    view! {
        {move || match active_tab.get() {
            "general" => view! { <GeneralTab vm=vm_general.clone() /> }.into_any(),
            "line" => view! { <LineTab vm=vm_line.clone() /> }.into_any(),
            "json" => view! { <JsonTab vm=vm_json.clone() /> }.into_any(),
            "links" => view! { <LinksTab vm=vm_links.clone() /> }.into_any(),
            "sales" => view! { <SalesTab vm=vm_sales.clone() /> }.into_any(),
            _ => view! { <GeneralTab vm=vm_general.clone() /> }.into_any(),
        }}
    }
}
