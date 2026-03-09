//! DrillSpec tab — drill-down configuration as JSON

use super::super::view_model::BiIndicatorDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn DrillSpecTab(vm: BiIndicatorDetailsVm) -> impl IntoView {
    view! {
        <CardAnimated delay_ms=0 nav_id="a024_bi_indicator_details_drill_spec_main">
            <h4 class="details-section__title">"Drill-down (DrillSpec JSON)"</h4>
            <p class="form__hint">
                "Опционально. Оставьте пустым, если для индикатора достаточно стандартного DataView drilldown. "
                "Текущий контракт: "
                <code>"{ \"target_type\": \"explore\", \"target_id\": \"...\", \"filter_mapping\": {} }"</code>
            </p>
            <div class="form__group">
                <Textarea
                    value=vm.drill_spec_json
                    placeholder=""
                    attr:rows=18
                    attr:class="code-editor bi-viewspec__json-editor"
                />
            </div>
        </CardAnimated>
    }
}
