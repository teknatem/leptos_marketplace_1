//! DataSpec tab — schema_id, sql_artifact_id, query_config JSON

use super::super::view_model::BiIndicatorDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn DataSpecTab(vm: BiIndicatorDetailsVm) -> impl IntoView {
    view! {
        <div class="detail-grid">
            <CardAnimated delay_ms=0>
                <h4 class="details-section__title">"Источник данных"</h4>
                <div class="details-grid--3col">
                    <div class="form__group" style="grid-column: 1 / 3;">
                        <label class="form__label">"Schema ID"</label>
                        <Input
                            value=vm.data_spec_schema_id
                            placeholder="Идентификатор схемы данных"
                        />
                    </div>

                    <div class="form__group">
                        <label class="form__label">"SQL артефакт (ID)"</label>
                        <Input
                            value=vm.data_spec_sql_artifact_id
                            placeholder="UUID артефакта a019 (необязательно)"
                        />
                    </div>
                </div>
            </CardAnimated>

            <CardAnimated delay_ms=80>
                <h4 class="details-section__title">"Query Config (JSON)"</h4>
                <div class="form__group">
                    <label class="form__label">"Конфигурация запроса (DashboardConfig)"</label>
                    <Textarea
                        value=vm.data_spec_query_config_json
                        placeholder="{}"
                        attr:rows=18
                        attr:style="font-family: monospace; font-size: 12px; width: 100%;"
                    />
                </div>
            </CardAnimated>
        </div>
    }
}
