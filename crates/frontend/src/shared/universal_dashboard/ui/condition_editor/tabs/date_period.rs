use contracts::shared::universal_dashboard::{ConditionDef, DatePreset};
use leptos::prelude::*;
use thaw::*;

/// Calculate the last day of the month for a given date string (YYYY-MM-DD)
fn calculate_end_of_month(date_str: &str) -> Option<String> {
    // Parse date string (format: YYYY-MM-DD)
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return None;
    }

    let year: i32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;

    if month < 1 || month > 12 {
        return None;
    }

    // Calculate last day of month
    let last_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            // Leap year check
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => return None,
    };

    Some(format!("{:04}-{:02}-{:02}", year, month, last_day))
}

/// Single preset button component
#[component]
fn PresetButton(
    /// The preset value
    preset_value: DatePreset,
    /// Currently selected preset
    #[prop(into)]
    selected_preset: Signal<Option<DatePreset>>,
    /// Callback when clicked
    on_select: Callback<DatePreset>,
) -> impl IntoView {
    let appearance = Memo::new(move |_| {
        if selected_preset.get() == Some(preset_value) {
            ButtonAppearance::Primary
        } else {
            ButtonAppearance::Secondary
        }
    });

    view! {
        <Button
            appearance=appearance
            size=ButtonSize::Small
            on_click=move |_| on_select.run(preset_value)
        >
            {preset_value.display_name()}
        </Button>
    }
}

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

    let on_preset_select = Callback::new(move |p: DatePreset| {
        preset.set(Some(p));
    });

    // Auto-fill end date with end of month when start date is selected
    Effect::new(move |prev: Option<String>| {
        let current_from = from_date.get();

        // Skip first run
        if prev.is_none() {
            return current_from.clone();
        }

        // Only process if from_date changed and has value
        if Some(&current_from) != prev.as_ref() && !current_from.is_empty() {
            // Only auto-fill if to_date is empty
            if to_date.get_untracked().is_empty() {
                if let Some(end_of_month) = calculate_end_of_month(&current_from) {
                    to_date.set(end_of_month);
                }
            }
        }

        current_from
    });

    // Create preset buttons statically to avoid disposal issues
    let preset_buttons_view = DatePreset::all()
        .iter()
        .map(|&p| {
            view! {
                <PresetButton
                    preset_value=p
                    selected_preset=preset.read_only()
                    on_select=on_preset_select
                />
            }
        })
        .collect_view();

    view! {
        <div class="condition-tab date-period-tab">
            <div class="form-group">
                <RadioGroup value=radio_value>
                    <Radio value="preset" label="Быстрый выбор"/>
                    <Radio value="custom" label="Свой период"/>
                </RadioGroup>
            </div>

            <div class="preset-buttons" style:display=move || {
                if use_preset.get() { "flex" } else { "none" }
            }>
                {preset_buttons_view}
            </div>

            <div class="custom-date-inputs" style:display=move || {
                if use_preset.get() { "none" } else { "block" }
            }>
                <div class="form-group">
                    <label>"С:"</label>
                    <input
                        type="date"
                        prop:value=move || from_date.get()
                        on:input=move |ev| {
                            from_date.set(event_target_value(&ev));
                        }
                        style="width: 100%; padding: 6px; border: 1px solid var(--thaw-color-neutral-stroke-1); border-radius: 4px;"
                    />
                </div>
                <div class="form-group">
                    <label>"По:"</label>
                    <input
                        type="date"
                        prop:value=move || to_date.get()
                        on:input=move |ev| {
                            to_date.set(event_target_value(&ev));
                        }
                        style="width: 100%; padding: 6px; border: 1px solid var(--thaw-color-neutral-stroke-1); border-radius: 4px;"
                    />
                </div>
            </div>
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
