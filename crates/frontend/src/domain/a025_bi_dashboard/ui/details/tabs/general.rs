//! General tab — basic fields + rating

use super::super::view_model::BiDashboardDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
use leptos::prelude::*;
use thaw::*;
use wasm_bindgen::JsCast;

#[component]
pub fn GeneralTab(vm: BiDashboardDetailsVm) -> impl IntoView {
    let status_sig = vm.status;
    let rating_sig = vm.rating;

    // Bridge: RwSignal<Option<f32>> for the Thaw Rating component
    let rating_f32: RwSignal<Option<f32>> =
        RwSignal::new(rating_sig.get_untracked().map(|r| r as f32));

    // When rating_f32 changes, sync back to rating_sig (u8)
    Effect::new(move |_| {
        let v = rating_f32.get();
        rating_sig.set(v.map(|f| f.round().clamp(1.0, 5.0) as u8));
    });

    view! {
        <div class="details-tabs__content">
            <CardAnimated delay_ms=0 nav_id="a025_bi_dashboard_details_general_main">
                <div class="details-section">
                    <h4 class="details-section__title">"Основные поля"</h4>
                    <div class="form__grid form__grid--2col">
                        <div class="form__group">
                            <label class="form__label">"Код"</label>
                            <Input value=vm.code placeholder="DASH-001" />
                        </div>
                        <div class="form__group">
                            <label class="form__label">"Статус"</label>
                            <select
                                class="form__select"
                                on:change=move |ev| {
                                    let target = ev.target().unwrap();
                                    let sel: &web_sys::HtmlSelectElement = target.unchecked_ref();
                                    status_sig.set(sel.value());
                                }
                            >
                                <option value="draft" selected=move || status_sig.get() == "draft">"Черновик"</option>
                                <option value="active" selected=move || status_sig.get() == "active">"Активен"</option>
                                <option value="archived" selected=move || status_sig.get() == "archived">"Архив"</option>
                            </select>
                        </div>
                    </div>

                    <div class="form__group">
                        <label class="form__label">"Наименование"</label>
                        <Input value=vm.description placeholder="Операционный дашборд" />
                    </div>

                    <div class="form__group">
                        <label class="form__label">"Комментарий"</label>
                        <Textarea value=vm.comment placeholder="Описание дашборда и его назначения" />
                    </div>

                    <div class="form__grid form__grid--2col">
                        <div class="form__group">
                            <label class="form__label">"Владелец (user ID)"</label>
                            <Input value=vm.owner_user_id placeholder="UUID пользователя" />
                        </div>
                        <div class="form__group form__group--inline">
                            <Checkbox checked=vm.is_public label="Публичный" />
                        </div>
                    </div>

                    <div class="form__group">
                        <label class="form__label">"Оценка"</label>
                        <div class="form__rate-row">
                            <Rating
                                value=rating_f32
                                max=5u8
                            />
                            <span class="form__rate-label">
                                {move || match rating_sig.get() {
                                    None => "не оценён".to_string(),
                                    Some(0) => "не оценён".to_string(),
                                    Some(1) => "★ Плохо".to_string(),
                                    Some(2) => "★★ Удовлетворительно".to_string(),
                                    Some(3) => "★★★ Хорошо".to_string(),
                                    Some(4) => "★★★★ Отлично".to_string(),
                                    Some(5) => "★★★★★ Превосходно".to_string(),
                                    Some(_) => String::new(),
                                }}
                            </span>
                        </div>
                    </div>
                </div>
            </CardAnimated>
        </div>
    }
}
