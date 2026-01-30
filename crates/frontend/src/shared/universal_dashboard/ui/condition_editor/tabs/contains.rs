use contracts::shared::universal_dashboard::ConditionDef;
use leptos::prelude::*;
use thaw::*;

/// Tab for text contains conditions (LIKE)
#[component]
pub fn ContainsTab(
    /// Pattern to search for
    pattern: RwSignal<String>,
) -> impl IntoView {
    view! {
        <div class="condition-tab contains-tab">
            <div class="form-group">
                <label>"Текст для поиска:"</label>
                <Input
                    value=pattern
                    placeholder="Введите текст"
                />
            </div>

            <div class="help-text">
                "Поиск записей, содержащих указанный текст (регистр не учитывается)"
            </div>
        </div>
    }
}

/// Helper to create ConditionDef from contains tab state
pub fn build_contains_condition(pattern: String) -> ConditionDef {
    ConditionDef::Contains { pattern }
}
