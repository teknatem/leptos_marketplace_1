//! JSON Tab - Raw JSON data from Yandex Market API

use leptos::prelude::*;

#[component]
pub fn JsonTab(raw_json: Signal<Option<String>>) -> impl IntoView {
    view! {
        <div class="json-info">
            <div style="margin-bottom: var(--space-lg); font-size: var(--font-size-sm); font-weight: var(--font-weight-semibold);">
                "Raw JSON from Yandex Market API:"
            </div>
            {move || {
                if let Some(json) = raw_json.get() {
                    view! {
                        <pre style="background: var(--color-bg-secondary); padding: var(--space-lg); border-radius: var(--radius-sm); overflow-x: auto; font-size: var(--font-size-xs); border: 1px solid var(--color-border-lighter);">
                            {json}
                        </pre>
                    }
                        .into_any()
                } else {
                    view! {
                        <div style="padding: var(--space-xl); text-align: center; color: var(--color-text-muted); font-size: var(--font-size-sm);">
                            "Загрузка raw JSON из Yandex Market..."
                        </div>
                    }
                        .into_any()
                }
            }}

        </div>
    }
}
