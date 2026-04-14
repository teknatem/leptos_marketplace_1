//! JSON tab — raw payload viewer

use super::super::view_model::WbSupplyDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn JsonTab(vm: WbSupplyDetailsVm) -> impl IntoView {
    view! {
        <CardAnimated nav_id="a029_wb_supply_details_json">
            <h3 class="details-section__title">"Сырые данные из WB API"</h3>
            {move || {
                if vm.raw_json_loading.get() {
                    return view! {
                        <Flex gap=FlexGap::Small style="align-items: center; padding: var(--spacing-lg);">
                            <Spinner />
                            <span>"Загрузка..."</span>
                        </Flex>
                    }
                    .into_any();
                }

                if let Some(json) = vm.raw_json.get() {
                    view! {
                        <pre style="overflow: auto; max-height: 600px; background: var(--color-surface-secondary); padding: var(--spacing-md); border-radius: var(--radius-sm); font-size: var(--font-size-xs); white-space: pre-wrap; word-break: break-all;">
                            {json}
                        </pre>
                    }
                    .into_any()
                } else {
                    view! {
                        <div style="color: var(--color-text-secondary); padding: var(--spacing-md);">
                            "JSON недоступен (не загружен из хранилища)"
                        </div>
                    }
                    .into_any()
                }
            }}
        </CardAnimated>
    }
}
