//! General tab - basic nomenclature fields
//!
//! Contains: description, full_description, code, article, parent_id, comment, is_folder

use super::super::view_model::NomenclatureDetailsVm;
use leptos::prelude::*;
use thaw::*;

/// General tab component with basic nomenclature fields
#[component]
pub fn GeneralTab(vm: NomenclatureDetailsVm) -> impl IntoView {
    view! {
        <div class="details-section">
            <h4 class="details-section__title">"Основные поля"</h4>
            <div class="details-grid--3col">
                // Description (required)
                <div class="form__group" style="grid-column: 1 / -1;">
                    <label class="form__label">"Наименование *"</label>
                    <Input value=vm.description placeholder="Введите наименование" />
                </div>

                // Full description
                <div class="form__group" style="grid-column: 1 / -1;">
                    <label class="form__label">"Полное наименование"</label>
                    <Input value=vm.full_description placeholder="Опционально" />
                </div>

                // Code
                <div class="form__group">
                    <label class="form__label">"Код"</label>
                    <Input value=vm.code placeholder="Опционально" />
                </div>

                // Article
                <div class="form__group">
                    <label class="form__label">"Артикул"</label>
                    <Input value=vm.article placeholder="Опционально" />
                </div>

                // Parent ID
                <div class="form__group">
                    <label class="form__label">"Родитель (UUID)"</label>
                    <Input value=vm.parent_id placeholder="Опционально" />
                </div>

                // Comment
                <div class="form__group" style="grid-column: 1 / -1;">
                    <label class="form__label">"Комментарий"</label>
                    <Textarea value=vm.comment placeholder="Опционально" attr:rows=3 />
                </div>

                // Flags
                <div class="details-flags" style="grid-column: 1 / -1;">
                    <Checkbox checked=vm.is_folder label="Это папка" />
                </div>
            </div>
        </div>
    }
}
