//! ViewSpec tab - Custom CSS + JSON format/thresholds editors

use super::super::view_model::BiIndicatorDetailsVm;
use crate::shared::code_format;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::icons::icon;
use leptos::prelude::*;
use thaw::*;
use wasm_bindgen::JsCast;

fn auto_resize_ta(ta: &web_sys::HtmlTextAreaElement) {
    let _ = ta.set_attribute("style", "");
    let sh = ta.scroll_height();
    if sh > 24 {
        let _ = ta.set_attribute("style", &format!("height: {}px", sh));
    }
}

#[component]
pub fn ViewSpecTab(vm: BiIndicatorDetailsVm) -> impl IntoView {
    let css_sig = vm.view_spec_custom_css;

    let css_ref = NodeRef::<leptos::html::Textarea>::new();
    {
        let r = css_ref.clone();
        Effect::new(move |_| {
            let _track = css_sig.get();
            request_animation_frame(move || {
                if let Some(el) = r.get() {
                    auto_resize_ta(&el);
                }
            });
        });
    }

    let on_css_input = move |ev: web_sys::Event| {
        let ta: web_sys::HtmlTextAreaElement = ev.target().unwrap().unchecked_into();
        css_sig.set(ta.value());
        auto_resize_ta(&ta);
    };

    let on_format_css = move |_| {
        css_sig.set(code_format::format_css(&css_sig.get_untracked()));
    };

    view! {
        <CardAnimated delay_ms=0 nav_id="a024_bi_indicator_details_view_spec_main">
            <div class="bi-viewspec">
                <div class="bi-viewspec__card-header">
                    <div>
                        <h4 class="details-section__title">"ViewSpec"</h4>
                        <p class="bi-viewspec__hint">
                            "Настройки отображения индикатора: CSS для карточки, форматирование числа и правила порогов."
                        </p>
                    </div>
                    <Button appearance=ButtonAppearance::Subtle on_click=on_format_css>
                        {icon("code")} " Форматировать CSS"
                    </Button>
                </div>

                <div class="bi-indicator-action__section">
                    <div class="bi-indicator-action__section-header">
                        <h5 class="bi-indicator-action__section-title">"Custom CSS"</h5>
                    </div>
                    <p class="form__hint">
                        "Используйте селектор " <code>".indicator-card"</code>
                        " и его дочерние элементы. Пользовательский стиль применяется на вкладке превью и в дашборде."
                    </p>
                    <textarea
                        node_ref=css_ref
                        class="code-editor"
                        placeholder=".indicator-card { border-radius: 20px; }"
                        prop:value=move || css_sig.get()
                        on:input=on_css_input
                    ></textarea>
                </div>

                <div class="bi-viewspec__row2">
                    <div class="bi-indicator-action__section bi-indicator-action__section--compact">
                        <div class="bi-indicator-action__section-header">
                            <h5 class="bi-indicator-action__section-title">"Формат значения"</h5>
                        </div>
                        <p class="form__hint">
                            <code>"{ \"kind\": \"Money\", \"currency\": \"RUB\" }"</code>
                        </p>
                        <Textarea
                            value=vm.view_spec_format_json
                            placeholder="{}"
                            attr:rows=10
                            attr:class="code-editor bi-viewspec__json-editor"
                        />
                    </div>

                    <div class="bi-indicator-action__section bi-indicator-action__section--compact">
                        <div class="bi-indicator-action__section-header">
                            <h5 class="bi-indicator-action__section-title">"Пороги"</h5>
                        </div>
                        <p class="form__hint">
                            <code>"[{ \"condition\": \"< 10\", \"color\": \"#e53935\" }]"</code>
                        </p>
                        <Textarea
                            value=vm.view_spec_thresholds_json
                            placeholder="[]"
                            attr:rows=10
                            attr:class="code-editor bi-viewspec__json-editor"
                        />
                    </div>
                </div>
            </div>
        </CardAnimated>
    }
}
