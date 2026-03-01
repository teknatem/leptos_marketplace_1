//! Meta tab — read-only metadata fields

use super::super::view_model::BiIndicatorDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn MetaTab(vm: BiIndicatorDetailsVm) -> impl IntoView {
    view! {
        <CardAnimated delay_ms=0>
            <h4 class="details-section__title">"Метаданные"</h4>
            <div class="details-grid--3col">
                <div class="form__group">
                    <label class="form__label">"Создан"</label>
                    <Input value=vm.created_at disabled=true placeholder="—" />
                </div>

                <div class="form__group">
                    <label class="form__label">"Изменён"</label>
                    <Input value=vm.updated_at disabled=true placeholder="—" />
                </div>

                <div class="form__group">
                    <label class="form__label">"Версия"</label>
                    <span class="form__value" style="padding: 4px 0; display: block;">
                        {move || vm.version.get().to_string()}
                    </span>
                </div>

                <div class="form__group">
                    <label class="form__label">"Создал"</label>
                    <Input value=vm.created_by disabled=true placeholder="—" />
                </div>

                <div class="form__group">
                    <label class="form__label">"Изменил"</label>
                    <Input value=vm.updated_by disabled=true placeholder="—" />
                </div>
            </div>
        </CardAnimated>
    }
}
