//! JSON Tab - displays current dashboard configuration as JSON

use crate::shared::json_viewer::JsonViewer;
use contracts::shared::universal_dashboard::DashboardConfig;
use leptos::prelude::*;
use thaw::{Button, ButtonAppearance, Card};

#[component]
pub fn JsonTab(#[prop(into)] config: Signal<DashboardConfig>) -> impl IntoView {
    let generated_json = RwSignal::new(None::<String>);

    let handle_refresh = move |_| {
        let cfg = config.get();
        match serde_json::to_string_pretty(&cfg) {
            Ok(json) => generated_json.set(Some(json)),
            Err(e) => generated_json.set(Some(format!("Ошибка сериализации: {}", e))),
        }
    };

    view! {
        <div class="json-tab">
            <Card>
                <div style="margin-bottom: 16px;">
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=handle_refresh
                    >
                        "Обновить JSON"
                    </Button>
                </div>

                {move || generated_json.get().map(|json| {
                    view! {
                        <JsonViewer
                            json_content=json
                            title="Конфигурация Dashboard".to_string()
                        />
                    }
                })}
            </Card>
        </div>
    }
}
