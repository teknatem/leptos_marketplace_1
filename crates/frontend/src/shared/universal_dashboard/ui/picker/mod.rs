//! Schema picker component for selecting a pivot schema

use contracts::shared::universal_dashboard::SchemaInfo;
use leptos::logging::log;
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
    // Internal value for Thaw Select (String, not Option<String>)
    let select_value = RwSignal::new(selected.get_untracked().unwrap_or_default());

    // One-way sync: parent `selected` → internal `select_value`
    // Also subscribes to `schemas` so that when schemas load,
    // we re-apply the parent value (Thaw Select may have cleared it
    // because no matching <option> existed yet).
    Effect::new(move |_| {
        let _ = schemas.get(); // subscribe to schemas changes
        let parent_val = selected.get().unwrap_or_default();
        if select_value.get_untracked() != parent_val {
            log!("[SchemaPicker] Syncing select_value to parent: {}", parent_val);
            select_value.set(parent_val);
        }
    });

    // Reverse sync: user changes select → update parent `selected`
    // Uses prev-tracking to only fire on actual user changes, not on our sync above.
    Effect::new(move |prev: Option<String>| {
        let val = select_value.get();

        // Skip first run
        if prev.is_none() {
            return val;
        }

        // Only process if value actually changed
        if Some(&val) == prev.as_ref() {
            return val;
        }

        // Ignore resets to empty when schemas aren't loaded
        // (Thaw Select resets value when no matching <option> exists)
        if val.is_empty() && schemas.get_untracked().is_empty() {
            log!("[SchemaPicker] Ignoring empty reset (schemas not loaded)");
            return val;
        }

        // Real user change — propagate to parent
        if val.is_empty() {
            log!("[SchemaPicker] User cleared schema selection");
            selected.set(None);
        } else {
            log!("[SchemaPicker] User selected schema: {}", val);
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
