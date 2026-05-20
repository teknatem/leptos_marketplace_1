use crate::domain::a032_wb_returns_claims::ui::details::view_model::WbReturnsClaimsDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::json_viewer::widget::JsonViewer;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn JsonTab(vm: WbReturnsClaimsDetailsVm) -> impl IntoView {
    view! {
        {move || {
            if vm.raw_json_loading.get() {
                return view! {
                    <div class="detail-grid">
                        <CardAnimated delay_ms=0 nav_id="a032_wb_returns_claims_details_json_loading">
                            <Flex gap=FlexGap::Small style="align-items: center; justify-content: center; padding: var(--spacing-xl);">
                                <Spinner />
                                <span>"Загрузка JSON..."</span>
                            </Flex>
                        </CardAnimated>
                    </div>
                }
                .into_any();
            }

            if let Some(json) = vm.raw_json.get() {
                view! {
                    <div class="detail-grid">
                        <CardAnimated
                            delay_ms=0
                            nav_id="a032_wb_returns_claims_details_json_main"
                            style="padding: var(--spacing-sm);"
                        >
                            <h4 class="details-section__title">"JSON заявки"</h4>
                            <div style="margin-bottom: var(--spacing-sm); color: var(--color-text-secondary);">
                                "Сериализованное представление заявки."
                            </div>
                            <div style="max-height: calc(100vh - 290px); overflow: auto;">
                                <JsonViewer json_content=json title="WB Returns Claim JSON".to_string() />
                            </div>
                        </CardAnimated>
                    </div>
                }
                .into_any()
            } else {
                view! {
                    <div class="detail-grid">
                        <CardAnimated delay_ms=0 nav_id="a032_wb_returns_claims_details_json_empty">
                            <h4 class="details-section__title">"JSON заявки"</h4>
                            <div style="color: var(--color-text-secondary);">
                                "JSON данные не загружены"
                            </div>
                        </CardAnimated>
                    </div>
                }
                .into_any()
            }
        }}
    }
}
