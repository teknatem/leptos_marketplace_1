use super::tabs::{GeneralTab, JsonTab, NomenclaturesTab};
use super::view_model::WbPromotionDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn WbPromotionDetail(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let vm = WbPromotionDetailsVm::new();
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let stored_id = StoredValue::new(id.clone());

    vm.load(id);

    Effect::new({
        let vm = vm.clone();
        move || {
            if let Some(promo) = vm.promotion.get() {
                let tab_key = format!("a020_wb_promotion_detail_{}", stored_id.get_value());
                let tab_title = format!("WB Акция: {}", promo.data.name);
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
        <PageFrame page_id="a020_wb_promotion--detail" category="detail">
            <Header vm=vm_header on_close=on_close />
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
                    } else if vm.promotion.get().is_some() {
                        view! {
                            <div class="page__tab-content">
                                <TabBar vm=vm_tabs.clone() />
                                <TabContent vm=vm_content.clone() />
                            </div>
                        }.into_any()
                    } else {
                    view! { <div>"Нет данных"</div> }.into_any()
                }
            }}
            </div>
        </PageFrame>
    }
}

#[component]
fn Header(vm: WbPromotionDetailsVm, on_close: Callback<()>) -> impl IntoView {
    let name = vm.promotion_name();

    view! {
        <div class="page__header">
            <div class="page__header-left">
                <h2>{move || format!("WB Акция: {}", name.get())}</h2>
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
fn TabBar(vm: WbPromotionDetailsVm) -> impl IntoView {
    let active_tab = vm.active_tab;
    let nomenclatures_count = {
        let promotion = vm.promotion;
        Signal::derive(move || {
            promotion
                .get()
                .map(|p| p.nomenclatures.len())
                .unwrap_or(0)
        })
    };
    let tab_icon = |name: &str| view! { <span class="tab-icon">{icon(name)}</span> };

    view! {
        <Flex
            gap=FlexGap::Small
            align=FlexAlign::Center
            style="margin-bottom: var(--spacing-md); padding: var(--spacing-sm); background: var(--color-bg-secondary); border-radius: var(--radius-lg); border: 1px solid var(--color-border);"
        >
            <Button
                appearance=Signal::derive({
                    let active_tab = active_tab;
                    move || if active_tab.get() == "general" { ButtonAppearance::Primary } else { ButtonAppearance::Subtle }
                })
                size=ButtonSize::Small
                on_click={
                    let vm = vm.clone();
                    move |_| vm.set_tab("general")
                }
            >
                {tab_icon("file-text")}
                "Общие"
            </Button>

            <Button
                appearance=Signal::derive({
                    let active_tab = active_tab;
                    move || if active_tab.get() == "nomenclatures" { ButtonAppearance::Primary } else { ButtonAppearance::Subtle }
                })
                size=ButtonSize::Small
                on_click={
                    let vm = vm.clone();
                    move |_| vm.set_tab("nomenclatures")
                }
            >
                {tab_icon("package")}
                "Товары"
                <Badge
                    appearance=BadgeAppearance::Tint
                    color=Signal::derive({
                        let active_tab = active_tab;
                        move || if active_tab.get() == "nomenclatures" { BadgeColor::Brand } else { BadgeColor::Informative }
                    })
                    attr:style="margin-left: 6px;"
                >
                    {move || nomenclatures_count.get().to_string()}
                </Badge>
            </Button>

            <Button
                appearance=Signal::derive({
                    let active_tab = active_tab;
                    move || if active_tab.get() == "json" { ButtonAppearance::Primary } else { ButtonAppearance::Subtle }
                })
                size=ButtonSize::Small
                on_click={
                    let vm = vm.clone();
                    move |_| vm.set_tab("json")
                }
            >
                {tab_icon("code")}
                "JSON"
            </Button>
        </Flex>
    }
}

#[component]
fn TabContent(vm: WbPromotionDetailsVm) -> impl IntoView {
    let active_tab = vm.active_tab;
    let vm_general = vm.clone();
    let vm_nomenclatures = vm.clone();
    let vm_json = vm.clone();

    view! {
        {move || match active_tab.get() {
            "general" => view! { <GeneralTab vm=vm_general.clone() /> }.into_any(),
            "nomenclatures" => view! { <NomenclaturesTab vm=vm_nomenclatures.clone() /> }.into_any(),
            "json" => view! { <JsonTab vm=vm_json.clone() /> }.into_any(),
            _ => view! { <GeneralTab vm=vm_general.clone() /> }.into_any(),
        }}
    }
}
