use contracts::shared::universal_dashboard::ConditionDef;
use leptos::prelude::*;
use thaw::*;

/// Tab for nullability conditions (IS NULL / IS NOT NULL)
#[component]
pub fn NullabilityTab(
    /// Is null flag
    is_null: RwSignal<bool>,
) -> impl IntoView {
    // Radio group value (for thaw RadioGroup API)
    let radio_value = RwSignal::new(if is_null.get_untracked() {
        "null".to_string()
    } else {
        "not_null".to_string()
    });

    // Sync radio_value changes to is_null
    Effect::new(move |prev: Option<String>| {
        let current = radio_value.get();
        if prev.is_some() {
            is_null.set(current == "null");
        }
        current
    });

    view! {
        <div class="condition-tab nullability-tab">
            <div class="form-group">
                <RadioGroup value=radio_value>
                    <Radio value="null" label="Поле не заполнено (IS NULL)"/>
                    <Radio value="not_null" label="Поле заполнено (IS NOT NULL)"/>
                </RadioGroup>
            </div>

            <div class="help-text">
                "Проверка на отсутствие или наличие значения"
            </div>
        </div>
    }
}

/// Helper to create ConditionDef from nullability tab state
pub fn build_nullability_condition(is_null: bool) -> ConditionDef {
    ConditionDef::Nullability { is_null }
}
