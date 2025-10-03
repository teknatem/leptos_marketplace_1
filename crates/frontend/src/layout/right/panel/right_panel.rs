use leptos::prelude::*;

#[component]
pub fn RightPanel() -> impl IntoView {
    view! {
        <div class="right-panel">
            <h3>Информационная панель</h3>

            <div class="info-item">
                <div class="info-label">Статус системы</div>
                <div class="info-value">Активна</div>
            </div>

            <div class="info-item">
                <div class="info-label">Версия</div>
                <div class="info-value">1.0.0</div>
            </div>
        </div>
    }
}
