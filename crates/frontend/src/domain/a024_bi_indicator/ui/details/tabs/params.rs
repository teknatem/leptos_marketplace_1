//! Params tab — typed parameters as JSON array

use super::super::view_model::BiIndicatorDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn ParamsTab(vm: BiIndicatorDetailsVm) -> impl IntoView {
    view! {
        <CardAnimated delay_ms=0>
            <h4 class="details-section__title">"Параметры (ParamDef[])"</h4>
            <p class="details-section__hint" style="color: var(--color-text-secondary); font-size: 12px; margin-bottom: var(--spacing-sm);">
                {"Массив параметров. Каждый элемент: { \"key\": \"...\", \"label\": \"...\", \"param_type\": \"Date|Period|...\", \"default_value\": ..., \"global_filter_key\": \"...\" }"}
            </p>
            <div class="form__group">
                <Textarea
                    value=vm.params_json
                    placeholder="[]"
                    attr:rows=22
                    attr:style="font-family: monospace; font-size: 12px; width: 100%;"
                />
            </div>
        </CardAnimated>
    }
}
