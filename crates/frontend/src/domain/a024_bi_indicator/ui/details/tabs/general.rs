//! General tab — base fields: code, description, comment, status, owner, is_public

use super::super::view_model::BiIndicatorDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn GeneralTab(vm: BiIndicatorDetailsVm) -> impl IntoView {
    view! {
        <div class="detail-grid">
            <div class="detail-grid__col">
                <CardAnimated delay_ms=0 nav_id="a024_bi_indicator_details_general_requisites">
                    <h4 class="details-section__title">"Реквизиты"</h4>
                    <div class="form__group">
                        <label class="form__label">"Код"</label>
                        <Input value=vm.code placeholder="Уникальный код индикатора" />
                    </div>

                    <div class="form__group">
                        <label class="form__label">"Название индикатора *"</label>
                        <Input value=vm.description placeholder="Название индикатора" />
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

                    <div class="form__group">
                        <Checkbox checked=vm.is_public label="Публичный (виден всем)" />
                    </div>
                </CardAnimated>
            </div>

            <div class="detail-grid__col">
                <CardAnimated delay_ms=80 nav_id="a024_bi_indicator_details_general_description">
                    <h4 class="details-section__title">"Комментарий и пояснения"</h4>
                    <div class="form__group">
                        <label class="form__label">"Комментарий"</label>
                        <Textarea
                            value=vm.comment
                            placeholder="Комментарий, одной фразой, чтобы понять, что это за индикатор"
                            attr:rows=5
                        />
                    </div>
                    <div class="form__group">
                        <label class="form__label">"Подробное описание"</label>
                        <Textarea
                            value=vm.explanation
                            placeholder="Опишите, что показывает индикатор, как он рассчитывается, откуда берутся данные и как пользователю интерпретировать результат."
                        />
                    </div>

                </CardAnimated>
            </div>
        </div>
    }
}
