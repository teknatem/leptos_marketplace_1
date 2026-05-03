//! Main page component for WB Supply details

use super::tabs::{GeneralTab, JsonTab, OrdersTab, StickersTab};
use super::view_model::WbSupplyDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn WbSupplyDetails(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let vm = WbSupplyDetailsVm::new();
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let stored_id = StoredValue::new(id.clone());

    vm.load(id);

    Effect::new({
        let vm = vm.clone();
        move || {
            if let Some(supply_data) = vm.supply.get() {
                let tab_key = format!("a029_wb_supply_details_{}", stored_id.get_value());
                let name = supply_data
                    .info
                    .name
                    .as_deref()
                    .unwrap_or(&supply_data.header.supply_id);
                let tab_title = format!("Поставка {}", name);
                tabs_store.update_tab_title(&tab_key, &tab_title);
            }
        }
    });

    Effect::new({
        let vm = vm.clone();
        move || match vm.active_tab.get() {
            "json" if !vm.raw_json_loaded.get() => vm.load_raw_json(),
            "stickers" => {}
            _ => {}
        }
    });

    let vm_header = vm.clone();
    let vm_tabs = vm.clone();
    let vm_content = vm.clone();

    view! {
        <PageFrame page_id="a029_wb_supply_details" category="detail">
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
                    } else if vm.supply.get().is_some() {
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
fn Header(vm: WbSupplyDetailsVm, on_close: Callback<()>) -> impl IntoView {
    let supply_id = vm.supply_id();

    view! {
        <div class="page__header">
            <div class="page__header-left">
                <h1 class="page__title">{move || format!("WB Supply {}", supply_id.get())}</h1>
                <Show when=move || vm.supply.get().is_some()>
                    {move || {
                        let is_done = vm.supply.get().map(|s| s.info.is_done).unwrap_or(false);
                        view! {
                            <Badge
                                appearance=BadgeAppearance::Filled
                                color=if is_done { BadgeColor::Success } else { BadgeColor::Warning }
                            >
                                {if is_done { "Завершена" } else { "Открыта" }}
                            </Badge>
                        }
                    }}
                </Show>
            </div>
            <div class="page__header-right">
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
fn TabBar(vm: WbSupplyDetailsVm) -> impl IntoView {
    let active_tab = vm.active_tab;
    let orders_count = vm.orders_count();

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
                class:page__tab--active=move || active_tab.get() == "orders"
                on:click={
                    let vm = vm.clone();
                    move |_| vm.set_tab("orders")
                }
            >
                {icon("list")}
                {move || format!("Заказы ({})", orders_count.get())}
            </button>

            <button
                class="page__tab"
                class:page__tab--active=move || active_tab.get() == "stickers"
                on:click={
                    let vm = vm.clone();
                    move |_| vm.set_tab("stickers")
                }
            >
                {icon("tag")} "Стикеры"
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
        </div>
    }
}

#[component]
fn TabContent(vm: WbSupplyDetailsVm) -> impl IntoView {
    let active_tab = vm.active_tab;

    view! {
        {move || match active_tab.get() {
            "general" => view! { <GeneralTab vm=vm.clone() /> }.into_any(),
            "orders" => view! { <OrdersTab vm=vm.clone() /> }.into_any(),
            "stickers" => view! { <StickersTab vm=vm.clone() /> }.into_any(),
            "json" => view! { <JsonTab vm=vm.clone() /> }.into_any(),
            _ => view! { <div>"Неизвестная вкладка"</div> }.into_any(),
        }}
    }
}
