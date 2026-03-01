//! DrillSpec tab — drill-down configuration as JSON

use super::super::view_model::BiIndicatorDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn DrillSpecTab(vm: BiIndicatorDetailsVm) -> impl IntoView {
    view! {
        <CardAnimated delay_ms=0>
            <h4 class="details-section__title">"Drill-down (DrillSpec JSON)"</h4>
            <p class="details-section__hint" style="color: var(--color-text-secondary); font-size: 12px; margin-bottom: var(--spacing-sm);">
                {"Опционально. Оставьте пустым если drill-down не нужен. Пример: { \"target\": { \"type\": \"Explore\", \"schema_id\": \"...\" }, \"filter_mapping\": {} }"}
            </p>
            <div class="form__group">
                <Textarea
                    value=vm.drill_spec_json
                    placeholder=""
                    attr:rows=18
                    attr:style="font-family: monospace; font-size: 12px; width: 100%;"
                />
            </div>
        </CardAnimated>
    }
}
