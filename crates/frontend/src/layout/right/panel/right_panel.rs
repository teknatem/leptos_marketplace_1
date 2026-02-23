//! Right panel component - правая боковая панель
//!
//! Содержит список открытых окон и другую контекстную информацию

use super::windows_list::WindowsList;
use leptos::prelude::*;

#[component]
pub fn RightPanel() -> impl IntoView {
    view! {
        <div class="app-panel__content">
            <WindowsList />
        </div>
    }
}
