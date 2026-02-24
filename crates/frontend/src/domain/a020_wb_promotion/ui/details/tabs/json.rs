use crate::domain::a020_wb_promotion::ui::details::view_model::WbPromotionDetailsVm;
use crate::shared::json_viewer::widget::JsonViewer;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn JsonTab(vm: WbPromotionDetailsVm) -> impl IntoView {
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
                    <JsonViewer
                        json_content=json
                        title="Raw JSON from WB".to_string()
                    />
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
