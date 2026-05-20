use super::tabs::{GeneralTab, JsonTab};
use super::view_model::WbReturnsClaimsDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn WbReturnsClaimsDetails(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let vm = WbReturnsClaimsDetailsVm::new();
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let stored_id = StoredValue::new(id.clone());

    vm.load(id);

    Effect::new({
        let vm = vm.clone();
        move || {
            if let Some(data) = vm.item.get() {
                let tab_key = format!("a032_wb_returns_claims_details_{}", stored_id.get_value());
                let short: String = data.claim_id.chars().take(16).collect();
                let tab_title = format!("Заявка {}", short);
                tabs_store.update_tab_title(&tab_key, &tab_title);
            }
        }
    });

    Effect::new({
        let vm = vm.clone();
        move || {
            if vm.active_tab.get() == "json" && !vm.raw_json_loaded.get() {
                vm.load_raw_json();
            }
        }
    });

    let vm_header = vm.clone();
    let vm_tabs = vm.clone();
    let vm_content = vm.clone();

    view! {
        <PageFrame page_id="a032_wb_returns_claims_details" category="detail">
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
                    } else if vm.item.get().is_some() {
                        view! { <TabContent vm=vm_content.clone() /> }.into_any()
                    } else {
                        view! { <div>"Нет данных"</div> }.into_any()
                    }
                }}
            </div>
        </PageFrame>
    }
}

#[component]
fn Header(vm: WbReturnsClaimsDetailsVm, on_close: Callback<()>) -> impl IntoView {
    let claim_id = vm.claim_id();
    let item = vm.item;

    view! {
        <div class="page__header">
            <div class="page__header-left">
                <h1 class="page__title">
                    {move || {
                        let cid = claim_id.get();
                        if cid.is_empty() {
                            "Заявка на возврат WB".to_string()
                        } else {
                            let short: String = cid.chars().take(16).collect();
                            format!("Заявка {}", short)
                        }
                    }}
                </h1>
                <Show when=move || item.get().is_some()>
                    {move || {
                        let status = item.get().and_then(|d| d.status);
                        view! {
                            <Badge
                                appearance=BadgeAppearance::Filled
                                color=if status == Some(3) {
                                    BadgeColor::Success
                                } else if status == Some(4) {
                                    BadgeColor::Danger
                                } else {
                                    BadgeColor::Warning
                                }
                            >
                                {match status {
                                    Some(1) => "Открыта",
                                    Some(2) => "На рассмотрении",
                                    Some(3) => "Одобрена",
                                    Some(4) => "Отклонена",
                                    Some(5) => "Закрыта",
                                    _ => "Неизвестен",
                                }}
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
fn TabBar(vm: WbReturnsClaimsDetailsVm) -> impl IntoView {
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
                {icon("file-text")} "Общие"
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
fn TabContent(vm: WbReturnsClaimsDetailsVm) -> impl IntoView {
    let active_tab = vm.active_tab;
    let vm_general = vm.clone();
    let vm_json = vm.clone();

    view! {
        {move || match active_tab.get() {
            "general" => view! { <GeneralTab vm=vm_general.clone() /> }.into_any(),
            "json" => view! { <JsonTab vm=vm_json.clone() /> }.into_any(),
            _ => view! { <GeneralTab vm=vm_general.clone() /> }.into_any(),
        }}
    }
}
