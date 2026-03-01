//! General tab — base fields: code, description, comment, status, owner, is_public

use super::super::view_model::BiIndicatorDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn GeneralTab(vm: BiIndicatorDetailsVm) -> impl IntoView {
    view! {
        <CardAnimated delay_ms=0>
            <h4 class="details-section__title">"Основные поля"</h4>
            <div class="details-grid--3col">
                <div class="form__group">
                    <label class="form__label">"Код"</label>
                    <Input value=vm.code placeholder="Уникальный код индикатора" />
                </div>

                <div class="form__group" style="grid-column: 2 / -1;">
                    <label class="form__label">"Наименование *"</label>
                    <Input value=vm.description placeholder="Название индикатора" />
                </div>

                <div class="form__group" style="grid-column: 1 / -1;">
                    <label class="form__label">"Комментарий"</label>
                    <Textarea value=vm.comment placeholder="Опционально" attr:rows=3 />
                </div>

                <div class="form__group">
                    <label class="form__label">"Статус"</label>
                    <Select value=vm.status>
                        <option value="draft">"Черновик"</option>
                        <option value="active">"Активен"</option>
                        <option value="archived">"Архив"</option>
                    </Select>
                </div>

                <div class="form__group">
                    <label class="form__label">"Владелец (user_id)"</label>
                    <Input value=vm.owner_user_id placeholder="ID пользователя" />
                </div>

                <div class="form__group" style="display: flex; align-items: center; padding-top: 24px;">
                    <Checkbox checked=vm.is_public label="Публичный (виден всем)" />
                </div>
            </div>
        </CardAnimated>
    }
}
