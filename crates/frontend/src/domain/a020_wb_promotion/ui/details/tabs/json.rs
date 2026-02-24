use crate::domain::a020_wb_promotion::ui::details::view_model::WbPromotionDetailsVm;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn JsonTab(vm: WbPromotionDetailsVm) -> impl IntoView {
    view! {
        <div style="padding: var(--spacing-lg);">
            {move || {
                if vm.raw_json_loading.get() {
                    return view! {
                        <Flex gap=FlexGap::Small style="align-items: center;">
                            <Spinner />
                            <span>"Загрузка JSON..."</span>
                        </Flex>
                    }.into_any();
                }

                if let Some(json) = vm.raw_json.get() {
                    view! {
                        <pre style="font-size: 11px; overflow: auto; max-height: 600px; background: var(--colorNeutralBackground2); padding: var(--spacing-md); border-radius: var(--borderRadiusMedium); border: 1px solid var(--colorNeutralStroke1); white-space: pre-wrap; word-break: break-all;">
                            {json}
                        </pre>
                    }.into_any()
                } else {
                    view! {
                        <div style="color: var(--colorNeutralForeground3); padding: var(--spacing-md);">
                            "Нет данных"
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}
