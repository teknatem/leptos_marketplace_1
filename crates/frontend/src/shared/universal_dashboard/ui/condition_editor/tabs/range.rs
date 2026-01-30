use contracts::shared::universal_dashboard::ConditionDef;
use leptos::prelude::*;
use thaw::*;

/// Tab for range conditions (BETWEEN)
#[component]
pub fn RangeTab(
    /// From value
    from_value: RwSignal<String>,
    /// To value
    to_value: RwSignal<String>,
) -> impl IntoView {
    view! {
        <div class="condition-tab range-tab">
            <div class="form-group">
                <label>"От:"</label>
                <Input
                    value=from_value
                    placeholder="Минимальное значение"
                />
            </div>

            <div class="form-group">
                <label>"До:"</label>
                <Input
                    value=to_value
                    placeholder="Максимальное значение"
                />
            </div>

            <div class="help-text">
                "Диапазон включает оба значения (от ≤ поле ≤ до)"
            </div>
        </div>
    }
}

/// Helper to create ConditionDef from range tab state
pub fn build_range_condition(from: String, to: String) -> ConditionDef {
    let from_opt = if from.is_empty() { None } else { Some(from) };
    let to_opt = if to.is_empty() { None } else { Some(to) };
    ConditionDef::Range {
        from: from_opt,
        to: to_opt,
    }
}
