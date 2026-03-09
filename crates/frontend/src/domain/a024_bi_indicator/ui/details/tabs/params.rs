//! Params tab — typed parameters as JSON array

use super::super::view_model::BiIndicatorDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn ParamsTab(vm: BiIndicatorDetailsVm) -> impl IntoView {
    view! {
        <CardAnimated delay_ms=0 nav_id="a024_bi_indicator_details_params_main">
            <h4 class="details-section__title">"Параметры индикатора (ParamDef[])"</h4>
            <p class="form__hint">
                "Используйте только актуальные типы: "
                <code>"date | date_range | string | integer | float | boolean | ref"</code>
                ". Если метрика и фильтры уже задаются через DataView, не дублируйте их здесь без необходимости."
            </p>
            <div class="form__group">
                <Textarea
                    value=vm.params_json
                    placeholder="[]"
                    attr:rows=22
                    attr:class="code-editor bi-viewspec__json-editor"
                />
            </div>
        </CardAnimated>
    }
}
