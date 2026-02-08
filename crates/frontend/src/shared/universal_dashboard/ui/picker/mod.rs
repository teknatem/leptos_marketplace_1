//! Schema picker component for selecting a pivot schema

use contracts::shared::universal_dashboard::SchemaInfo;
use leptos::prelude::*;
use thaw::Select;

/// Schema picker component using Thaw Select
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
    // Convert Option<String> to String for Select (empty string = not selected)
    let select_value = RwSignal::new(selected.get_untracked().unwrap_or_default());

    // Update select_value when selected changes (from outside)
    Effect::new(move |_| {
        let new_value = selected.get().unwrap_or_default();
        select_value.set(new_value);
    });

    // Update selected when user changes select_value
    Effect::new(move |prev: Option<String>| {
        let val = select_value.get();

        // Skip first run (initialization)
        if prev.is_none() {
            return val.clone();
        }

        // Only process if value actually changed
        if Some(&val) == prev.as_ref() {
            return val.clone();
        }

        if val.is_empty() {
            selected.set(None);
        } else {
            selected.set(Some(val.clone()));
            if let Some(cb) = on_change {
                cb.run(val.clone());
            }
        }
        val
    });

    view! {
        <div class="schema-picker">
            <label class="schema-picker-label">""</label>
            <Select value=select_value>
                <option value="">"-- Выберите схему --"</option>
                <For
                    each=move || schemas.get()
                    key=|s| s.id.clone()
                    children=move |s: SchemaInfo| {
                        let id = s.id.clone();
                        let name = s.name.clone();
                        view! {
                            <option value=id>{name}</option>
                        }
                    }
                />
            </Select>
        </div>
    }
}
