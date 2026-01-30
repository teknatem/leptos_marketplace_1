use contracts::shared::universal_dashboard::FilterCondition;
use leptos::prelude::*;
use thaw::{Button, ButtonAppearance, ButtonSize};
use wasm_bindgen::JsCast;

/// Component to display a filter condition in a table cell
#[component]
pub fn ConditionDisplay(
    /// Current filter condition (None = no condition set)
    condition: Option<FilterCondition>,
    /// Callback when user clicks to edit
    on_edit: Callback<()>,
    /// Callback when user toggles active state
    on_toggle: Callback<bool>,
) -> impl IntoView {
    match condition {
        Some(cond) => {
            let is_active = cond.active;
            let display_text = cond.display_text.clone();

            // Display existing condition with checkbox at the start
            let btn_class = if is_active {
                "condition-text-btn-thaw condition-text-btn-active"
            } else {
                "condition-text-btn-thaw condition-text-btn-inactive"
            };

            view! {
                <div class="condition-display-with-checkbox">
                    <input
                        type="checkbox"
                        checked=is_active
                        on:change=move |ev| {
                            ev.stop_propagation();
                            let target = ev.target().expect("event target");
                            let input = target.dyn_into::<web_sys::HtmlInputElement>().expect("input element");
                            on_toggle.run(input.checked());
                        }
                        title="Включить/отключить условие"
                        class="condition-checkbox"
                    />
                    <Button
                        appearance=ButtonAppearance::Transparent
                        size=ButtonSize::Small
                        on_click=move |_| on_edit.run(())
                        attr:class=btn_class
                    >
                        {display_text.clone()}
                    </Button>
                </div>
            }
            .into_any()
        }
        None => {
            // No condition - subtle add button
            view! {
                <Button
                    appearance=ButtonAppearance::Subtle
                    size=ButtonSize::Small
                    on_click=move |_| on_edit.run(())
                    attr:class="condition-add-btn-thaw"
                >
                    "+ Добавить"
                </Button>
            }
            .into_any()
        }
    }
}
