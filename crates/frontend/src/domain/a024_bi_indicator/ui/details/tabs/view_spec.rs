//! ViewSpec tab — style picker + optional HTML/CSS code editors

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
    let style_sig = vm.view_spec_style_name;
    let srcdoc = vm.build_preview_srcdoc();

    let is_custom = Signal::derive(move || style_sig.get() == "custom");

    // ── HTML editor ──────────────────────────────────────────────────────
    let html_ref = NodeRef::<leptos::html::Textarea>::new();
    let html_sig = vm.view_spec_custom_html;

    {
        let r = html_ref.clone();
        Effect::new(move |_| {
            let _track = html_sig.get();
            request_animation_frame(move || {
                if let Some(el) = r.get() {
                    auto_resize_ta(&el);
                }
            });
        });
    }

    let on_html_input = move |ev: web_sys::Event| {
        let ta: web_sys::HtmlTextAreaElement = ev.target().unwrap().unchecked_into();
        html_sig.set(ta.value());
        auto_resize_ta(&ta);
    };

    let on_format_html = move |_| {
        html_sig.set(code_format::format_html(&html_sig.get_untracked()));
    };

    // ── CSS editor ───────────────────────────────────────────────────────
    let css_ref = NodeRef::<leptos::html::Textarea>::new();
    let css_sig = vm.view_spec_custom_css;

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

            // ── Style picker ─────────────────────────────────────────────
            <CardAnimated delay_ms=0>
                <h4 class="details-section__title">"Стиль карточки"</h4>
                <p class="bi-viewspec__hint" style="margin-bottom: var(--spacing-sm);">
                    "Разработчик задаёт шаблоны стилей. Пользователь выбирает стиль — "
                    "система подставляет данные и отображает карточку."
                </p>
                <RadioGroup value=style_sig>
                    <Radio value="classic" label="Classic — спарклайн + левая цветовая полоса + delta-pill" />
                    <Radio value="modern" label="Modern — кольцо прогресса + градиентная рамка + dot-delta" />
                    <Radio value="custom" label="Custom — произвольный HTML/CSS (или LLM-генерация)" />
                </RadioGroup>
            </CardAnimated>

            // ── Live preview (always visible) ────────────────────────────
            <CardAnimated delay_ms=60>
                <h4 class="details-section__title">
                    {icon("eye")} " Предпросмотр"
                </h4>
                <div class="bi-viewspec__preview-frame">
                    <iframe
                        sandbox="allow-same-origin"
                        srcdoc=move || srcdoc.get()
                        style="width: 100%; height: 100%; border: none; border-radius: 8px; background: transparent;"
                    ></iframe>
                </div>
            </CardAnimated>

            // ── Custom HTML/CSS editors — only shown for "custom" style ──
            {move || is_custom.get().then(|| view! {
                <CardAnimated delay_ms=80>
                    <div class="bi-viewspec__card-header">
                        <div>
                            <h4 class="details-section__title">"HTML шаблон"</h4>
                            <p class="bi-viewspec__hint">
                                "Плейсхолдеры: "
                                <code>"{{value}}"</code>
                                ", "
                                <code>"{{delta}}"</code>
                                ", "
                                <code>"{{title}}"</code>
                                ", "
                                <code>"{{status}}"</code>
                                ", "
                                <code>"{{chip}}"</code>
                                ". JS запрещён."
                            </p>
                        </div>
                        <Button appearance=ButtonAppearance::Subtle on_click=on_format_html>
                            {icon("code")} " Форматировать"
                        </Button>
                    </div>
                    <textarea
                        node_ref=html_ref
                        class="code-editor"
                        placeholder="<div class=\"bi-card\">\n  <span>{{title}}</span>\n  <div>{{value}}</div>\n</div>"
                        prop:value=move || html_sig.get()
                        on:input=on_html_input
                    ></textarea>
                </CardAnimated>

                <CardAnimated delay_ms=100>
                    <div class="bi-viewspec__card-header">
                        <div>
                            <h4 class="details-section__title">"CSS стили"</h4>
                            <p class="bi-viewspec__hint">
                                "Переменные: "
                                <code>"--bi-primary"</code>
                                ", "
                                <code>"--bi-success"</code>
                                ", "
                                <code>"--bi-danger"</code>
                                ", "
                                <code>"--bi-text"</code>
                                ", "
                                <code>"--bi-text-secondary"</code>
                            </p>
                        </div>
                        <Button appearance=ButtonAppearance::Subtle on_click=on_format_css>
                            {icon("code")} " Форматировать"
                        </Button>
                    </div>
                    <textarea
                        node_ref=css_ref
                        class="code-editor"
                        placeholder=".bi-card {\n  text-align: center;\n}\n.bi-card__value {\n  font-size: 2.5rem;\n  font-weight: 700;\n}"
                        prop:value=move || css_sig.get()
                        on:input=on_css_input
                    ></textarea>
                </CardAnimated>
            })}

            // ── Format + Thresholds (always visible) ─────────────────────
            <div class="bi-viewspec__row2">
                <CardAnimated delay_ms=120>
                    <h4 class="details-section__title">"Формат значения (JSON)"</h4>
                    <p class="bi-viewspec__hint">
                        <code>"{ \"kind\": \"Money\", \"currency\": \"RUB\" }"</code>
                    </p>
                    <div class="form__group">
                        <Textarea
                            value=vm.view_spec_format_json
                            placeholder="{}"
                            attr:rows=6
                            attr:style="font-family: monospace; font-size: 12px; width: 100%;"
                        />
                    </div>
                </CardAnimated>

                <CardAnimated delay_ms=140>
                    <h4 class="details-section__title">"Пороговые значения (JSON)"</h4>
                    <p class="bi-viewspec__hint">
                        <code>"[{ \"condition\": \"< 10\", \"color\": \"#e53935\" }]"</code>
                    </p>
                    <div class="form__group">
                        <Textarea
                            value=vm.view_spec_thresholds_json
                            placeholder="[]"
                            attr:rows=6
                            attr:style="font-family: monospace; font-size: 12px; width: 100%;"
                        />
                    </div>
                </CardAnimated>
            </div>
        </div>
    }
}

