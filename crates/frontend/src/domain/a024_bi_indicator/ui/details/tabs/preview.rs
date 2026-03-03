//! Preview tab — live sandbox rendering of the indicator

use super::super::view_model::BiIndicatorDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::icons::icon;
use leptos::prelude::*;
use thaw::*;
use wasm_bindgen::JsCast;

#[component]
pub fn PreviewTab(vm: BiIndicatorDetailsVm) -> impl IntoView {
    let srcdoc = vm.build_preview_srcdoc();
    let style_sig = vm.view_spec_style_name;

    let is_modern = Signal::derive(move || style_sig.get() == "modern");
    let is_custom = Signal::derive(move || style_sig.get() == "custom");

    let iframe_dims = {
        let size_sig = vm.preview_size;
        Signal::derive(move || match size_sig.get().as_str() {
            "2x1" => ("100%", "200px"),
            "2x2" => ("100%", "400px"),
            "1x2" => ("50%", "400px"),
            _ => ("100%", "200px"),
        })
    };

    let preview_size = vm.preview_size;
    let preview_status = vm.preview_status;
    let preview_delta_dir = vm.preview_delta_dir;

    view! {
        <div class="bi-preview">
            <CardAnimated delay_ms=0>
                <div class="bi-preview__controls">
                    <h4 class="details-section__title">"Тестовые данные"</h4>
                    <div class="bi-preview__test-data">
                        <div class="form__group">
                            <label class="form__label">"Название"</label>
                            <Input value=vm.preview_title placeholder="Выручка" />
                        </div>
                        <div class="form__group">
                            <label class="form__label">"Значение"</label>
                            <Input value=vm.preview_value placeholder="₽2.40M" />
                        </div>
                        <div class="form__group">
                            <label class="form__label">"Дельта"</label>
                            <Input value=vm.preview_delta placeholder="+12.5%" />
                        </div>
                        <div class="form__group">
                            <label class="form__label">"Направление дельты"</label>
                            <select
                                class="form__select"
                                on:change=move |ev| {
                                    let target = ev.target().unwrap();
                                    let sel: &web_sys::HtmlSelectElement = target.unchecked_ref();
                                    preview_delta_dir.set(sel.value());
                                }
                            >
                                <option value="up" selected=true>"↑ Вверх (ok)"</option>
                                <option value="down">"↓ Вниз (bad)"</option>
                                <option value="flat">"→ Нейтрально"</option>
                            </select>
                        </div>
                        <div class="form__group">
                            <label class="form__label">"Статус"</label>
                            <select
                                class="form__select"
                                on:change=move |ev| {
                                    let target = ev.target().unwrap();
                                    let sel: &web_sys::HtmlSelectElement = target.unchecked_ref();
                                    preview_status.set(sel.value());
                                }
                            >
                                <option value="ok" selected=true>"ok (зелёный)"</option>
                                <option value="bad">"bad (красный)"</option>
                                <option value="warn">"warn (жёлтый)"</option>
                                <option value="neutral">"neutral (серый)"</option>
                            </select>
                        </div>
                        <div class="form__group">
                            <label class="form__label">"Категория (chip)"</label>
                            <Input value=vm.preview_chip placeholder="Выручка" />
                        </div>

                        // Progress ring — only relevant for modern style
                        {move || is_modern.get().then(|| view! {
                            <div class="form__group">
                                <label class="form__label">"Прогресс (0–100)"</label>
                                <input
                                    type="number"
                                    class="form__input"
                                    min="0"
                                    max="100"
                                    prop:value=move || vm.preview_progress.get().to_string()
                                    on:input=move |ev| {
                                        let input: web_sys::HtmlInputElement =
                                            ev.target().unwrap().unchecked_into();
                                        let val: u8 = input.value().parse().unwrap_or(50);
                                        vm.preview_progress.set(val.min(100));
                                    }
                                />
                            </div>
                        })}

                        <div class="form__group">
                            <label class="form__label">"Размер ячейки"</label>
                            <select
                                class="form__select"
                                on:change=move |ev| {
                                    let target = ev.target().unwrap();
                                    let sel: &web_sys::HtmlSelectElement = target.unchecked_ref();
                                    preview_size.set(sel.value());
                                }
                            >
                                <option value="1x1" selected=true>"1x1 (компактный)"</option>
                                <option value="2x1">"2x1 (широкий)"</option>
                                <option value="1x2">"1x2 (высокий)"</option>
                                <option value="2x2">"2x2 (большой)"</option>
                            </select>
                        </div>
                    </div>
                </div>
            </CardAnimated>

            <CardAnimated delay_ms=80>
                <h4 class="details-section__title">
                    {icon("eye")} " Предпросмотр индикатора"
                </h4>
                <div class="bi-preview__sandbox" style="margin-top: var(--spacing-sm);">
                    <div
                        class="bi-preview__frame-wrapper"
                        style:width=move || iframe_dims.get().0
                        style:height=move || iframe_dims.get().1
                    >
                        <iframe
                            class="bi-preview__iframe"
                            sandbox="allow-same-origin"
                            srcdoc=move || srcdoc.get()
                            style="width: 100%; height: 100%; border: none; border-radius: 8px; background: transparent;"
                        ></iframe>
                    </div>
                </div>
            </CardAnimated>

            // Hint only for custom style when no HTML/CSS set
            {move || {
                if !is_custom.get() {
                    return None;
                }
                let style_sig2 = vm.view_spec_style_name;
                let html_sig = vm.view_spec_custom_html;
                let css_sig = vm.view_spec_custom_css;
                let html = html_sig.get();
                let css = css_sig.get();
                let _ = style_sig2.get();
                if html.trim().is_empty() && css.trim().is_empty() {
                    Some(view! {
                        <CardAnimated delay_ms=160>
                            <div class="bi-preview__empty-hint">
                                <p style="color: var(--color-text-secondary); text-align: center; padding: var(--spacing-lg);">
                                    "Стиль \"custom\" — нет HTML/CSS шаблона. "
                                    "Перейдите на вкладку "
                                    <strong>"ViewSpec"</strong>
                                    " для редактирования или используйте "
                                    <strong>"LLM-генерацию"</strong>
                                    "."
                                </p>
                            </div>
                        </CardAnimated>
                    })
                } else {
                    None
                }
            }}
        </div>
    }
}
