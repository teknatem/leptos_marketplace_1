//! General tab - basic nomenclature fields
//!
//! Contains: description, full_description, code, article, parent_id, comment, is_folder

use super::super::view_model::NomenclatureDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
use leptos::prelude::*;
use thaw::*;

/// General tab component with basic nomenclature fields
#[component]
pub fn GeneralTab(vm: NomenclatureDetailsVm) -> impl IntoView {
    view! {
        <CardAnimated delay_ms=0>
            <h4 class="details-section__title">"Основные поля"</h4>
            <div class="details-grid--3col">
                <div class="form__group" style="grid-column: 1 / -1;">
                    <label class="form__label">"Наименование *"</label>
                    <Input value=vm.description placeholder="Введите наименование" />
                </div>

                <div class="form__group" style="grid-column: 1 / -1;">
                    <label class="form__label">"Полное наименование"</label>
                    <Input value=vm.full_description placeholder="Опционально" />
                </div>

                <div class="form__group">
                    <label class="form__label">"Код"</label>
                    <Input value=vm.code placeholder="Опционально" />
                </div>

                <div class="form__group">
                    <label class="form__label">"Артикул"</label>
                    <Input value=vm.article placeholder="Опционально" />
                </div>

                <div class="form__group">
                    <label class="form__label">"Родитель (UUID)"</label>
                    <Input value=vm.parent_id placeholder="Опционально" />
                </div>

                <div class="form__group" style="grid-column: 1 / -1;">
                    <label class="form__label">"Комментарий"</label>
                    <Textarea value=vm.comment placeholder="Опционально" attr:rows=3 />
                </div>

                <div class="details-flags" style="grid-column: 1 / 2;">
                    <Checkbox checked=vm.is_folder label="Это папка" />
                </div>

                <div class="details-flags" style="grid-column: 2 / -1;">
                    <Checkbox
                        checked=vm.is_derivative
                        attr:disabled=true
                        label="Производная позиция"
                    />
                </div>

                <div class="form__group" style="grid-column: 1 / -1;">
                    <label class="form__label">"Базовая номенклатура (UUID)"</label>
                    <Input value=vm.base_nomenclature_ref disabled=true placeholder="Не задано" />
                </div>
            </div>
        </CardAnimated>
    }
}
