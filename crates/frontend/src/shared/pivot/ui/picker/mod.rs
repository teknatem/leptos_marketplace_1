//! Schema picker component for selecting a pivot schema

use leptos::prelude::*;
use contracts::shared::pivot::SchemaInfo;

/// Schema picker component using native HTML select with Thaw styling
#[component]
pub fn SchemaPicker(
    /// Available schemas
    #[prop(into)]
    schemas: Signal<Vec<SchemaInfo>>,
    /// Currently selected schema ID
    #[prop(into)]
    selected: RwSignal<Option<String>>,
    /// Callback when selection changes
    #[prop(optional)]
    on_change: Option<Callback<String>>,
) -> impl IntoView {
    let handle_change = move |ev: web_sys::Event| {
        use wasm_bindgen::JsCast;
        let target = ev.target().unwrap();
        let select = target.dyn_into::<web_sys::HtmlSelectElement>().unwrap();
        let value = select.value();
        
        if value.is_empty() {
            selected.set(None);
        } else {
            selected.set(Some(value.clone()));
            if let Some(cb) = on_change {
                cb.run(value);
            }
        }
    };

    view! {
        <div class="schema-picker">
            <label class="schema-picker-label">"Схема данных"</label>
            <select
                class="schema-picker-select"
                on:change=handle_change
                prop:value=move || selected.get().unwrap_or_default()
            >
                <option value="">"-- Выберите схему --"</option>
                {move || {
                    schemas.get().into_iter().map(|s| {
                        let id = s.id.clone();
                        let name = s.name;
                        view! {
                            <option value=id>{name}</option>
                        }
                    }).collect_view()
                }}
            </select>
        </div>
    }
}
