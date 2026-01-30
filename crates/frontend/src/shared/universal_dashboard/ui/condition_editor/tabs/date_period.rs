use contracts::shared::universal_dashboard::{ConditionDef, DatePreset};
use leptos::prelude::*;
use thaw::*;

/// Tab for date period conditions with presets
#[component]
pub fn DatePeriodTab(
    /// Selected preset
    preset: RwSignal<Option<DatePreset>>,
    /// Custom from date
    from_date: RwSignal<String>,
    /// Custom to date
    to_date: RwSignal<String>,
) -> impl IntoView {
    let use_preset = RwSignal::new(preset.get_untracked().is_some());
    
    // Radio group value (for thaw RadioGroup API)
    let radio_value = RwSignal::new(if use_preset.get_untracked() {
        "preset".to_string()
    } else {
        "custom".to_string()
    });

    // Sync radio_value changes to use_preset
    Effect::new(move |prev: Option<String>| {
        let current = radio_value.get();
        if prev.is_some() {
            match current.as_str() {
                "preset" => {
                    use_preset.set(true);
                    if preset.get_untracked().is_none() {
                        preset.set(Some(DatePreset::ThisMonth));
                    }
                }
                "custom" => {
                    use_preset.set(false);
                    preset.set(None);
                }
                _ => {}
            }
        }
        current
    });

    view! {
        <div class="condition-tab date-period-tab">
            <div class="form-group">
                <RadioGroup value=radio_value>
                    <Radio value="preset" label="Быстрый выбор"/>
                    <Radio value="custom" label="Свой период"/>
                </RadioGroup>
            </div>

            {move || {
                if use_preset.get() {
                    view! {
                        <div class="preset-buttons">
                            <For
                                each=|| DatePreset::all().to_vec()
                                key=|p| format!("{:?}", p)
                                children=move |p: DatePreset| {
                                    let is_selected = move || preset.get() == Some(p);
                                    view! {
                                        <Button
                                            appearance=move || {
                                                if is_selected() {
                                                    ButtonAppearance::Primary
                                                } else {
                                                    ButtonAppearance::Secondary
                                                }
                                            }
                                            size=ButtonSize::Small
                                            on_click=move |_| preset.set(Some(p))
                                        >
                                            {p.display_name()}
                                        </Button>
                                    }
                                }
                            />
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div class="custom-date-inputs">
                            <div class="form-group">
                                <label>"С:"</label>
                                <Input
                                    value=from_date
                                    placeholder="ГГГГ-ММ-ДД"
                                />
                            </div>
                            <div class="form-group">
                                <label>"По:"</label>
                                <Input
                                    value=to_date
                                    placeholder="ГГГГ-ММ-ДД"
                                />
                            </div>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}

/// Helper to create ConditionDef from date period tab state
pub fn build_date_period_condition(
    preset: Option<DatePreset>,
    from: String,
    to: String,
) -> ConditionDef {
    let from_opt = if from.is_empty() { None } else { Some(from) };
    let to_opt = if to.is_empty() { None } else { Some(to) };
    ConditionDef::DatePeriod {
        preset,
        from: from_opt,
        to: to_opt,
    }
}
