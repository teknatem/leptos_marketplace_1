//! JSON tab - raw JSON from WB API

use super::super::view_model::WbSalesDetailsVm;
use leptos::prelude::*;
use thaw::*;

/// JSON tab component - displays raw JSON from WB API
#[component]
pub fn JsonTab(vm: WbSalesDetailsVm) -> impl IntoView {
    view! {
        {move || {
            if vm.raw_json_loading.get() {
                return view! {
                    <Card>
                        <Flex gap=FlexGap::Small style="align-items: center; justify-content: center; padding: var(--spacing-xl);">
                            <Spinner />
                            <span>"Загрузка JSON..."</span>
                        </Flex>
                    </Card>
                }.into_any();
            }

            if let Some(json) = vm.raw_json.get() {
                view! {
                    <Card>
                        <h4 class="details-section__title">"Raw JSON from WB"</h4>
                        <pre style="margin: 0; max-height: 70vh; overflow: auto; font-size: var(--font-size-sm); background: var(--color-bg-secondary); padding: var(--spacing-md); border-radius: var(--radius-sm);">
                            {json}
                        </pre>
                    </Card>
                }.into_any()
            } else {
                view! {
                    <Card>
                        <h4 class="details-section__title">"Raw JSON from WB"</h4>
                        <div style="color: var(--color-text-secondary);">
                            "JSON данные не загружены"
                        </div>
                    </Card>
                }.into_any()
            }
        }}
    }
}
