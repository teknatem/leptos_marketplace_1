//! JSON tab - raw JSON from WB API

use super::super::view_model::WbSalesDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::json_viewer::widget::JsonViewer;
use leptos::prelude::*;
use thaw::*;

/// JSON tab component - displays raw JSON from WB API
#[component]
pub fn JsonTab(vm: WbSalesDetailsVm) -> impl IntoView {
    view! {
        {move || {
            if vm.raw_json_loading.get() {
                return view! {
                    <CardAnimated delay_ms=0 nav_id="a012_wb_sales_details_json_loading">
                        <Flex gap=FlexGap::Small style="align-items: center; justify-content: center; padding: var(--spacing-xl);">
                            <Spinner />
                            <span>"Загрузка JSON..."</span>
                        </Flex>
                    </CardAnimated>
                }.into_any();
            }

            if let Some(json) = vm.raw_json.get() {
                view! {
                    <CardAnimated
                        delay_ms=0
                        nav_id="a012_wb_sales_details_json_main"
                        style="padding: var(--spacing-sm);"
                    >
                        <JsonViewer
                            json_content=json
                            title="Raw JSON from WB".to_string()
                        />
                    </CardAnimated>
                }.into_any()
            } else {
                view! {
                    <CardAnimated delay_ms=0 nav_id="a012_wb_sales_details_json_empty">
                        <h4 class="details-section__title">"Raw JSON from WB"</h4>
                        <div style="color: var(--color-text-secondary);">
                            "JSON данные не загружены"
                        </div>
                    </CardAnimated>
                }.into_any()
            }
        }}
    }
}
