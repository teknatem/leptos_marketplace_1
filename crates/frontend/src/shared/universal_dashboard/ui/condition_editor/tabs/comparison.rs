use contracts::shared::universal_dashboard::{ComparisonOp, ConditionDef};
use leptos::prelude::*;
use thaw::*;

/// Tab for comparison conditions (=, !=, <, >, <=, >=)
#[component]
pub fn ComparisonTab(
    /// Current operator
    operator: RwSignal<ComparisonOp>,
    /// Current value
    value: RwSignal<String>,
) -> impl IntoView {
    // Local select value for thaw Select component
    let select_value = RwSignal::new(operator.get_untracked().symbol().to_string());

    // Sync operator -> select_value
    Effect::new(move |_| {
        select_value.set(operator.get().symbol().to_string());
    });

    // Sync select_value -> operator
    Effect::new(move |prev: Option<String>| {
        let current = select_value.get();
        if prev.is_some() && prev.as_ref() != Some(&current) {
            let op = match current.as_str() {
                "=" => ComparisonOp::Eq,
                "≠" => ComparisonOp::NotEq,
                "<" => ComparisonOp::Lt,
                ">" => ComparisonOp::Gt,
                "≤" => ComparisonOp::LtEq,
                "≥" => ComparisonOp::GtEq,
                _ => ComparisonOp::Eq,
            };
            operator.set(op);
        }
        current
    });

    view! {
        <div class="condition-tab comparison-tab">
            <div class="form-group">
                <label>"Оператор:"</label>
                <Select value=select_value size=SelectSize::Small>
                    <option value="=">"= (равно)"</option>
                    <option value="≠">"≠ (не равно)"</option>
                    <option value="<">"< (меньше)"</option>
                    <option value=">">">(больше)"</option>
                    <option value="≤">"≤ (меньше или равно)"</option>
                    <option value="≥">"≥ (больше или равно)"</option>
                </Select>
            </div>

            <div class="form-group">
                <label>"Значение:"</label>
                <Input
                    value=value
                    placeholder="Введите значение"
                />
            </div>
        </div>
    }
}

/// Helper to create ConditionDef from comparison tab state
pub fn build_comparison_condition(operator: ComparisonOp, value: String) -> ConditionDef {
    ConditionDef::Comparison { operator, value }
}
