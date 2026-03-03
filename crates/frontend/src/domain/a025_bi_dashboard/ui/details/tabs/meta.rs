//! Meta tab — read-only metadata display

use super::super::view_model::BiDashboardDetailsVm;
use crate::shared::components::card_animated::CardAnimated;
use leptos::prelude::*;

#[component]
pub fn MetaTab(vm: BiDashboardDetailsVm) -> impl IntoView {
    view! {
        <div class="details-tabs__content">
            <CardAnimated delay_ms=0>
                <div class="details-section">
                    <h4 class="details-section__title">"Метаданные"</h4>
                    <table class="form__meta-table">
                        <tbody>
                            <tr>
                                <td class="form__meta-key">"ID"</td>
                                <td class="form__meta-value">
                                    {move || vm.id.get().unwrap_or_else(|| "—".to_string())}
                                </td>
                            </tr>
                            <tr>
                                <td class="form__meta-key">"Версия"</td>
                                <td class="form__meta-value">{move || vm.version.get().to_string()}</td>
                            </tr>
                            <tr>
                                <td class="form__meta-key">"Создан"</td>
                                <td class="form__meta-value">{move || {
                                    let v = vm.created_at.get();
                                    if v.is_empty() { "—".to_string() } else { v }
                                }}</td>
                            </tr>
                            <tr>
                                <td class="form__meta-key">"Обновлён"</td>
                                <td class="form__meta-value">{move || {
                                    let v = vm.updated_at.get();
                                    if v.is_empty() { "—".to_string() } else { v }
                                }}</td>
                            </tr>
                            <tr>
                                <td class="form__meta-key">"Создал"</td>
                                <td class="form__meta-value">{move || {
                                    let v = vm.created_by.get();
                                    if v.is_empty() { "—".to_string() } else { v }
                                }}</td>
                            </tr>
                            <tr>
                                <td class="form__meta-key">"Обновил"</td>
                                <td class="form__meta-value">{move || {
                                    let v = vm.updated_by.get();
                                    if v.is_empty() { "—".to_string() } else { v }
                                }}</td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </CardAnimated>
        </div>
    }
}
