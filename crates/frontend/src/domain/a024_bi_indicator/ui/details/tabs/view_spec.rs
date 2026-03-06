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
        <div class="bi-viewspec">
            <CardAnimated delay_ms=0>
                <div class="bi-viewspec__card-header">
                    <div>
                        <h4 class="details-section__title">"Custom CSS"</h4>
                        <p class="bi-viewspec__hint">
                            "Используйте селектор "
                            <code>".indicator-card"</code>
                            " и его дочерние элементы. Дизайн «Custom CSS» выбирается на закладке Превью."
                        </p>
                    </div>
                    <Button appearance=ButtonAppearance::Subtle on_click=on_format_css>
                        {icon("code")} " Форматировать"
                    </Button>
                </div>
                <textarea
                    node_ref=css_ref
                    class="code-editor"
                    placeholder=".indicator-card { border-radius: 20px; }"
                    prop:value=move || css_sig.get()
                    on:input=on_css_input
                ></textarea>
            </CardAnimated>

            <div class="bi-viewspec__row2">
                <CardAnimated delay_ms=60>
                    <h4 class="details-section__title">"Формат значения (JSON)"</h4>
                    <p class="bi-viewspec__hint">
                        <code>"{ \"kind\": \"Money\", \"currency\": \"RUB\" }"</code>
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

                <CardAnimated delay_ms=80>
                    <h4 class="details-section__title">"Пороговые значения (JSON)"</h4>
                    <p class="bi-viewspec__hint">
                        <code>"[{ \"condition\": \"< 10\", \"color\": \"#e53935\" }]"</code>
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
        </div>
    }
}
