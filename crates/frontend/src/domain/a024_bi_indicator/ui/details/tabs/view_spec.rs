//! ViewSpec tab — custom_html, custom_css, format, thresholds

use super::super::view_model::BiIndicatorDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn ViewSpecTab(vm: BiIndicatorDetailsVm) -> impl IntoView {
    view! {
        <div class="detail-grid">
            <CardAnimated delay_ms=0>
                <h4 class="details-section__title">"HTML шаблон"</h4>
                <p class="details-section__hint" style="color: var(--color-text-secondary); font-size: 12px; margin-bottom: var(--spacing-sm);">
                    {"Разрешены плейсхолдеры: {{value}}, {{delta}}, {{title}}. JS запрещён — HTML санитизируется."}
                </p>
                <div class="form__group">
                    <Textarea
                        value=vm.view_spec_custom_html
                        placeholder={"<div class=\"indicator\">{{value}}</div>"}
                        attr:rows=10
                        attr:style="font-family: monospace; font-size: 12px; width: 100%;"
                    />
                </div>
            </CardAnimated>

            <CardAnimated delay_ms=80>
                <h4 class="details-section__title">"CSS стили"</h4>
                <div class="form__group">
                    <Textarea
                        value=vm.view_spec_custom_css
                        placeholder={".indicator { font-size: 2rem; font-weight: bold; }"}
                        attr:rows=10
                        attr:style="font-family: monospace; font-size: 12px; width: 100%;"
                    />
                </div>
            </CardAnimated>

            <CardAnimated delay_ms=160>
                <h4 class="details-section__title">"Формат значения (JSON)"</h4>
                <p class="details-section__hint" style="color: var(--color-text-secondary); font-size: 12px; margin-bottom: var(--spacing-sm);">
                    {"Пример: { \"format_type\": \"Currency\", \"currency\": \"RUB\", \"decimal_places\": 0 }"}
                </p>
                <div class="form__group">
                    <Textarea
                        value=vm.view_spec_format_json
                        placeholder="{}"
                        attr:rows=8
                        attr:style="font-family: monospace; font-size: 12px; width: 100%;"
                    />
                </div>
            </CardAnimated>

            <CardAnimated delay_ms=240>
                <h4 class="details-section__title">"Пороговые значения (JSON)"</h4>
                <p class="details-section__hint" style="color: var(--color-text-secondary); font-size: 12px; margin-bottom: var(--spacing-sm);">
                    {"Пример: [{ \"condition\": \"< 10\", \"color\": \"#e53935\", \"label\": \"Низкая маржа\" }]"}
                </p>
                <div class="form__group">
                    <Textarea
                        value=vm.view_spec_thresholds_json
                        placeholder="[]"
                        attr:rows=8
                        attr:style="font-family: monospace; font-size: 12px; width: 100%;"
                    />
                </div>
            </CardAnimated>
        </div>
    }
}
