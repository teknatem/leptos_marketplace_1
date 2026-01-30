//! Result Tab - displays pivot table results

use leptos::prelude::*;
use contracts::shared::universal_dashboard::ExecuteDashboardResponse;
use crate::shared::universal_dashboard::ui::PivotTable;

#[component]
pub fn ResultTab(
    #[prop(into)] loading: Signal<bool>,
    #[prop(into)] error: Signal<Option<String>>,
    #[prop(into)] response: Signal<Option<ExecuteDashboardResponse>>,
) -> impl IntoView {
    view! {
        <div class="result-tab">
            {move || {
                if loading.get() {
                    view! {
                        <div class="loading-state">
                            <div class="spinner"></div>
                            <p>"Выполнение запроса..."</p>
                        </div>
                    }.into_any()
                } else if let Some(err) = error.get() {
                    view! {
                        <div class="error-state">
                            <p class="error-title">"Ошибка"</p>
                            <p class="error-message">{err}</p>
                        </div>
                    }.into_any()
                } else if response.get().is_none() {
                    view! {
                        <div class="empty-state">
                            <p>"Настройте поля и нажмите \"Обновить\""</p>
                        </div>
                    }.into_any()
                } else {
                    view! { <PivotTable response=response /> }.into_any()
                }
            }}
        </div>
    }
}
