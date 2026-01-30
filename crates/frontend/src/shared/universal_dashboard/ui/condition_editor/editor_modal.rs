use contracts::shared::universal_dashboard::{
    ComparisonOp, ConditionDef, DatePreset, FieldDefOwned, FilterCondition, ValueType,
};
use leptos::prelude::*;
use thaw::*;

use super::tabs::*;

/// Active tab in the condition editor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditorTab {
    Comparison,
    Range,
    DatePeriod,
    Nullability,
    Contains,
}

/// Modal dialog for editing filter conditions
#[component]
pub fn ConditionEditorModal(
    /// Whether the modal is open
    open: RwSignal<bool>,
    /// Field being edited
    field: Signal<Option<FieldDefOwned>>,
    /// Existing condition (None = creating new)
    existing_condition: Option<FilterCondition>,
    /// Callback when condition is saved
    on_save: Callback<FilterCondition>,
    /// Callback when condition is deleted (optional, only for existing conditions)
    #[prop(optional)]
    on_delete: Option<Callback<()>>,
) -> impl IntoView {
    // Check if we're editing an existing condition (must be done before existing_condition is moved)
    let has_existing_condition = existing_condition.is_some();

    // Active tab
    let active_tab = RwSignal::new(EditorTab::Comparison);
    let selected_tab_value = RwSignal::new("Comparison".to_string());

    // Comparison tab state
    let comparison_operator = RwSignal::new(ComparisonOp::Eq);
    let comparison_value = RwSignal::new(String::new());

    // Range tab state
    let range_from = RwSignal::new(String::new());
    let range_to = RwSignal::new(String::new());

    // Date period tab state
    let date_preset = RwSignal::new(Some(DatePreset::ThisMonth));
    let date_from = RwSignal::new(String::new());
    let date_to = RwSignal::new(String::new());

    // Nullability tab state
    let is_null = RwSignal::new(true);

    // Contains tab state
    let contains_pattern = RwSignal::new(String::new());

    // Load existing condition when modal opens
    Effect::new(move || {
        if open.get() {
            if let Some(cond) = &existing_condition {
                match &cond.definition {
                    ConditionDef::Comparison { operator, value } => {
                        active_tab.set(EditorTab::Comparison);
                        selected_tab_value.set("Comparison".to_string());
                        comparison_operator.set(*operator);
                        comparison_value.set(value.clone());
                    }
                    ConditionDef::Range { from, to } => {
                        active_tab.set(EditorTab::Range);
                        selected_tab_value.set("Range".to_string());
                        range_from.set(from.clone().unwrap_or_default());
                        range_to.set(to.clone().unwrap_or_default());
                    }
                    ConditionDef::DatePeriod { preset, from, to } => {
                        active_tab.set(EditorTab::DatePeriod);
                        selected_tab_value.set("DatePeriod".to_string());
                        date_preset.set(*preset);
                        date_from.set(from.clone().unwrap_or_default());
                        date_to.set(to.clone().unwrap_or_default());
                    }
                    ConditionDef::Nullability { is_null: null_val } => {
                        active_tab.set(EditorTab::Nullability);
                        selected_tab_value.set("Nullability".to_string());
                        is_null.set(*null_val);
                    }
                    ConditionDef::Contains { pattern } => {
                        active_tab.set(EditorTab::Contains);
                        selected_tab_value.set("Contains".to_string());
                        contains_pattern.set(pattern.clone());
                    }
                    ConditionDef::InList { .. } => {
                        // Not implemented in phase 1
                        active_tab.set(EditorTab::Comparison);
                        selected_tab_value.set("Comparison".to_string());
                    }
                }
            } else {
                // Reset to defaults for new condition
                active_tab.set(EditorTab::Comparison);
                selected_tab_value.set("Comparison".to_string());
                comparison_operator.set(ComparisonOp::Eq);
                comparison_value.set(String::new());
                range_from.set(String::new());
                range_to.set(String::new());
                date_preset.set(Some(DatePreset::ThisMonth));
                date_from.set(String::new());
                date_to.set(String::new());
                is_null.set(true);
                contains_pattern.set(String::new());
            }
        }
    });

    // Available tabs based on field type
    let available_tabs = move || {
        if let Some(f) = field.get() {
            let vt = f.get_value_type();
            match &vt {
                ValueType::Integer | ValueType::Numeric => {
                    vec![
                        EditorTab::Comparison,
                        EditorTab::Range,
                        EditorTab::Nullability,
                    ]
                }
                ValueType::Date | ValueType::DateTime => {
                    vec![
                        EditorTab::DatePeriod,
                        EditorTab::Comparison,
                        EditorTab::Nullability,
                    ]
                }
                ValueType::Text => {
                    vec![
                        EditorTab::Comparison,
                        EditorTab::Contains,
                        EditorTab::Nullability,
                    ]
                }
                ValueType::Ref { .. } => {
                    vec![EditorTab::Comparison, EditorTab::Nullability]
                }
                ValueType::Boolean => vec![EditorTab::Comparison, EditorTab::Nullability],
            }
        } else {
            vec![EditorTab::Comparison]
        }
    };

    // Build condition definition from current tab
    let build_condition = move || -> Option<ConditionDef> {
        match active_tab.get() {
            EditorTab::Comparison => {
                let val = comparison_value.get();
                if val.is_empty() {
                    return None;
                }
                Some(build_comparison_condition(comparison_operator.get(), val))
            }
            EditorTab::Range => Some(build_range_condition(range_from.get(), range_to.get())),
            EditorTab::DatePeriod => Some(build_date_period_condition(
                date_preset.get(),
                date_from.get(),
                date_to.get(),
            )),
            EditorTab::Nullability => Some(build_nullability_condition(is_null.get())),
            EditorTab::Contains => {
                let pat = contains_pattern.get();
                if pat.is_empty() {
                    return None;
                }
                Some(build_contains_condition(pat))
            }
        }
    };

    // Handle save
    let handle_save = move |_| {
        if let Some(f) = field.get() {
            if let Some(def) = build_condition() {
                let condition = FilterCondition::new(f.id.clone(), f.get_value_type(), def)
                    .with_field_name(&f.name);

                on_save.run(condition);
                open.set(false);
            }
        }
    };

    // Handle cancel
    let handle_cancel = move |_| {
        open.set(false);
    };

    // Handle delete
    let handle_delete = move |_| {
        if let Some(callback) = on_delete {
            callback.run(());
            open.set(false);
        }
    };

    // Tab label
    let tab_label = move |tab: EditorTab| match tab {
        EditorTab::Comparison => "Сравнение",
        EditorTab::Range => "Диапазон",
        EditorTab::DatePeriod => "Период",
        EditorTab::Nullability => "Пустота",
        EditorTab::Contains => "Содержит",
    };

    // Sync selected_tab_value -> active_tab
    Effect::new(move |prev: Option<String>| {
        let current = selected_tab_value.get();
        if prev.is_some() {
            let tab = match current.as_str() {
                "Comparison" => EditorTab::Comparison,
                "Range" => EditorTab::Range,
                "DatePeriod" => EditorTab::DatePeriod,
                "Nullability" => EditorTab::Nullability,
                "Contains" => EditorTab::Contains,
                _ => EditorTab::Comparison,
            };
            active_tab.set(tab);
        }
        current
    });

    view! {
        <Dialog open=open>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>
                        {move || {
                            if let Some(f) = field.get() {
                                format!("Условие: {}", f.name)
                            } else {
                                "Редактор условия".to_string()
                            }
                        }}
                    </DialogTitle>
                    <DialogContent>
                        <div class="condition-editor-modal">
                            // Tab list
                            <TabList selected_value=selected_tab_value>
                                {move || {
                                    available_tabs()
                                        .into_iter()
                                        .map(|tab| {
                                            view! {
                                                <Tab value=format!("{:?}", tab)>
                                                    {tab_label(tab)}
                                                </Tab>
                                            }
                                        })
                                        .collect_view()
                                }}
                            </TabList>

                            // Tab content
                            <div class="tab-content">
                                {move || match active_tab.get() {
                                    EditorTab::Comparison => {
                                        view! {
                                            <ComparisonTab
                                                operator=comparison_operator
                                                value=comparison_value
                                            />
                                        }
                                            .into_any()
                                    }
                                    EditorTab::Range => {
                                        view! { <RangeTab from_value=range_from to_value=range_to /> }
                                            .into_any()
                                    }
                                    EditorTab::DatePeriod => {
                                        view! {
                                            <DatePeriodTab
                                                preset=date_preset
                                                from_date=date_from
                                                to_date=date_to
                                            />
                                        }
                                            .into_any()
                                    }
                                    EditorTab::Nullability => {
                                        view! { <NullabilityTab is_null=is_null /> }.into_any()
                                    }
                                    EditorTab::Contains => {
                                        view! { <ContainsTab pattern=contains_pattern /> }.into_any()
                                    }
                                }}
                            </div>
                        </div>
                    </DialogContent>
                    <DialogActions>
                        <div style="display: flex; justify-content: space-between; width: 100%;">
                            <div>
                                {move || {
                                    if has_existing_condition && on_delete.is_some() {
                                        view! {
                                            <Button
                                                appearance=ButtonAppearance::Secondary
                                                on_click=handle_delete
                                                attr:class="delete-condition-btn"
                                            >
                                                "Удалить условие"
                                            </Button>
                                        }
                                            .into_any()
                                    } else {
                                        view! { <div></div> }.into_any()
                                    }
                                }}

                            </div>
                            <div style="display: flex; gap: 8px;">
                                <Button appearance=ButtonAppearance::Secondary on_click=handle_cancel>
                                    "Отмена"
                                </Button>
                                <Button appearance=ButtonAppearance::Primary on_click=handle_save>
                                    "Применить"
                                </Button>
                            </div>
                        </div>
                    </DialogActions>
                </DialogBody>
            </DialogSurface>
        </Dialog>
    }
}
