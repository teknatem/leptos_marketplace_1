use crate::domain::a032_wb_returns_claims::ui::details::view_model::WbReturnsClaimsDetailsVm;
use leptos::prelude::*;

#[component]
pub fn JsonTab(vm: WbReturnsClaimsDetailsVm) -> impl IntoView {
    view! {
        <div class="tab-content tab-content--data">
            <div class="card card--animated" data-nav-id="a032_wb_returns_claims_details_json">
                <div class="card__body">
                    <h3 class="details-section__title">"Данные (JSON)"</h3>
                    {move || {
                        if let Some(json) = vm.raw_json.get() {
                            view! {
                                <pre class="code-block code-block--json">{json}</pre>
                            }
                            .into_any()
                        } else {
                            view! {
                                <div class="text-muted">"Нет данных"</div>
                            }
                            .into_any()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}
